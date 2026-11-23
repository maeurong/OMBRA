use ndarray::{Array2, ArrayBase, ArrayView2, Data, DataMut, Ix2};
use rayon::prelude::*;

// Convenience accessors that document the invariant once instead of in every
// call site. All Array2<f32>s and views in this module are either allocated
// via Array2::zeros (standard C-order contiguous) or are views over PyO3
// numpy arrays that we have verified to be contiguous at the FFI boundary.
// The .expect() messages are diagnostic in case that invariant is ever
// violated by a future refactor.
const VEG_CONTIG_INVARIANT: &str =
    "vegetation.rs invariant: all f32 arrays here are contiguous (Array2::zeros or contiguous numpy views)";

#[inline(always)]
fn as_slice_checked<D>(arr: &ArrayBase<D, Ix2>) -> &[f32]
where
    D: Data<Elem = f32>,
{
    arr.as_slice().expect(VEG_CONTIG_INVARIANT)
}

#[inline(always)]
fn as_slice_mut_checked<D>(arr: &mut ArrayBase<D, Ix2>) -> &mut [f32]
where
    D: DataMut<Elem = f32>,
{
    arr.as_slice_mut().expect(VEG_CONTIG_INVARIANT)
}

/// Pure result type for lside_veg (no PyO3 dependency).
pub(crate) struct LsideVegPureResult {
    pub least: Array2<f32>,
    pub lsouth: Array2<f32>,
    pub lwest: Array2<f32>,
    pub lnorth: Array2<f32>,
}

/// Lside model variant.
///
/// `V2022a` is UMEP's default (and this port's validated baseline).
/// `V2026` is the variant introduced with UMEP's opt-in 2026a
/// ground-surface scheme (Bridoux, University of Gothenburg, merged
/// 2026-05): the ground term leaves Lside entirely (the scheme's
/// `outgoing_longwave_calc` solid-angle march supplies directional
/// ground+wall longwave instead), the reflection term drops the Lup
/// contribution (`Lrefl = Ldown * viktrefl * (1-ewall) * 0.5`), and the
/// anisotropic branch returns zeros. Reference:
/// tests/reference/umep_2026a/Lside_veg.py (`Lside_veg_v2026`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LsideVariant {
    V2022a,
    V2026,
}

/// Pure-ndarray implementation of Lside_veg (v2022a / v2026 variants).
/// Callable from pipeline.rs (fused path) or from the PyO3 wrappers
/// (modular path). The `Lup*` views are unused by `V2026` (upstream's
/// v2026 signature has no Lup parameters); callers may pass any
/// same-shaped array for that variant.
#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn lside_veg_variant_pure(
    variant: LsideVariant,
    svfS: ArrayView2<f32>,
    svfW: ArrayView2<f32>,
    svfN: ArrayView2<f32>,
    svfE: ArrayView2<f32>,
    svfEveg: ArrayView2<f32>,
    svfSveg: ArrayView2<f32>,
    svfWveg: ArrayView2<f32>,
    svfNveg: ArrayView2<f32>,
    svfEaveg: ArrayView2<f32>,
    svfSaveg: ArrayView2<f32>,
    svfWaveg: ArrayView2<f32>,
    svfNaveg: ArrayView2<f32>,
    azimuth: f32,
    altitude: f32,
    Ta: f32,
    Tw: f32,
    SBC: f32,
    ewall: f32,
    Ldown: ArrayView2<f32>,
    esky: f32,
    t: f32,
    F_sh: ArrayView2<f32>,
    CI: f32,
    LupE: ArrayView2<f32>,
    LupS: ArrayView2<f32>,
    LupW: ArrayView2<f32>,
    LupN: ArrayView2<f32>,
    anisotropic_longwave: bool,
    valid: Option<ArrayView2<u8>>,
) -> LsideVegPureResult {
    let shape = svfE.shape();
    let (rows, cols) = (shape[0], shape[1]);
    let vikttot: f32 = 4.4897;
    let TaK = Ta + 273.15;
    let TaK_pow4 = TaK.powi(4);
    let c = 1.0 - CI;
    let Lsky_allsky = esky * SBC * TaK_pow4 * (1.0 - c) + c * SBC * TaK_pow4;
    let altitude_day = altitude > 0.0;

    let sun_east = azimuth > (180.0 - t) && azimuth <= (360.0 - t);
    let sun_south = azimuth <= (90.0 - t) || azimuth > (270.0 - t);
    let sun_west = azimuth > (360.0 - t) || azimuth <= (180.0 - t);
    let sun_north = azimuth > (90.0 - t) && azimuth <= (270.0 - t);

    let temp_e = TaK + Tw * ((azimuth - 180.0 + t) * std::f32::consts::PI / 180.0).sin();
    let temp_s = TaK + Tw * ((azimuth - 270.0 + t) * std::f32::consts::PI / 180.0).sin();
    let temp_w = TaK + Tw * ((azimuth + t) * std::f32::consts::PI / 180.0).sin();
    let temp_n = TaK + Tw * ((azimuth - 90.0 + t) * std::f32::consts::PI / 180.0).sin();

    #[inline]
    fn poly(x: f32) -> f32 {
        63.227 * x.powi(6) - 161.51 * x.powi(5) + 156.91 * x.powi(4) - 70.424 * x.powi(3)
            + 16.773 * x.powi(2)
            - 0.4863 * x
    }

    let npix = rows * cols;
    let mut least_vec = vec![0.0f32; npix];
    let mut lsouth_vec = vec![0.0f32; npix];
    let mut lwest_vec = vec![0.0f32; npix];
    let mut lnorth_vec = vec![0.0f32; npix];

    least_vec
        .par_iter_mut()
        .zip(lsouth_vec.par_iter_mut())
        .zip(lwest_vec.par_iter_mut())
        .zip(lnorth_vec.par_iter_mut())
        .enumerate()
        .for_each(|(idx, (((least, lsouth), lwest), lnorth))| {
            let r = idx / cols;
            let c = idx % cols;
            if let Some(ref v) = valid {
                if v[[r, c]] == 0 {
                    *least = f32::NAN;
                    *lsouth = f32::NAN;
                    *lwest = f32::NAN;
                    *lnorth = f32::NAN;
                    return;
                }
            }
            let compute = |svf: f32,
                           svfveg: f32,
                           svfaveg: f32,
                           lup: f32,
                           sun_cond: bool,
                           temp_wall: f32|
             -> f32 {
                if anisotropic_longwave {
                    // v2026: directional ground/wall longwave comes entirely
                    // from the ground scheme's gvfLside*; Lside contributes 0.
                    return match variant {
                        LsideVariant::V2022a => lup * 0.5,
                        LsideVariant::V2026 => 0.0,
                    };
                }
                let svfalfa = (1.0 - svf).clamp(0.0, 1.0).sqrt().asin();
                let poly_svf_val = poly(svf);
                let poly_svfaveg_val = poly(svfaveg);
                let viktonlywall = (vikttot - poly_svf_val) / vikttot;
                let viktaveg = (vikttot - poly_svfaveg_val) / vikttot;
                let viktwall = viktonlywall - viktaveg;
                let svfvegbu = svfveg + svf - 1.0;
                let poly_svfvegbu = poly(svfvegbu);
                let viktsky = poly_svfvegbu / vikttot;
                let viktrefl = (vikttot - poly_svfvegbu) / vikttot;
                let viktveg = viktrefl - viktwall;
                let fsh_val = F_sh[(r, c)];
                let ldown_val = Ldown[(r, c)];
                let f_sh_scaled = 2.0 * fsh_val - 1.0;
                let (lwallsun, lwallsh) = if altitude_day {
                    let alfa_b = svfalfa.atan();
                    let beta_b = (svfalfa * f_sh_scaled).tan().atan();
                    let betasun = ((alfa_b - beta_b) / 2.0) + beta_b;
                    if sun_cond {
                        let lwallsun = SBC
                            * ewall
                            * temp_wall.powi(4)
                            * viktwall
                            * (1.0 - f_sh_scaled)
                            * betasun.cos()
                            * 0.5;
                        let lwallsh = SBC * ewall * TaK_pow4 * viktwall * f_sh_scaled * 0.5;
                        (lwallsun, lwallsh)
                    } else {
                        let lwallsun = 0.0;
                        let lwallsh = SBC * ewall * TaK_pow4 * viktwall * 0.5;
                        (lwallsun, lwallsh)
                    }
                } else {
                    (0.0, SBC * ewall * TaK_pow4 * viktwall * 0.5)
                };
                let lsky = ((svf + svfveg - 1.0) * Lsky_allsky) * viktsky * 0.5;
                let lveg = SBC * ewall * TaK_pow4 * viktveg * 0.5;
                match variant {
                    LsideVariant::V2022a => {
                        let lground = lup * 0.5;
                        let lrefl = (ldown_val + lup) * viktrefl * (1.0 - ewall) * 0.5;
                        lsky + lwallsun + lwallsh + lveg + lground + lrefl
                    }
                    // v2026: ground term moves to outgoing_longwave_calc;
                    // reflection no longer includes Lup.
                    LsideVariant::V2026 => {
                        let lrefl = ldown_val * viktrefl * (1.0 - ewall) * 0.5;
                        lsky + lwallsun + lwallsh + lveg + lrefl
                    }
                }
            };
            *least = compute(
                svfE[(r, c)],
                svfEveg[(r, c)],
                svfEaveg[(r, c)],
                LupE[(r, c)],
                sun_east,
                temp_e,
            );
            *lsouth = compute(
                svfS[(r, c)],
                svfSveg[(r, c)],
                svfSaveg[(r, c)],
                LupS[(r, c)],
                sun_south,
                temp_s,
            );
            *lwest = compute(
                svfW[(r, c)],
                svfWveg[(r, c)],
                svfWaveg[(r, c)],
                LupW[(r, c)],
                sun_west,
                temp_w,
            );
            *lnorth = compute(
                svfN[(r, c)],
                svfNveg[(r, c)],
                svfNaveg[(r, c)],
                LupN[(r, c)],
                sun_north,
                temp_n,
            );
        });

    // INVARIANT: each *_vec was filled by Zip::indexed over the (rows, cols) shape
    // above, so len == rows * cols. `from_shape_vec` cannot fail here; the
    // debug_assert pins the invariant if the loop above is ever changed.
    debug_assert_eq!(least_vec.len(), rows * cols);
    debug_assert_eq!(lsouth_vec.len(), rows * cols);
    debug_assert_eq!(lwest_vec.len(), rows * cols);
    debug_assert_eq!(lnorth_vec.len(), rows * cols);
    LsideVegPureResult {
        least: Array2::from_shape_vec((rows, cols), least_vec)
            .expect("least_vec length mismatches (rows, cols)"),
        lsouth: Array2::from_shape_vec((rows, cols), lsouth_vec)
            .expect("lsouth_vec length mismatches (rows, cols)"),
        lwest: Array2::from_shape_vec((rows, cols), lwest_vec)
            .expect("lwest_vec length mismatches (rows, cols)"),
        lnorth: Array2::from_shape_vec((rows, cols), lnorth_vec)
            .expect("lnorth_vec length mismatches (rows, cols)"),
    }
}

/// Baseline v2022a entry point — the fused pipeline and the public
/// `lside_veg` pyfunction call this; behaviour is byte-identical to the
/// pre-variant implementation.
#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn lside_veg_pure(
    svfS: ArrayView2<f32>,
    svfW: ArrayView2<f32>,
    svfN: ArrayView2<f32>,
    svfE: ArrayView2<f32>,
    svfEveg: ArrayView2<f32>,
    svfSveg: ArrayView2<f32>,
    svfWveg: ArrayView2<f32>,
    svfNveg: ArrayView2<f32>,
    svfEaveg: ArrayView2<f32>,
    svfSaveg: ArrayView2<f32>,
    svfWaveg: ArrayView2<f32>,
    svfNaveg: ArrayView2<f32>,
    azimuth: f32,
    altitude: f32,
    Ta: f32,
    Tw: f32,
    SBC: f32,
    ewall: f32,
    Ldown: ArrayView2<f32>,
    esky: f32,
    t: f32,
    F_sh: ArrayView2<f32>,
    CI: f32,
    LupE: ArrayView2<f32>,
    LupS: ArrayView2<f32>,
    LupW: ArrayView2<f32>,
    LupN: ArrayView2<f32>,
    anisotropic_longwave: bool,
    valid: Option<ArrayView2<u8>>,
) -> LsideVegPureResult {
    lside_veg_variant_pure(
        LsideVariant::V2022a,
        svfS,
        svfW,
        svfN,
        svfE,
        svfEveg,
        svfSveg,
        svfWveg,
        svfNveg,
        svfEaveg,
        svfSaveg,
        svfWaveg,
        svfNaveg,
        azimuth,
        altitude,
        Ta,
        Tw,
        SBC,
        ewall,
        Ldown,
        esky,
        t,
        F_sh,
        CI,
        LupE,
        LupS,
        LupW,
        LupN,
        anisotropic_longwave,
        valid,
    )
}

/// Pure result type for kside_veg isotropic path (no PyO3 dependency).
pub(crate) struct KsideVegPureResult {
    pub keast: Array2<f32>,
    pub ksouth: Array2<f32>,
    pub kwest: Array2<f32>,
    pub knorth: Array2<f32>,
    pub kside_i: Array2<f32>,
}

/// Pure-ndarray implementation of Kside_veg_v2022a (isotropic path).
/// The anisotropic shortwave uses `anisotropic_sky_pure()` in the fused pipeline.
#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn kside_veg_isotropic_pure(
    radI: f32,
    radD: f32,
    radG: f32,
    shadow: ArrayView2<f32>,
    svfS: ArrayView2<f32>,
    svfW: ArrayView2<f32>,
    svfN: ArrayView2<f32>,
    svfE: ArrayView2<f32>,
    svfEveg: ArrayView2<f32>,
    svfSveg: ArrayView2<f32>,
    svfWveg: ArrayView2<f32>,
    svfNveg: ArrayView2<f32>,
    azimuth: f32,
    altitude: f32,
    psi: f32,
    t: f32,
    albedo: f32,
    F_sh: ArrayView2<f32>,
    KupE: ArrayView2<f32>,
    KupS: ArrayView2<f32>,
    KupW: ArrayView2<f32>,
    KupN: ArrayView2<f32>,
    cyl: bool,
    valid: Option<ArrayView2<u8>>,
) -> KsideVegPureResult {
    let shape = svfE.shape();
    let (rows, cols) = (shape[0], shape[1]);
    let deg2rad = std::f32::consts::PI / 180.0;
    let vikttot = 4.4897f32;

    let mut Keast = Array2::<f32>::zeros((rows, cols));
    let mut Ksouth = Array2::<f32>::zeros((rows, cols));
    let mut Kwest = Array2::<f32>::zeros((rows, cols));
    let mut Knorth = Array2::<f32>::zeros((rows, cols));
    let mut KsideI = Array2::<f32>::zeros((rows, cols));

    #[inline]
    fn kvikt_veg(svf: f32, svfveg: f32, vikttot: f32) -> (f32, f32) {
        let poly = |x: f32| -> f32 {
            63.227 * x.powi(6) - 161.51 * x.powi(5) + 156.91 * x.powi(4) - 70.424 * x.powi(3)
                + 16.773 * x.powi(2)
                - 0.4863 * x
        };
        let viktwall = (vikttot - poly(svf)) / vikttot;
        let svfvegbu = svfveg + svf - 1.0;
        let viktveg_tot = (vikttot - poly(svfvegbu)) / vikttot;
        let viktveg = viktveg_tot - viktwall;
        (viktveg, viktwall)
    }

    // Precompute svfviktbuveg arrays
    let mut svfviktbuvegE = Array2::<f32>::zeros((rows, cols));
    let mut svfviktbuvegS = Array2::<f32>::zeros((rows, cols));
    let mut svfviktbuvegW = Array2::<f32>::zeros((rows, cols));
    let mut svfviktbuvegN = Array2::<f32>::zeros((rows, cols));

    for r in 0..rows {
        for c in 0..cols {
            if let Some(ref v) = valid {
                if v[[r, c]] == 0 {
                    continue;
                }
            }
            let (vveg, vwall) = kvikt_veg(svfE[(r, c)], svfEveg[(r, c)], vikttot);
            svfviktbuvegE[(r, c)] = vwall + vveg * (1.0 - psi);
            let (vveg, vwall) = kvikt_veg(svfS[(r, c)], svfSveg[(r, c)], vikttot);
            svfviktbuvegS[(r, c)] = vwall + vveg * (1.0 - psi);
            let (vveg, vwall) = kvikt_veg(svfW[(r, c)], svfWveg[(r, c)], vikttot);
            svfviktbuvegW[(r, c)] = vwall + vveg * (1.0 - psi);
            let (vveg, vwall) = kvikt_veg(svfN[(r, c)], svfNveg[(r, c)], vikttot);
            svfviktbuvegN[(r, c)] = vwall + vveg * (1.0 - psi);
        }
    }

    // Direct radiation
    if cyl {
        for r in 0..rows {
            for c in 0..cols {
                KsideI[(r, c)] = shadow[(r, c)] * radI * (altitude * deg2rad).cos();
            }
        }
    } else {
        for r in 0..rows {
            for c in 0..cols {
                let sh_val = shadow[(r, c)];
                if azimuth > (360.0 - t) || azimuth <= (180.0 - t) {
                    Keast[(r, c)] = radI
                        * sh_val
                        * (altitude * deg2rad).cos()
                        * ((azimuth + t) * deg2rad).sin();
                }
                if azimuth > (90.0 - t) && azimuth <= (270.0 - t) {
                    Ksouth[(r, c)] = radI
                        * sh_val
                        * (altitude * deg2rad).cos()
                        * ((azimuth - 90.0 + t) * deg2rad).sin();
                }
                if azimuth > (180.0 - t) && azimuth <= (360.0 - t) {
                    Kwest[(r, c)] = radI
                        * sh_val
                        * (altitude * deg2rad).cos()
                        * ((azimuth - 180.0 + t) * deg2rad).sin();
                }
                if azimuth <= (90.0 - t) || azimuth > (270.0 - t) {
                    Knorth[(r, c)] = radI
                        * sh_val
                        * (altitude * deg2rad).cos()
                        * ((azimuth - 270.0 + t) * deg2rad).sin();
                }
            }
        }
    }

    // Isotropic diffuse/reflected radiation
    let ke_slice = as_slice_mut_checked(&mut Keast);
    let ks_slice = as_slice_mut_checked(&mut Ksouth);
    let kw_slice = as_slice_mut_checked(&mut Kwest);
    let kn_slice = as_slice_mut_checked(&mut Knorth);
    let fsh_slice = as_slice_checked(&F_sh);
    let svf_e_slice = as_slice_checked(&svfviktbuvegE);
    let svf_s_slice = as_slice_checked(&svfviktbuvegS);
    let svf_w_slice = as_slice_checked(&svfviktbuvegW);
    let svf_n_slice = as_slice_checked(&svfviktbuvegN);
    let kup_e_slice = as_slice_checked(&KupE);
    let kup_s_slice = as_slice_checked(&KupS);
    let kup_w_slice = as_slice_checked(&KupW);
    let kup_n_slice = as_slice_checked(&KupN);
    // `valid` is ArrayView2<u8>, not Array2<f32>, so the helpers above don't apply.
    let valid_slice = valid.as_ref().map(|v| v.as_slice().expect(VEG_CONTIG_INVARIANT));
    ke_slice
        .par_iter_mut()
        .zip(ks_slice.par_iter_mut())
        .zip(kw_slice.par_iter_mut())
        .zip(kn_slice.par_iter_mut())
        .enumerate()
        .for_each(|(idx, (((ke, ks), kw), kn))| {
            if let Some(ref vs) = valid_slice {
                if vs[idx] == 0 {
                    *ke = f32::NAN;
                    *ks = f32::NAN;
                    *kw = f32::NAN;
                    *kn = f32::NAN;
                    return;
                }
            }
            let fsh = fsh_slice[idx];
            let svf_e = svf_e_slice[idx];
            let svf_s = svf_s_slice[idx];
            let svf_w = svf_w_slice[idx];
            let svf_n = svf_n_slice[idx];
            let kup_e = kup_e_slice[idx];
            let kup_s = kup_s_slice[idx];
            let kup_w = kup_w_slice[idx];
            let kup_n = kup_n_slice[idx];
            let mix = radG * (1.0 - fsh) + radD * fsh;
            *ke += (radD * (1.0 - svf_e) + albedo * (svf_e * mix) + kup_e) * 0.5;
            *ks += (radD * (1.0 - svf_s) + albedo * (svf_s * mix) + kup_s) * 0.5;
            *kw += (radD * (1.0 - svf_w) + albedo * (svf_w * mix) + kup_w) * 0.5;
            *kn += (radD * (1.0 - svf_n) + albedo * (svf_n * mix) + kup_n) * 0.5;
        });

    KsideVegPureResult {
        keast: Keast,
        ksouth: Ksouth,
        kwest: Kwest,
        knorth: Knorth,
        kside_i: KsideI,
    }
}
