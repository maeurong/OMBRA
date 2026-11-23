//! Fused timestep pipeline — single FFI entrance/exit per timestep.
//!
//! Orchestrates: shadows → ground_temp → GVF → thermal_delay → radiation → Tmrt
//! All intermediate arrays stay as ndarray::Array2<f32> — never cross FFI boundary.
//!
//! Supports both isotropic and anisotropic (Perez) sky models.

use ndarray::{s, Array1, Array2, Array3, ArrayView1, ArrayView2, Zip};
#[cfg(feature = "gpu")]
use std::sync::OnceLock;

use crate::ground::{compute_ground_temperature_pure, ts_wave_delay_batch_pure, GroundTempResult};
use crate::ground_surface::{outgoing_longwave_calc_pure, surface_temperature_calc_pure};
use crate::gvf::{gvf_calc_pure, gvf_calc_with_cache, GvfResultPure};
#[cfg(feature = "gpu")]
use crate::gvf::gvf_calc_with_cache_gpu;
use crate::gvf_geometry::{precompute_gvf_geometry_cpu, GvfGeometryCache};
#[cfg(feature = "gpu")]
use crate::gvf_geometry::precompute_gvf_geometry_gpu;
use crate::shadowing::{calculate_shadows_rust, ShadowingResultRust};
use crate::sky::{anisotropic_sky_pure, cylindric_wedge_pure_masked};
use crate::tmrt::compute_tmrt_from_dir_sums_pure;
use crate::vegetation::{kside_veg_isotropic_pure, lside_veg_pure, lside_veg_variant_pure, LsideVariant};

#[cfg(feature = "gpu")]
use crate::gpu::AnisoGpuContext;
#[cfg(feature = "gpu")]
use crate::gpu::GvfGpuContext;

use std::time::Instant;

const PI: f32 = std::f32::consts::PI;
const SBC: f32 = 5.67051e-8;
const KELVIN_OFFSET: f32 = 273.15;

/// Check once per process whether timing output is enabled (``SOLWEIG_TIMING=1``).
fn timing_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("SOLWEIG_TIMING")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

// ── GPU anisotropic sky context (lazy-initialized, shares device with shadows) ──

#[cfg(feature = "gpu")]
static ANISO_GPU_CONTEXT: OnceLock<Option<AnisoGpuContext>> = OnceLock::new();

#[cfg(feature = "gpu")]
static ANISO_GPU_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[cfg(feature = "gpu")]
fn get_aniso_gpu_context() -> Option<&'static AnisoGpuContext> {
    if !ANISO_GPU_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    ANISO_GPU_CONTEXT
        .get_or_init(|| {
            // Share device/queue from the shadow GPU context
            let shadow_ctx = crate::shadowing::get_gpu_context()?;
            let device = shadow_ctx.device.clone();
            let queue = shadow_ctx.queue.clone();
            let ctx = AnisoGpuContext::new(device, queue);
            eprintln!("[GPU] Anisotropic sky GPU context initialized");
            Some(ctx)
        })
        .as_ref()
}

// ── GPU GVF context (lazy-initialized, shares device with shadows) ──────

#[cfg(feature = "gpu")]
static GVF_GPU_CONTEXT: OnceLock<Option<GvfGpuContext>> = OnceLock::new();

#[cfg(feature = "gpu")]
static GVF_GPU_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

#[cfg(feature = "gpu")]
fn get_gvf_gpu_context() -> Option<&'static GvfGpuContext> {
    if !GVF_GPU_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    GVF_GPU_CONTEXT
        .get_or_init(|| {
            let shadow_ctx = crate::shadowing::get_gpu_context()?;
            let device = shadow_ctx.device.clone();
            let queue = shadow_ctx.queue.clone();
            let ctx = GvfGpuContext::new(device, queue);
            eprintln!("[GPU] GVF GPU context initialized");
            Some(ctx)
        })
        .as_ref()
}

// ── GPU GVF geometry precompute ─────────────────────────────────────────
//
// The precompute now runs on the shared `GvfGpuContext` (see above): it writes
// blocking_distance/facesh straight into the resident geometry buffers the
// per-timestep dispatch reads, so there is no readback+re-upload round trip.
// One CPU mirror of the geometry is still produced (fallback + Python
// inspection). This flag gates whether the geometry march runs on the GPU;
// toggle via pipeline.enable/disable_gvf_precompute_gpu().
#[cfg(feature = "gpu")]
static GVF_PRECOMPUTE_GPU_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

// Scalar input structs (WeatherScalars / HumanScalars / ConfigScalars)
// extracted to pipeline_scalars.rs. Re-exported here so the lib.rs
// registration path `pipeline::WeatherScalars` keeps working.
pub use crate::pipeline_scalars::{ConfigScalars, HumanScalars, WeatherScalars};

/// Result of [`compute_timestep_rust`]: owned arrays, no Python types
/// (Send-safe).
///
/// `shadow`, `kdown`, `kup`, `ldown` and `lup` are `Some` when requested in
/// `TimestepInputs::output_mask`. `timeadd` and the `tg*` arrays are the
/// thermal state to pass into the next timestep. `tg`, `rn`, `rn_past` and
/// `g` are `Some` only when the 2026a ground scheme ran.
pub struct TimestepResultRaw {
    pub tmrt: Array2<f32>,
    pub shadow: Option<Array2<f32>>,
    pub kdown: Option<Array2<f32>>,
    pub kup: Option<Array2<f32>>,
    pub ldown: Option<Array2<f32>>,
    pub lup: Option<Array2<f32>>,
    pub timeadd: f32,
    pub tgmap1: Array2<f32>,
    pub tgmap1_e: Array2<f32>,
    pub tgmap1_s: Array2<f32>,
    pub tgmap1_w: Array2<f32>,
    pub tgmap1_n: Array2<f32>,
    pub tgout1: Array2<f32>,
    pub tg: Option<Array2<f32>>,
    pub rn: Option<Array2<f32>>,
    pub rn_past: Option<Array2<f32>>,
    pub g: Option<Array2<f32>>,
}

// Output selection flags for `TimestepInputs::output_mask`.
pub const OUT_SHADOW: u32 = 1 << 0;
pub const OUT_KDOWN: u32 = 1 << 1;
pub const OUT_KUP: u32 = 1 << 2;
pub const OUT_LDOWN: u32 = 1 << 3;
pub const OUT_LUP: u32 = 1 << 4;
pub const OUT_ALL: u32 = OUT_SHADOW | OUT_KDOWN | OUT_KUP | OUT_LDOWN | OUT_LUP;

use crate::pipeline_inputs::{
    GroundSchemeViews, PropertyViews, SurfaceViews, SvfViews, ThermalStateViews, TimestepInputs,
};

// Radiation helpers extracted to pipeline_radiation.rs. Re-import only
// the symbols used inside compute_timestep below.
#[cfg(feature = "gpu")]
use crate::pipeline_radiation::kside_dirs_sum_aniso_from_kup;
use crate::pipeline_radiation::{
    asvf_for_svf_cached, compute_ani_lum_from_packed, compute_esky, compute_kdown, compute_kup,
    compute_ldown, lside_dirs_sum_aniso_from_lup, patch_lut_for_option_cached,
    side_sum_from_directional,
};

// ── Rust-callable timestep ─────────────────────────────────────────────────

/// Check what [`compute_timestep_rust`] cannot safely index: wall inputs when
/// walls are enabled, and the shape of the anisotropic shadow matrices.
fn validate_timestep_inputs(inputs: &TimestepInputs) -> Result<(), String> {
    let config = inputs.config;
    if config.has_walls && (inputs.surface.wall_ht.is_none() || inputs.surface.wall_asp.is_none()) {
        return Err("config.has_walls=true requires both wall_ht and wall_asp inputs".to_string());
    }

    // Validate anisotropic shadow-matrix dimensions match the DSM grid before
    // indexing them. Without this check a mis-sized array silently reads OOB
    // garbage (or aborts via wgpu) inside the hot loop.
    let dsm_shape = inputs.surface.dsm.dim();
    let n_patches = patch_lut_for_option_cached(inputs.weather.patch_option)
        .altitudes
        .len();
    let expected_pack = n_patches.div_ceil(8);
    for (name, opt) in [
        ("shmat", inputs.shmat.as_ref()),
        ("vegshmat", inputs.vegshmat.as_ref()),
        ("vbshmat", inputs.vbshmat.as_ref()),
    ] {
        if let Some(arr) = opt {
            let (rows, cols, depth) = arr.dim();
            if (rows, cols) != dsm_shape {
                return Err(format!(
                    "{name} dim ({}, {}, _) does not match DSM dim {:?}",
                    rows, cols, dsm_shape
                ));
            }
            // A stale precomputed shadow-matrix file generated with a different
            // patch option would otherwise index out of range deep in the
            // radiation loop and abort the process (panic = "abort").
            if depth != expected_pack {
                return Err(format!(
                    "{name} packed patch depth {depth} does not match patch_option {} \
                     ({n_patches} patches -> {expected_pack} packed bytes); the precomputed \
                     shadow matrices were generated with a different patch option — \
                     recompute the SVF/shadow cache (force_recompute=True)",
                    inputs.weather.patch_option
                ));
            }
        }
    }
    Ok(())
}

/// Compute a single timestep from plain `ndarray` views: shadows, ground
/// temperature, ground view factors, radiation and Tmrt.
///
/// This is the whole per-timestep model. The Python binding
/// [`compute_timestep`] only borrows the numpy arrays, calls this, and wraps
/// the result. It does not touch Python and can run without the GIL.
///
/// Returns `Err` when wall inputs are missing with `has_walls` set, or when a
/// shadow matrix does not match the DSM grid or the patch option. Other
/// rasters are assumed to share the DSM shape.
pub fn compute_timestep_rust(inputs: &TimestepInputs) -> Result<TimestepResultRaw, String> {
    validate_timestep_inputs(inputs)?;
    Ok(timestep_physics(inputs))
}

fn timestep_physics(inputs: &TimestepInputs) -> TimestepResultRaw {
    let weather = inputs.weather;
    let human = inputs.human;
    let config = inputs.config;
    let gvf_cache_inner = inputs.gvf_cache;
    let valid_v = inputs.valid_mask;
    let dsm_v = inputs.surface.dsm;
    let cdsm_v = inputs.surface.cdsm;
    let tdsm_v = inputs.surface.tdsm;
    let bush_v = inputs.surface.bush;
    let wall_ht_v = inputs.surface.wall_ht;
    let wall_asp_v = inputs.surface.wall_asp;
    let svf_v = inputs.svf.svf;
    let svf_n_v = inputs.svf.svf_n;
    let svf_e_v = inputs.svf.svf_e;
    let svf_s_v = inputs.svf.svf_s;
    let svf_w_v = inputs.svf.svf_w;
    let svf_veg_v = inputs.svf.svf_veg;
    let svf_veg_n_v = inputs.svf.svf_veg_n;
    let svf_veg_e_v = inputs.svf.svf_veg_e;
    let svf_veg_s_v = inputs.svf.svf_veg_s;
    let svf_veg_w_v = inputs.svf.svf_veg_w;
    let svf_aveg_v = inputs.svf.svf_aveg;
    let svf_aveg_n_v = inputs.svf.svf_aveg_n;
    let svf_aveg_e_v = inputs.svf.svf_aveg_e;
    let svf_aveg_s_v = inputs.svf.svf_aveg_s;
    let svf_aveg_w_v = inputs.svf.svf_aveg_w;
    let svfbuveg_v = inputs.svf.svfbuveg;
    let svfalfa_v = inputs.svf.svfalfa;
    let alb_grid_v = inputs.properties.alb_grid;
    let emis_grid_v = inputs.properties.emis_grid;
    let tgk_grid_v = inputs.properties.tgk_grid;
    let tstart_grid_v = inputs.properties.tstart_grid;
    let tmaxlst_grid_v = inputs.properties.tmaxlst_grid;
    let buildings_v = inputs.buildings;
    let lc_grid_v = inputs.lc_grid;
    let tgmap1_v = inputs.state.tgmap1;
    let tgmap1_e_v = inputs.state.tgmap1_e;
    let tgmap1_s_v = inputs.state.tgmap1_s;
    let tgmap1_w_v = inputs.state.tgmap1_w;
    let tgmap1_n_v = inputs.state.tgmap1_n;
    let tgout1_v = inputs.state.tgout1;
    let firstdaytime = inputs.state.firstdaytime;
    let timeadd = inputs.state.timeadd;
    let timestep_dec = inputs.state.timestep_dec;
    let sch = inputs.ground_scheme.as_ref();
    let sch_timestep_s = sch.map(|s| s.timestep_s);
    let sch_tg_v = sch.map(|s| s.tg);
    let sch_tm_v = sch.map(|s| s.tm);
    let sch_rn_v = sch.map(|s| s.rn);
    let sch_rn_past_v = sch.map(|s| s.rn_past);
    let sch_g_v = sch.map(|s| s.g);
    let sch_cap_v = sch.map(|s| s.cap);
    let sch_diff_v = sch.map(|s| s.diff);
    let sch_a1_v = sch.map(|s| s.a1);
    let sch_a2_v = sch.map(|s| s.a2);
    let sch_a3_v = sch.map(|s| s.a3);
    let sch_lc_v = sch.map(|s| s.lc_grid);
    let sch_shadow_past_v = sch.map(|s| s.shadow_past);
    let use_scheme = sch.is_some();
    let shmat_v = inputs.shmat;
    let vegshmat_v = inputs.vegshmat;
    let vbshmat_v = inputs.vbshmat;
    let want_shadow = (inputs.output_mask & OUT_SHADOW) != 0;
    let want_kdown = (inputs.output_mask & OUT_KDOWN) != 0;
    let want_kup = (inputs.output_mask & OUT_KUP) != 0;
    let want_ldown = (inputs.output_mask & OUT_LDOWN) != 0;
    let want_lup = (inputs.output_mask & OUT_LUP) != 0;

    let shape = dsm_v.dim();

    // Wall aspect in radians for shadows
    let wall_asp_rad: Option<Array2<f32>> = wall_asp_v.map(|a| a.mapv(|d| d * PI / 180.0));
    let wall_asp_rad_view = wall_asp_rad.as_ref().map(|a| a.view());

    // ── Step 1: Shadows ──────────────────────────────────────────────────
    let t_shadow = Instant::now();
    let shadow_result: ShadowingResultRust = calculate_shadows_rust(
        weather.sun_azimuth,
        weather.sun_altitude,
        config.pixel_size,
        config.max_height,
        dsm_v,
        if config.use_veg { cdsm_v } else { None },
        if config.use_veg { tdsm_v } else { None },
        if config.use_veg { bush_v } else { None },
        if config.has_walls { wall_ht_v } else { None },
        if config.has_walls {
            wall_asp_rad_view
        } else {
            None
        },
        None,  // walls_scheme
        None,  // aspect_scheme
        false, // need_full_wall_outputs (pipeline only needs wall_sun)
        3.0,   // min_sun_altitude
        config.max_shadow_distance_m,
    );

    // Combine shadows with vegetation transmissivity
    let bldg_sh = &shadow_result.bldg_sh;
    let shadow = if config.use_veg {
        let veg_sh = &shadow_result.veg_sh;
        bldg_sh - &((1.0 - veg_sh) * (1.0 - weather.psi))
    } else {
        bldg_sh.clone()
    };
    let shadow_f32 = shadow;

    let wallsun = shadow_result
        .wall_sun
        .unwrap_or_else(|| Array2::zeros(shape));

    let shadow_dur = t_shadow.elapsed();

    // ── Step 2: Ground Temperature ───────────────────────────────────────
    let t_ground = Instant::now();
    let ground: GroundTempResult = compute_ground_temperature_pure(
        weather.sun_altitude,
        weather.altmax,
        weather.dectime,
        weather.snup,
        weather.global_rad,
        weather.rad_g0,
        weather.zen_deg,
        tgk_grid_v,
        tstart_grid_v,
        tmaxlst_grid_v,
        config.tgk_wall,
        config.tstart_wall,
        config.tmaxlst_wall,
    );

    let ground_dur = t_ground.elapsed();

    // At night (sun below horizon), UMEP Python zeros Tgwall and Tg.
    // The sinusoidal model keeps producing non-zero values after sunrise
    // (since dectime > snup_frac even after sunset), but UMEP's runner
    // explicitly overrides them to zero when altitude <= 0.
    let ground = if !weather.is_daytime {
        GroundTempResult {
            tg: Array2::<f32>::zeros(ground.tg.dim()),
            tg_wall: 0.0,
            ci_tg: ground.ci_tg,
        }
    } else {
        ground
    };

    // ── UMEP 2026a ground-surface scheme (opt-in) ────────────────────────
    // When active, the force-restore/OHM surface temperature replaces the
    // classic sinusoidal Tg, and the solid-angle outgoing-longwave march
    // supplies Lup / gvfalb* / directional side longwave (replacing the
    // GVF + thermal-delay steps). The classic wall temperature
    // (ground.tg_wall) is retained. Ordering follows Solweig_2026a_calc.
    if use_scheme {
        let timestep_s = sch_timestep_s.expect("scheme: timestep_s");
        let tg_state = sch_tg_v.expect("scheme: tg");
        let tm_state = sch_tm_v.expect("scheme: tm");
        let rn_state = sch_rn_v.expect("scheme: rn");
        let rn_past_state = sch_rn_past_v.expect("scheme: rn_past");
        let g_state = sch_g_v.expect("scheme: g");
        let cap_v = sch_cap_v.expect("scheme: cap");
        let diff_v = sch_diff_v.expect("scheme: diff");
        let a1_v = sch_a1_v.expect("scheme: a1");
        let a2_v = sch_a2_v.expect("scheme: a2");
        let a3_v = sch_a3_v.expect("scheme: a3");
        let lc_scheme_v = sch_lc_v.expect("scheme: lc_grid");
        let shadow_past_v = sch_shadow_past_v.expect("scheme: shadow_past");

        // Upstream zeros the shadow grid at night (altitude <= 0); both
        // the surface-temperature damping mask and the march consume it.
        let scheme_shadow = if weather.is_daytime {
            shadow_f32.clone()
        } else {
            Array2::<f32>::zeros(shape)
        };

        let esky = compute_esky(weather.ta, weather.rh);
        let zen_rad = weather.sun_zenith * PI / 180.0;
        let f_sh = cylindric_wedge_pure_masked(zen_rad, svfalfa_v, Some(valid_v));
        let sin_alt = (weather.sun_altitude * PI / 180.0).sin();
        let rad_i = weather.direct_rad;
        let rad_d = weather.diffuse_rad;
        let rad_g = weather.global_rad;
        let psi = weather.psi;
        let cyl = human.is_standing;

        let use_aniso = config.use_anisotropic
            && shmat_v.is_some()
            && vegshmat_v.is_some()
            && vbshmat_v.is_some();

        // Anisotropic setup (Perez luminance + CI-corrected esky). The
        // scheme uses the CPU sky solver only; GPU aniso for the scheme
        // is future work and produces identical results.
        let (lv_arr, esky_a) = if use_aniso {
            let lv = crate::perez::perez_v3(
                weather.zen_deg,
                weather.sun_azimuth,
                weather.diffuse_rad,
                weather.direct_rad,
                weather.jday,
                weather.patch_option,
            );
            let ci = weather.clearness_index;
            let ea = if ci < 0.95 { ci * esky + (1.0 - ci) } else { esky };
            (Some(lv), ea)
        } else {
            (None, esky)
        };

        // Diffuse shortwave into each cell (drad)
        let drad = if use_aniso {
            let lv = lv_arr.as_ref().expect("lv gated by use_aniso");
            let shmat_a = shmat_v.expect("shmat: gated by use_aniso");
            let vegshmat_a = vegshmat_v.expect("vegshmat: gated by use_aniso");
            let ani_lum = compute_ani_lum_from_packed(
                shmat_a,
                vegshmat_a,
                lv.column(2),
                psi,
                valid_v,
            );
            ani_lum.mapv(|v| v * rad_d)
        } else {
            svfbuveg_v.mapv(|sv| rad_d * sv)
        };

        // Kdown (identical formula to the classic path)
        let kdown = compute_kdown(
            rad_i,
            rad_d,
            rad_g,
            scheme_shadow.view(),
            sin_alt,
            svfbuveg_v,
            config.albedo_wall,
            f_sh.view(),
            drad.view(),
            valid_v,
        );

        // Isotropic Ldown — the value fed to the surface-temperature ODE,
        // the march, and Lside_veg_v2026 (the anisotropic sky reassigns
        // the Tmrt Ldown below, matching upstream).
        let ldown_iso = compute_ldown(
            esky,
            weather.ta,
            ground.tg_wall,
            svf_v,
            svf_veg_v,
            svf_aveg_v,
            config.emis_wall,
            weather.clearness_index,
            valid_v,
        );

        // Force-restore/OHM surface temperature step → new Tg + fluxes.
        let st = surface_temperature_calc_pure(
            kdown.view(),
            ldown_iso.view(),
            rn_state,
            rn_past_state,
            g_state,
            tg_state,
            tm_state,
            alb_grid_v,
            emis_grid_v,
            cap_v,
            diff_v,
            lc_scheme_v,
            a1_v,
            a2_v,
            a3_v,
            timestep_s,
            weather.rh,
            scheme_shadow.view(),
            shadow_past_v,
        );

        // Outgoing-longwave solid-angle march. Walls default to zeros
        // when the scene has none (no wall longwave contribution).
        let walls_owned: Array2<f32> = if config.has_walls {
            wall_ht_v
                .expect("wall_ht: gated by config.has_walls=true")
                .to_owned()
        } else {
            Array2::<f32>::zeros(shape)
        };
        let march = outgoing_longwave_calc_pure(
            st.tg.view(),
            ground.tg_wall,
            weather.ta,
            ldown_iso.view(),
            emis_grid_v,
            alb_grid_v,
            buildings_v,
            scheme_shadow.view(),
            wallsun.view(),
            walls_owned.view(),
            config.pixel_size,
        );

        // Kup from the march's albedo view factors (gvfalbsun→gvfalb,
        // gvfalbtot→gvfalbnosh).
        let (kup, kup_e, kup_s, kup_w, kup_n) = compute_kup(
            rad_i,
            rad_d,
            rad_g,
            weather.sun_altitude,
            svfbuveg_v,
            config.albedo_wall,
            f_sh.view(),
            march.gvfalbsun.view(),
            march.gvfalbsun_e.view(),
            march.gvfalbsun_s.view(),
            march.gvfalbsun_w.view(),
            march.gvfalbsun_n.view(),
            march.gvfalbtot.view(),
            march.gvfalbtot_e.view(),
            march.gvfalbtot_s.view(),
            march.gvfalbtot_w.view(),
            march.gvfalbtot_n.view(),
            valid_v,
        );

        // Lside_veg_v2026: reflection loses the Lup term; the ground
        // longwave leaves Lside entirely (supplied by the march's
        // gvfLside*). The anisotropic branch returns zeros. The Lup*
        // slots are unused by V2026 — Ldown stands in.
        let lside = lside_veg_variant_pure(
            LsideVariant::V2026,
            svf_s_v,
            svf_w_v,
            svf_n_v,
            svf_e_v,
            svf_veg_e_v,
            svf_veg_s_v,
            svf_veg_w_v,
            svf_veg_n_v,
            svf_aveg_e_v,
            svf_aveg_s_v,
            svf_aveg_w_v,
            svf_aveg_n_v,
            weather.sun_azimuth,
            weather.sun_altitude,
            weather.ta,
            ground.tg_wall,
            SBC,
            config.emis_wall,
            ldown_iso.view(),
            esky,
            0.0,
            f_sh.view(),
            weather.clearness_index,
            ldown_iso.view(),
            ldown_iso.view(),
            ldown_iso.view(),
            ldown_iso.view(),
            use_aniso,
            Some(valid_v),
        );

        let (kdown_out, ldown_out, kside_dirs_sum, lside_dirs_sum, kside_total, lside_total) =
            if use_aniso {
                let shmat_a = shmat_v.expect("shmat: gated by use_aniso");
                let vegshmat_a = vegshmat_v.expect("vegshmat: gated by use_aniso");
                let vbshmat_a = vbshmat_v.expect("vbshmat: gated by use_aniso");
                let lv = lv_arr.as_ref().expect("lv gated by use_aniso");
                let patch_lut = patch_lut_for_option_cached(weather.patch_option);
                let steradians_arr = ArrayView1::from(patch_lut.steradians.as_slice());
                let asvf_cache = asvf_for_svf_cached(svf_v);
                let asvf_arr = ArrayView2::from_shape(shape, asvf_cache.as_slice())
                    .expect("ASVF cache shape matches DSM");

                let ani = anisotropic_sky_pure(
                    shmat_a,
                    vegshmat_a,
                    vbshmat_a,
                    weather.sun_altitude,
                    weather.sun_azimuth,
                    esky_a,
                    weather.ta,
                    cyl,
                    false,
                    config.albedo_wall,
                    ground.tg_wall,
                    config.emis_wall,
                    rad_i,
                    rad_d,
                    asvf_arr,
                    lv.view(),
                    steradians_arr,
                    march.gvf_lup.view(),
                    lv.view(),
                    scheme_shadow.view(),
                    kup_e.view(),
                    kup_s.view(),
                    kup_w.view(),
                    kup_n.view(),
                    None,
                    None,
                    Some(valid_v),
                );
                let ani_kside_dirs_sum = side_sum_from_directional(
                    ani.knorth.view(),
                    ani.keast.view(),
                    ani.ksouth.view(),
                    ani.kwest.view(),
                    valid_v,
                );
                // Directional Lside comes entirely from the march
                // (Lside_veg_v2026 anisotropic returns zeros).
                let lside_dirs_sum = side_sum_from_directional(
                    march.gvf_lside_n.view(),
                    march.gvf_lside_e.view(),
                    march.gvf_lside_s.view(),
                    march.gvf_lside_w.view(),
                    valid_v,
                );
                // 2026a Tmrt composition: the Fcyl longwave term is the
                // anisotropic-sky Lside alone, as in 2025a. Upstream
                // dropped the mean-of-directions addend in umep-solweig
                // 866b137 (Solweig_2026a_calc).
                (
                    kdown,
                    ani.ldown,
                    ani_kside_dirs_sum,
                    lside_dirs_sum,
                    ani.kside,
                    ani.lside,
                )
            } else {
                let kside = kside_veg_isotropic_pure(
                    rad_i,
                    rad_d,
                    rad_g,
                    scheme_shadow.view(),
                    svf_s_v,
                    svf_w_v,
                    svf_n_v,
                    svf_e_v,
                    svf_veg_e_v,
                    svf_veg_s_v,
                    svf_veg_w_v,
                    svf_veg_n_v,
                    weather.sun_azimuth,
                    weather.sun_altitude,
                    psi,
                    0.0,
                    config.albedo_wall,
                    f_sh.view(),
                    kup_e.view(),
                    kup_s.view(),
                    kup_w.view(),
                    kup_n.view(),
                    cyl,
                    Some(valid_v),
                );
                let kside_dirs_sum = side_sum_from_directional(
                    kside.knorth.view(),
                    kside.keast.view(),
                    kside.ksouth.view(),
                    kside.kwest.view(),
                    valid_v,
                );
                // Directional Lside = march ground/wall side longwave +
                // Lside_veg_v2026 sky/wall/veg terms.
                let least_tot = &march.gvf_lside_e + &lside.least;
                let lsouth_tot = &march.gvf_lside_s + &lside.lsouth;
                let lwest_tot = &march.gvf_lside_w + &lside.lwest;
                let lnorth_tot = &march.gvf_lside_n + &lside.lnorth;
                let lside_dirs_sum = side_sum_from_directional(
                    lnorth_tot.view(),
                    least_tot.view(),
                    lsouth_tot.view(),
                    lwest_tot.view(),
                    valid_v,
                );
                (
                    kdown,
                    ldown_iso,
                    kside_dirs_sum,
                    lside_dirs_sum,
                    kside.kside_i,
                    Array2::<f32>::zeros(shape),
                )
            };

        // Lup (Fup upwelling) is the march output — no thermal delay.
        let lup = march.gvf_lup;

        let tmrt = compute_tmrt_from_dir_sums_pure(
            kdown_out.view(),
            kup.view(),
            ldown_out.view(),
            lup.view(),
            kside_dirs_sum.view(),
            lside_dirs_sum.view(),
            kside_total.view(),
            lside_total.view(),
            human.abs_k,
            human.abs_l,
            human.is_standing,
            use_aniso,
        );

        return TimestepResultRaw {
            tmrt,
            // The scheme carries shadow_past forward from this shadow;
            // return it whenever the caller asked for shadow output.
            shadow: if want_shadow { Some(scheme_shadow) } else { None },
            kdown: if want_kdown { Some(kdown_out) } else { None },
            kup: if want_kup { Some(kup) } else { None },
            ldown: if want_ldown { Some(ldown_out) } else { None },
            lup: if want_lup { Some(lup) } else { None },
            // Thermal-delay state is untouched by the scheme (the
            // force-restore ODE carries the inertia); pass it through.
            timeadd,
            tgmap1: tgmap1_v.to_owned(),
            tgmap1_e: tgmap1_e_v.to_owned(),
            tgmap1_s: tgmap1_s_v.to_owned(),
            tgmap1_w: tgmap1_w_v.to_owned(),
            tgmap1_n: tgmap1_n_v.to_owned(),
            tgout1: tgout1_v.to_owned(),
            tg: Some(st.tg),
            rn: Some(st.rn),
            rn_past: Some(st.rn_past),
            g: Some(st.g),
        };
    }

    // ── Step 3: GVF ─────────────────────────────────────────────────────
    let t_gvf = Instant::now();
    let first = {
        let h = human.height.round();
        if h == 0.0 {
            1.0
        } else {
            h
        }
    };
    let second = (human.height * 20.0).round();

    let gvf: GvfResultPure = if config.has_walls {
        if let Some(cache) = gvf_cache_inner {
            // Gated by config.has_walls=true (validated at entry).
            let wh = wall_ht_v.expect("wall_ht: gated by config.has_walls=true");
            // Try GPU first, fall back to CPU
            #[cfg(feature = "gpu")]
            {
                if let Some(ctx) = get_gvf_gpu_context() {
                    match gvf_calc_with_cache_gpu(
                        ctx,
                        cache,
                        wallsun.view(),
                        wh,
                        buildings_v,
                        shadow_f32.view(),
                        ground.tg.view(),
                        ground.tg_wall,
                        weather.ta,
                        emis_grid_v,
                        config.emis_wall,
                        alb_grid_v,
                        SBC,
                        config.albedo_wall,
                        weather.ta,
                        lc_grid_v,
                        lc_grid_v.is_some(),
                    ) {
                        Ok(result) => {
                            crate::shadowing::record_gpu_dispatch();
                            result
                        }
                        Err(e) => {
                            eprintln!("[GPU] GVF GPU dispatch failed, falling back to CPU: {}", e);
                            crate::shadowing::record_gpu_fallback();
                            GVF_GPU_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
                            gvf_calc_with_cache(
                                cache,
                                wallsun.view(),
                                wh,
                                buildings_v,
                                shadow_f32.view(),
                                ground.tg.view(),
                                ground.tg_wall,
                                weather.ta,
                                emis_grid_v,
                                config.emis_wall,
                                alb_grid_v,
                                SBC,
                                config.albedo_wall,
                                weather.ta,
                                lc_grid_v,
                                lc_grid_v.is_some(),
                            )
                        }
                    }
                } else {
                    gvf_calc_with_cache(
                        cache,
                        wallsun.view(),
                        wh,
                        buildings_v,
                        shadow_f32.view(),
                        ground.tg.view(),
                        ground.tg_wall,
                        weather.ta,
                        emis_grid_v,
                        config.emis_wall,
                        alb_grid_v,
                        SBC,
                        config.albedo_wall,
                        weather.ta,
                        lc_grid_v,
                        lc_grid_v.is_some(),
                    )
                }
            }
            #[cfg(not(feature = "gpu"))]
            gvf_calc_with_cache(
                cache,
                wallsun.view(),
                wh,
                buildings_v,
                shadow_f32.view(),
                ground.tg.view(),
                ground.tg_wall,
                weather.ta,
                emis_grid_v,
                config.emis_wall,
                alb_grid_v,
                SBC,
                config.albedo_wall,
                weather.ta, // twater = ta
                lc_grid_v,
                lc_grid_v.is_some(),
            )
        } else {
            // Full GVF (first timestep or no cache).
            // wall_ht_v and wall_asp_v are guaranteed Some here: this
            // branch is gated by config.has_walls=true, which we
            // validate at function entry (line ~990) rejecting calls
            // without both arrays.
            let wh = wall_ht_v.expect("wall_ht: gated by config.has_walls=true");
            gvf_calc_pure(
                wallsun.view(),
                wh,
                buildings_v,
                config.pixel_size,
                shadow_f32.view(),
                first,
                second,
                wall_asp_v.expect("wall_asp: gated by config.has_walls=true"),
                ground.tg.view(),
                ground.tg_wall,
                weather.ta,
                emis_grid_v,
                config.emis_wall,
                alb_grid_v,
                SBC,
                config.albedo_wall,
                weather.ta, // twater = ta
                lc_grid_v,
                lc_grid_v.is_some(),
            )
        }
    } else {
        // Simplified GVF (no walls) - compute inline
        let gvf_simple = 1.0 - &svf_v;
        let tg_with_shadow = &ground.tg * &shadow_f32;
        // Lup = emis × SBC × (Ta + Tg_shadow + 273.15)^4
        let lup_simple = {
            let mut arr = Array2::<f32>::zeros(shape);
            Zip::indexed(&mut arr).par_for_each(|(r, c), out| {
                if valid_v[[r, c]] == 0 {
                    *out = f32::NAN;
                    return;
                }
                let t = weather.ta + tg_with_shadow[[r, c]] + KELVIN_OFFSET;
                *out = emis_grid_v[[r, c]] * SBC * t.powi(4);
            });
            arr
        };
        let gvfalb_simple = &alb_grid_v * &gvf_simple;

        GvfResultPure {
            gvf_lup: lup_simple.clone(),
            gvfalb: gvfalb_simple.clone(),
            gvfalbnosh: Some(alb_grid_v.to_owned()),
            gvf_lup_e: lup_simple.clone(),
            gvfalb_e: gvfalb_simple.clone(),
            gvfalbnosh_e: Some(alb_grid_v.to_owned()),
            gvf_lup_s: lup_simple.clone(),
            gvfalb_s: gvfalb_simple.clone(),
            gvfalbnosh_s: Some(alb_grid_v.to_owned()),
            gvf_lup_w: lup_simple.clone(),
            gvfalb_w: gvfalb_simple.clone(),
            gvfalbnosh_w: Some(alb_grid_v.to_owned()),
            gvf_lup_n: lup_simple.clone(),
            gvfalb_n: gvfalb_simple,
            gvfalbnosh_n: Some(alb_grid_v.to_owned()),
            gvf_sum: Some(Array2::zeros(shape)),
            gvf_norm: Some(Array2::ones(shape)),
        }
    };

    let gvf_dur = t_gvf.elapsed();

    // ── Step 4: Thermal Delay ────────────────────────────────────────────
    let t_delay = Instant::now();
    let tg_temp = &ground.tg * &shadow_f32 + weather.ta;

    let delay = ts_wave_delay_batch_pure(
        gvf.gvf_lup.view(),
        gvf.gvf_lup_e.view(),
        gvf.gvf_lup_s.view(),
        gvf.gvf_lup_w.view(),
        gvf.gvf_lup_n.view(),
        tg_temp.view(),
        firstdaytime,
        timeadd,
        timestep_dec,
        tgmap1_v,
        tgmap1_e_v,
        tgmap1_s_v,
        tgmap1_w_v,
        tgmap1_n_v,
        tgout1_v,
    );

    let delay_dur = t_delay.elapsed();

    // ── Step 5: Radiation ─────────────────────────────────────────────────
    let t_radiation = Instant::now();
    let esky = compute_esky(weather.ta, weather.rh);
    let sin_alt = (weather.sun_altitude * PI / 180.0).sin();
    let rad_i = weather.direct_rad;
    let rad_d = weather.diffuse_rad;
    let rad_g = weather.global_rad;
    let psi = weather.psi;
    let cyl = human.is_standing;

    // F_sh (cylindric wedge shadow fraction) — shared by both paths
    let zen_rad = weather.sun_zenith * PI / 180.0;
    let f_sh = cylindric_wedge_pure_masked(zen_rad, svfalfa_v, Some(valid_v));

    // Kup helper used in both isotropic and anisotropic paths.
    // In cached-GVF mode, read gvfalbnosh* directly from geometry cache to
    // avoid per-timestep cloning of those static arrays.
    let compute_kup_with =
        |gvfalbnosh: ArrayView2<f32>,
         gvfalbnosh_e: ArrayView2<f32>,
         gvfalbnosh_s: ArrayView2<f32>,
         gvfalbnosh_w: ArrayView2<f32>,
         gvfalbnosh_n: ArrayView2<f32>| {
            compute_kup(
                rad_i,
                rad_d,
                rad_g,
                weather.sun_altitude,
                svfbuveg_v,
                config.albedo_wall,
                f_sh.view(),
                gvf.gvfalb.view(),
                gvf.gvfalb_e.view(),
                gvf.gvfalb_s.view(),
                gvf.gvfalb_w.view(),
                gvf.gvfalb_n.view(),
                gvfalbnosh,
                gvfalbnosh_e,
                gvfalbnosh_s,
                gvfalbnosh_w,
                gvfalbnosh_n,
                valid_v,
            )
        };
    let compute_kup_all = || {
        if let Some(cache) = gvf_cache_inner {
            compute_kup_with(
                cache.cached_albnosh.view(),
                cache.cached_albnosh_e.view(),
                cache.cached_albnosh_s.view(),
                cache.cached_albnosh_w.view(),
                cache.cached_albnosh_n.view(),
            )
        } else {
            let gvfalbnosh = gvf
                .gvfalbnosh
                .as_ref()
                .expect("gvfalbnosh missing without cache");
            let gvfalbnosh_e = gvf
                .gvfalbnosh_e
                .as_ref()
                .expect("gvfalbnosh_e missing without cache");
            let gvfalbnosh_s = gvf
                .gvfalbnosh_s
                .as_ref()
                .expect("gvfalbnosh_s missing without cache");
            let gvfalbnosh_w = gvf
                .gvfalbnosh_w
                .as_ref()
                .expect("gvfalbnosh_w missing without cache");
            let gvfalbnosh_n = gvf
                .gvfalbnosh_n
                .as_ref()
                .expect("gvfalbnosh_n missing without cache");
            compute_kup_with(
                gvfalbnosh.view(),
                gvfalbnosh_e.view(),
                gvfalbnosh_s.view(),
                gvfalbnosh_w.view(),
                gvfalbnosh_n.view(),
            )
        }
    };

    // Branch: anisotropic vs isotropic
    let use_aniso = config.use_anisotropic
        && shmat_v.is_some()
        && vegshmat_v.is_some()
        && vbshmat_v.is_some();

    let (kup, kdown, ldown, kside_dirs_sum, lside_dirs_sum, kside_total, lside_total) =
        if use_aniso {
            // === Anisotropic sky ===
            // The three shadow matrices are guaranteed Some by use_aniso
            // (lines just above check .is_some() on all three).
            let shmat_a = shmat_v.expect("shmat: gated by use_aniso");
            let vegshmat_a = vegshmat_v.expect("vegshmat: gated by use_aniso");
            let vbshmat_a = vbshmat_v.expect("vbshmat: gated by use_aniso");

            // Perez sky luminance distribution (computed in Rust — no Python round-trip)
            let lv_arr = crate::perez::perez_v3(
                weather.zen_deg,
                weather.sun_azimuth,
                weather.diffuse_rad,
                weather.direct_rad,
                weather.jday,
                weather.patch_option,
            );
            let patch_lut = patch_lut_for_option_cached(weather.patch_option);
            #[allow(unused_variables)] // GPU branch only
            let patch_altitude_arr = ArrayView1::from(patch_lut.altitudes.as_slice());
            #[allow(unused_variables)] // GPU branch only
            let patch_azimuth_arr = ArrayView1::from(patch_lut.azimuths.as_slice());
            let steradians_arr = ArrayView1::from(patch_lut.steradians.as_slice());
            let patch_altitude_sin_arr =
                ArrayView1::from(patch_lut.altitude_sin.as_slice());

            // ASVF from SVF (arccos(sqrt(clip(svf, 0, 1)))) cached by SVF buffer.
            // The cache key already includes (nrows, ncols, hash), so the cached
            // buffer length equals shape.0 * shape.1 by construction. The expect
            // is kept as a defensive guard with a precise diagnostic.
            let asvf_cache = asvf_for_svf_cached(svf_v);
            let asvf_arr =
                ArrayView2::from_shape(shape, asvf_cache.as_slice()).unwrap_or_else(|e| {
                    panic!(
                        "ASVF cache shape mismatch: requested {:?}, buffer len {} ({})",
                        shape,
                        asvf_cache.len(),
                        e
                    )
                });

            // Esky anisotropic (Jonsson + CI correction)
            let esky_a = {
                let ci = weather.clearness_index;
                if ci < 0.95 {
                    ci * esky + (1.0 - ci)
                } else {
                    esky
                }
            };

            // Full anisotropic sky calculation (ldown, kside, lside totals)
            // Try GPU path first; fall back to CPU if unavailable.
            #[allow(unused_variables)] // GPU branch only
            let deg2rad = PI / 180.0;
            #[allow(unused_variables)]
            let (lum_chi, rad_tot) = if weather.sun_altitude > 0.0 {
                let patch_luminance = lv_arr.column(2);
                let mut rad_tot = 0.0f32;
                let n_patches = patch_luminance.len();
                for i in 0..n_patches {
                    rad_tot +=
                        patch_luminance[i] * steradians_arr[i] * patch_altitude_sin_arr[i];
                }
                if rad_tot > 0.0 {
                    (patch_luminance.mapv(|lum| (lum * rad_d) / rad_tot), rad_tot)
                } else {
                    (Array1::<f32>::zeros(n_patches), 0.0)
                }
            } else {
                (Array1::<f32>::zeros(lv_arr.shape()[0]), 0.0)
            };

            #[allow(unused_variables)]
            let (_, esky_band) =
                crate::emissivity_models::model2(&lv_arr, esky_a, weather.ta);

            // Launch anisotropic GPU dispatch early, then compute Kup/lside_dirs
            // while GPU work is in flight.
            #[cfg(feature = "gpu")]
            let mut gpu_ctx = None;
            #[cfg(feature = "gpu")]
            let mut gpu_pending = None;

            #[cfg(feature = "gpu")]
            if let Some(ctx) = get_aniso_gpu_context() {
                match ctx.dispatch_begin(
                    shmat_a,
                    vegshmat_a,
                    vbshmat_a,
                    asvf_arr,
                    delay.lup.view(),
                    valid_v,
                    patch_altitude_arr,
                    patch_azimuth_arr,
                    steradians_arr,
                    esky_band.view(),
                    lum_chi.view(),
                    weather.sun_altitude,
                    weather.sun_azimuth,
                    weather.ta,
                    cyl,
                    config.albedo_wall,
                    ground.tg_wall,
                    config.emis_wall,
                    rad_i,
                    rad_d,
                    psi,
                    rad_tot,
                ) {
                    Ok(pending) => {
                        gpu_ctx = Some(ctx);
                        gpu_pending = Some(pending);
                    }
                    Err(e) => {
                        eprintln!(
                            "[GPU] Anisotropic dispatch begin failed: {}. CPU fallback.",
                            e
                        );
                        crate::shadowing::record_gpu_fallback();
                    }
                }
            }

            // Shared thermal side inputs (always needed in anisotropic mode).
            let (kup, kup_e, kup_s, kup_w, kup_n) = compute_kup_all();

            let lside_dirs_sum = lside_dirs_sum_aniso_from_lup(
                delay.lup_e.view(),
                delay.lup_s.view(),
                delay.lup_w.view(),
                delay.lup_n.view(),
                valid_v,
            );

            #[cfg(feature = "gpu")]
            let gpu_result = if let (Some(ctx), Some(pending)) = (gpu_ctx, gpu_pending) {
                match ctx.dispatch_end(pending) {
                    Ok(gpu) => {
                        crate::shadowing::record_gpu_dispatch();
                        Some(gpu)
                    }
                    Err(e) => {
                        eprintln!(
                            "[GPU] Anisotropic dispatch end failed: {}. CPU fallback.",
                            e
                        );
                        crate::shadowing::record_gpu_fallback();
                        None
                    }
                }
            } else {
                None
            };

            // Compute anisotropic sky: GPU path + CPU fallback
            #[allow(unused_mut)]
            let mut used_gpu = false;
            #[allow(unused_mut)]
            let mut ani_ldown = Array2::<f32>::zeros(shape);
            #[allow(unused_mut)]
            let mut ani_lside = Array2::<f32>::zeros(shape);
            #[allow(unused_mut)]
            let mut ani_kside = Array2::<f32>::zeros(shape);
            #[allow(unused_mut)]
            let mut drad = Array2::<f32>::zeros(shape);
            #[allow(unused_mut)]
            let mut ani_kside_dirs_sum = Array2::<f32>::zeros(shape);

            #[cfg(feature = "gpu")]
            if let Some(gpu) = gpu_result {
                // GPU path: derive kside and k-directional from GPU partial outputs
                let kside_i = if cyl {
                    &shadow_f32 * rad_i * (weather.sun_altitude * deg2rad).cos()
                } else {
                    Array2::<f32>::zeros(shape)
                };
                if weather.sun_altitude > 0.0 {
                    ani_kside = kside_i + &gpu.kside_partial;
                    // Ground-reflected shortwave to directional (both postures)
                    ani_kside_dirs_sum = kside_dirs_sum_aniso_from_kup(
                        kup_e.view(),
                        kup_s.view(),
                        kup_w.view(),
                        kup_n.view(),
                        valid_v,
                    );
                    // Box posture: add direct beam to directional faces
                    if !cyl {
                        let cos_alt =
                            (weather.sun_altitude * deg2rad).cos();
                        let azi = weather.sun_azimuth;
                        let mut kdir_box = Array2::<f32>::zeros(shape);
                        // East face
                        if azi > 360.0 || azi <= 180.0 {
                            kdir_box += &(&shadow_f32
                                * rad_i
                                * cos_alt
                                * (azi * deg2rad).sin());
                        }
                        // South face
                        if azi > 90.0 && azi <= 270.0 {
                            kdir_box += &(&shadow_f32
                                * rad_i
                                * cos_alt
                                * ((azi - 90.0) * deg2rad).sin());
                        }
                        // West face
                        if azi > 180.0 && azi <= 360.0 {
                            kdir_box += &(&shadow_f32
                                * rad_i
                                * cos_alt
                                * ((azi - 180.0) * deg2rad).sin());
                        }
                        // North face
                        if azi <= 90.0 || azi > 270.0 {
                            kdir_box += &(&shadow_f32
                                * rad_i
                                * cos_alt
                                * ((azi - 270.0) * deg2rad).sin());
                        }
                        ani_kside_dirs_sum += &kdir_box;
                    }
                }
                ani_ldown = gpu.ldown;
                ani_lside = gpu.lside;
                drad = gpu.drad;
                used_gpu = true;
            }

            if !used_gpu {
                // drad via direct accumulation from packed shadow matrices.
                // Shadow matrices are bitpacked: 1 bit per patch, 8 patches per byte.
                // ani_lum = sum_i((sh_i - (1 - veg_i) * (1 - psi)) * lv_i)
                let lv_col2 = lv_arr.column(2);
                let ani_lum =
                    compute_ani_lum_from_packed(shmat_a, vegshmat_a, lv_col2, psi, valid_v);
                drad = ani_lum.mapv(|v| v * rad_d);

                let ani = anisotropic_sky_pure(
                    shmat_a,
                    vegshmat_a,
                    vbshmat_a,
                    weather.sun_altitude,
                    weather.sun_azimuth,
                    esky_a,
                    weather.ta,
                    cyl,
                    false, // wall_scheme
                    config.albedo_wall,
                    ground.tg_wall,
                    config.emis_wall,
                    rad_i,
                    rad_d,
                    asvf_arr,
                    lv_arr.view(),
                    steradians_arr,
                    delay.lup.view(),
                    lv_arr.view(),
                    shadow_f32.view(),
                    kup_e.view(),
                    kup_s.view(),
                    kup_w.view(),
                    kup_n.view(),
                    None, // voxel_table
                    None, // voxel_maps
                    Some(valid_v),
                );
                ani_ldown = ani.ldown;
                ani_lside = ani.lside;
                ani_kside = ani.kside;
                ani_kside_dirs_sum = side_sum_from_directional(
                    ani.knorth.view(),
                    ani.keast.view(),
                    ani.ksouth.view(),
                    ani.kwest.view(),
                    valid_v,
                );
            }

            // Kdown (shared formula, but with anisotropic drad)
            let kdown = compute_kdown(
                rad_i,
                rad_d,
                rad_g,
                shadow_f32.view(),
                sin_alt,
                svfbuveg_v,
                config.albedo_wall,
                f_sh.view(),
                drad.view(),
                valid_v,
            );

            // From anisotropic: ldown from ani_sky, lside from lside_veg, kside from ani_sky
            (
                kup,
                kdown,
                ani_ldown,
                ani_kside_dirs_sum,
                lside_dirs_sum,
                ani_kside,
                ani_lside,
            )
        } else {
            // === Isotropic sky ===
            let (kup, kup_e, kup_s, kup_w, kup_n) = compute_kup_all();

            // drad (isotropic diffuse)
            let drad = svfbuveg_v.mapv(|sv| rad_d * sv);

            // Ldown
            let ldown = compute_ldown(
                esky,
                weather.ta,
                ground.tg_wall,
                svf_v,
                svf_veg_v,
                svf_aveg_v,
                config.emis_wall,
                weather.clearness_index,
                valid_v,
            );

            // kside_veg (isotropic)
            let kside = kside_veg_isotropic_pure(
                rad_i,
                rad_d,
                rad_g,
                shadow_f32.view(),
                svf_s_v,
                svf_w_v,
                svf_n_v,
                svf_e_v,
                svf_veg_e_v,
                svf_veg_s_v,
                svf_veg_w_v,
                svf_veg_n_v,
                weather.sun_azimuth,
                weather.sun_altitude,
                psi,
                0.0, // t (instrument offset)
                config.albedo_wall,
                f_sh.view(),
                kup_e.view(),
                kup_s.view(),
                kup_w.view(),
                kup_n.view(),
                cyl,
                Some(valid_v),
            );

            // lside_veg (isotropic)
            let lside = lside_veg_pure(
                svf_s_v,
                svf_w_v,
                svf_n_v,
                svf_e_v,
                svf_veg_e_v,
                svf_veg_s_v,
                svf_veg_w_v,
                svf_veg_n_v,
                svf_aveg_e_v,
                svf_aveg_s_v,
                svf_aveg_w_v,
                svf_aveg_n_v,
                weather.sun_azimuth,
                weather.sun_altitude,
                weather.ta,
                ground.tg_wall,
                SBC,
                config.emis_wall,
                ldown.view(),
                esky,
                0.0, // t
                f_sh.view(),
                weather.clearness_index,
                delay.lup_e.view(),
                delay.lup_s.view(),
                delay.lup_w.view(),
                delay.lup_n.view(),
                false, // isotropic
                Some(valid_v),
            );

            // Kdown
            let kdown = compute_kdown(
                rad_i,
                rad_d,
                rad_g,
                shadow_f32.view(),
                sin_alt,
                svfbuveg_v,
                config.albedo_wall,
                f_sh.view(),
                drad.view(),
                valid_v,
            );

            // Isotropic: kside_total = kside_i, lside_total = zeros
            let kside_dirs_sum = side_sum_from_directional(
                kside.knorth.view(),
                kside.keast.view(),
                kside.ksouth.view(),
                kside.kwest.view(),
                valid_v,
            );
            let lside_dirs_sum = side_sum_from_directional(
                lside.lnorth.view(),
                lside.least.view(),
                lside.lsouth.view(),
                lside.lwest.view(),
                valid_v,
            );
            (
                kup,
                kdown,
                ldown,
                kside_dirs_sum,
                lside_dirs_sum,
                kside.kside_i,
                Array2::<f32>::zeros(shape),
            )
        };

    let radiation_dur = t_radiation.elapsed();

    // ── Step 6: Tmrt ─────────────────────────────────────────────────────
    let t_tmrt = Instant::now();
    let tmrt = compute_tmrt_from_dir_sums_pure(
        kdown.view(),
        kup.view(),
        ldown.view(),
        delay.lup.view(),
        kside_dirs_sum.view(),
        lside_dirs_sum.view(),
        kside_total.view(),
        lside_total.view(),
        human.abs_k,
        human.abs_l,
        human.is_standing,
        use_aniso,
    );
    let tmrt_dur = t_tmrt.elapsed();

    if timing_enabled() {
        let total =
            shadow_dur + ground_dur + gvf_dur + delay_dur + radiation_dur + tmrt_dur;
        let total_ms = total.as_secs_f64() * 1000.0;
        let shadow_ms = shadow_dur.as_secs_f64() * 1000.0;
        let ground_ms = ground_dur.as_secs_f64() * 1000.0;
        let gvf_ms = gvf_dur.as_secs_f64() * 1000.0;
        let delay_ms = delay_dur.as_secs_f64() * 1000.0;
        let rad_ms = radiation_dur.as_secs_f64() * 1000.0;
        let tmrt_ms = tmrt_dur.as_secs_f64() * 1000.0;
        // GPU duty cycle: shadow always uses GPU (when available);
        // radiation includes GPU aniso dispatch when anisotropic is active.
        let gpu_ms = shadow_ms + if use_aniso { rad_ms } else { 0.0 };
        let duty = if total_ms > 0.0 {
            gpu_ms / total_ms * 100.0
        } else {
            0.0
        };
        eprintln!(
            "[TIMING] shadow={:.1}ms ground={:.1}ms gvf={:.1}ms delay={:.1}ms \
     radiation={:.1}ms tmrt={:.1}ms | total={:.1}ms gpu_duty={:.0}%",
            shadow_ms, ground_ms, gvf_ms, delay_ms, rad_ms, tmrt_ms, total_ms, duty,
        );
    }

    TimestepResultRaw {
        tmrt,
        shadow: if want_shadow { Some(shadow_f32) } else { None },
        kdown: if want_kdown { Some(kdown) } else { None },
        kup: if want_kup { Some(kup) } else { None },
        ldown: if want_ldown { Some(ldown) } else { None },
        lup: if want_lup { Some(delay.lup) } else { None },
        timeadd: delay.timeadd,
        tgmap1: delay.tgmap1,
        tgmap1_e: delay.tgmap1_e,
        tgmap1_s: delay.tgmap1_s,
        tgmap1_w: delay.tgmap1_w,
        tgmap1_n: delay.tgmap1_n,
        tgout1: delay.tgout1,
        // Ground scheme inactive on the classic path.
        tg: None,
        rn: None,
        rn_past: None,
        g: None,
    }
}
