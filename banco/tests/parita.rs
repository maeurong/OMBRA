//! The parity bench: OMBRA's vendored timestep, extended by ADR 0004 to read
//! the two transmissivities as rasters, against upstream solweig's own
//! `compute_timestep_rust`, from the clone `prepara.sh` checks out at the pin
//! of `vendor/solweig/PROVENIENZA.toml`. Same input, bit for bit.
//!
//! The two sides share the Scenario — surface model, canopy, walls, ground,
//! weather, person — and nothing derived from it. Every upstream input that a
//! sky pass or a view-factor cache produces comes from upstream's own calls,
//! never from OMBRA's `PassataDelCielo`: a field copied across would carry an
//! error on OMBRA's side, a swapped n/e/s/w or a beam read as sky, into
//! upstream's inputs too, and the comparison would agree with it.
//!
//! Row 0 is the northernmost; the azimuth is degrees from north, clockwise.

use ndarray::{s, Array2, ArrayView2, Zip};
use ombra::derivazione::{Raster, StratoDiChioma};
use ombra::dominio::Data;
use ombra::motore::{
    self, HumanScalars, IngressiDelloScenario, Muri, PassataDelCielo, PassoDelloScenario,
    ProprietaDelSuolo, StatoTermico, WeatherScalars,
};
use ombra::sole::posizione;
use rayon::ThreadPool;
use rustalgos::api as monte;
use std::ops::Range;

const LATO: usize = 16;
const FORMA: (usize, usize) = (LATO, LATO);
const PASSO_M: f32 = 1.0;
const LATITUDINE: f64 = 43.07;
const LONGITUDINE: f64 = 12.55;
const FUSO: f64 = 1.0;
const GIORNO: Data = Data {
    anno: 2026,
    mese: 6,
    giorno: 21,
};
/// The column the east half starts at, in the third and in the fourth case.
const MEZZERIA: usize = LATO / 2;

const PERSONA: HumanScalars = HumanScalars {
    height: 1.1,
    abs_k: 0.7,
    abs_l: 0.95,
    is_standing: true,
};

// --- The Scenario, shared by the two sides -----------------------------------

struct Scenario {
    dsm: Raster,
    /// OMBRA's `costruite`: the height of the Edificio, 0 elsewhere.
    costruite: Raster,
    /// Upstream's `buildings`, 1 on the ground and 0 under the Edificio,
    /// written from the Edificio and not from `costruite`. Upstream's Python
    /// would derive it with `detect_building_mask` (2 m above the DEM), which
    /// a 6 m Edificio on flat ground satisfies cell for cell.
    edifici_di_monte: Raster,
    /// Canopy top and trunk zone, absolute, NaN where there is no tree: the
    /// form a `StratoDiChioma` carries, handed to both sides.
    chiome: Raster,
    zona_tronco: Raster,
    muri: Muri,
    suolo: ProprietaDelSuolo,
}

/// 16 × 16 cells at 1 m: an Edificio of 4 × 4 cells, 6 m tall, in the
/// north-west, and an Albero of 5 × 5 cells across the middle column in the
/// south, canopy top from 8 m at the centre down to 5 m at the rim, trunk zone
/// up to 2 m. In the afternoon both cast their shadow east, away from each
/// other, and the canopy's falls on both halves of the third case.
fn scenario() -> Scenario {
    let mut dsm = Raster::zeros(FORMA);
    let mut costruite = Raster::zeros(FORMA);
    let mut edifici_di_monte = Raster::ones(FORMA);
    for r in 2..6 {
        for c in 2..6 {
            dsm[[r, c]] = 6.0;
            costruite[[r, c]] = 6.0;
            edifici_di_monte[[r, c]] = 0.0;
        }
    }

    let mut chiome = Raster::from_elem(FORMA, f32::NAN);
    let mut zona_tronco = Raster::from_elem(FORMA, f32::NAN);
    let centro = (11_usize, 7_usize);
    for r in centro.0 - 2..=centro.0 + 2 {
        for c in centro.1 - 2..=centro.1 + 2 {
            let anello = r.abs_diff(centro.0).max(c.abs_diff(centro.1));
            chiome[[r, c]] = [8.0, 6.5, 5.0][anello];
            zona_tronco[[r, c]] = 2.0;
        }
    }

    let altezza = muri_alla_findwalls(&dsm);
    let orientamento_gradi = solweig_vendorato::wall_aspect::compute_wall_aspect_pure(
        altezza.view(),
        1.0 / PASSO_M,
        dsm.view(),
        None,
        None,
    )
    .expect("l'esposizione dei muri del banco");

    // Upstream's land-cover class 1, `pysrc/solweig/models/surface.py:1959`
    // at the pin: albedo, emissivity, TgK, Tstart, TmaxLST.
    let suolo = ProprietaDelSuolo {
        albedo: Raster::from_elem(FORMA, 0.18),
        emissivita: Raster::from_elem(FORMA, 0.95),
        tgk: Raster::from_elem(FORMA, 0.58),
        tstart: Raster::from_elem(FORMA, -9.78),
        tmaxlst: Raster::from_elem(FORMA, 15.0),
    };

    Scenario {
        dsm,
        costruite,
        edifici_di_monte,
        chiome,
        zona_tronco,
        muri: Muri {
            altezza,
            orientamento_gradi,
        },
        suolo,
    }
}

/// Upstream's `findwalls` (`pysrc/solweig/physics/wallalgorithms.py:36-56` at
/// the pin) with the `walllimit` of 1 m that `surface.py:1163` passes: the
/// height of the tallest of the four neighbours above the cell, zero below
/// the limit and on the border.
fn muri_alla_findwalls(dsm: &Raster) -> Raster {
    let mut muri = Raster::zeros(dsm.dim());
    for r in 1..dsm.nrows() - 1 {
        for c in 1..dsm.ncols() - 1 {
            let vicini = dsm[[r - 1, c]]
                .max(dsm[[r + 1, c]])
                .max(dsm[[r, c - 1]].max(dsm[[r, c + 1]]));
            let muro = vicini - dsm[[r, c]];
            muri[[r, c]] = if muro < 1.0 { 0.0 } else { muro };
        }
    }
    muri
}

/// Six Ore of 21 June, from 14 to 20, labelled at the end of each hour and
/// with the sun at its middle, as upstream's `Weather.compute_derived` does
/// (`pysrc/solweig/models/weather.py:150-151`). The sun sets near 19:50, so
/// the last Ora straddles it.
///
/// The weather is a made-up clear afternoon, the same on both sides: the
/// bench compares the step, not the weather.
fn le_sei_ore() -> Vec<WeatherScalars> {
    let sole =
        |ora: f64| posizione(GIORNO, ora, FUSO, LATITUDINE, LONGITUDINE).expect("il 21 giugno");
    let giornata: Vec<f64> = (0..1440)
        .map(|minuto| sole(f64::from(minuto) / 60.0).altezza_gradi)
        .collect();
    let altmax = giornata.iter().copied().fold(f64::NEG_INFINITY, f64::max) as f32;
    let alba = giornata
        .iter()
        .position(|&altezza| altezza > 0.0)
        .expect("un'alba il 21 giugno") as f32
        / 60.0;

    (15..=20)
        .map(|fine: u8| {
            let posizione = sole(f64::from(fine) - 0.5);
            let altezza = posizione.altezza_gradi as f32;
            let seno = altezza.to_radians().sin().max(0.0);
            let diretta = 800.0 * seno.sqrt();
            let diffusa = 120.0 * seno.sqrt();
            let globale = diretta * seno + diffusa;
            let ora = f32::from(fine - 15);
            WeatherScalars {
                sun_azimuth: posizione.azimut_gradi as f32,
                sun_altitude: altezza,
                sun_zenith: 90.0 - altezza,
                ta: 31.0 - 0.8 * ora,
                rh: 40.0 + 3.0 * ora,
                global_rad: globale,
                direct_rad: diretta,
                diffuse_rad: diffusa,
                altmax,
                clearness_index: if altezza > 0.0 { 0.8 } else { 0.0 },
                // `(hour + minute / 60) / 24` of the label,
                // `pysrc/solweig/computation.py:341`.
                dectime: f32::from(fine) / 24.0,
                snup: alba,
                rad_g0: globale,
                zen_deg: 90.0 - altezza,
                is_daytime: altezza > 0.0,
                jday: 172,
                patch_option: 2,
            }
        })
        .collect()
}

/// Upstream's initial state, `pysrc/solweig/models/state.py:61-83`, with
/// `timestep_dec` at the hour `timeseries.py:587-588` sets right after.
/// `timestep_dec` stays at this value for every Ora, on both sides.
fn stato_iniziale() -> StatoTermico {
    StatoTermico {
        firstdaytime: 1,
        timeadd: 0.0,
        timestep_dec: 1.0 / 24.0,
        tgmap1: Raster::zeros(FORMA),
        tgmap1_e: Raster::zeros(FORMA),
        tgmap1_s: Raster::zeros(FORMA),
        tgmap1_w: Raster::zeros(FORMA),
        tgmap1_n: Raster::zeros(FORMA),
        tgout1: Raster::zeros(FORMA),
    }
}

/// `svfalfa` as upstream's Python computes it,
/// `pysrc/solweig/components/svf_resolution.py:98-100`. Each side applies it
/// to its own `svf` and `svf_veg`.
fn svfalfa(svf: &Raster, svf_veg: &Raster) -> Raster {
    Zip::from(svf).and(svf_veg).map_collect(|&svf, &svf_veg| {
        let tmp = (svf + svf_veg - 1.0).clamp(0.0, 1.0);
        (1.0 - tmp).clamp(0.0, 1.0).sqrt().asin()
    })
}

/// One rayon pool of one thread, as OMBRA's Motore runs: the kernel's
/// reductions sum in f32 in an order the pool decides.
fn pool_a_un_thread() -> ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("un pool rayon da un thread")
}

// --- OMBRA's side ------------------------------------------------------------

/// What OMBRA's `PassoDelloScenario` borrows, owned.
struct LatoOmbra {
    strati: Vec<StratoDiChioma>,
    cielo: PassataDelCielo,
    svfalfa: Raster,
    verso_il_cielo: Raster,
}

impl LatoOmbra {
    /// `trasmissivita_strato` is the beam transmissivity of the one canopy
    /// layer, which OMBRA's sky pass also uses for `svfbuveg`;
    /// `verso_il_cielo` is the sky transmissivity raster of ADR 0004.
    fn nuovo(sc: &Scenario, trasmissivita_strato: f32, verso_il_cielo: f32) -> Self {
        let strati = vec![StratoDiChioma {
            nome: "banco",
            trasmissivita: trasmissivita_strato,
            chiome: sc.chiome.clone(),
            zona_tronco: sc.zona_tronco.clone(),
        }];
        let cielo = motore::cielo(&sc.dsm, f64::from(PASSO_M), &strati, true)
            .expect("la passata del cielo di OMBRA");
        let svfalfa = svfalfa(
            &cielo.sky_view_factor.senza_chiome,
            &cielo.direzionali.svf_veg,
        );
        Self {
            strati,
            cielo,
            svfalfa,
            verso_il_cielo: Raster::from_elem(FORMA, verso_il_cielo),
        }
    }

    fn passo<'a>(&'a self, sc: &'a Scenario, anisotropo: bool) -> PassoDelloScenario<'a> {
        PassoDelloScenario::nuovo(IngressiDelloScenario {
            modello_di_superficie: &sc.dsm,
            passo_m: f64::from(PASSO_M),
            strati: &self.strati,
            costruite: &sc.costruite,
            muri: &sc.muri,
            cielo: &self.cielo,
            svfalfa: &self.svfalfa,
            suolo: &sc.suolo,
            trasmissivita_verso_il_cielo: &self.verso_il_cielo,
            persona: PERSONA,
            anisotropo,
        })
        .expect("il passo di OMBRA")
    }
}

// --- Upstream's side ---------------------------------------------------------

/// What upstream's step reads that no Ora changes, from upstream's own calls.
struct LatoMonte {
    svf: monte::SvfIntermediate,
    svfalfa: Raster,
    gvf: monte::GvfGeometryCache,
    config: monte::ConfigScalars,
    cespugli: Raster,
    valida: Array2<u8>,
}

/// Upstream's relief for the ray march: the highest of DSM and canopy above
/// the lowest DSM cell, NaN ignored. `SurfaceData.max_height`
/// (`pysrc/solweig/models/surface.py:1432-1460`) for the step and
/// `_max_shadow_height` (`:246-270`) for the sky pass agree when the canopy
/// is in use.
fn rilievo_di_monte(dsm: &Raster, cdsm: &Raster) -> f32 {
    let massimo = |campo: &Raster| campo.iter().copied().fold(f32::NAN, f32::max);
    let minimo = dsm.iter().copied().fold(f32::NAN, f32::min);
    massimo(dsm).max(massimo(cdsm)) - minimo
}

impl LatoMonte {
    fn nuovo(sc: &Scenario, pool: &ThreadPool, anisotropo: bool) -> Self {
        let max_height = rilievo_di_monte(&sc.dsm, &sc.chiome);
        // The arguments of `SurfaceData.compute_svf`, `surface.py:1406-1416`:
        // 153 patches, 3 degrees, and no cap on the shadow, as the pyfunction
        // `calculate_svf` turns its `None` into 0 (`rust/src/skyview.rs:958`).
        let svf = pool
            .install(|| {
                monte::calculate_svf_inner(
                    sc.dsm.clone(),
                    sc.chiome.clone(),
                    sc.zona_tronco.clone(),
                    PASSO_M,
                    true,
                    max_height,
                    2,
                    Some(3.0),
                    None,
                    None,
                    0.0,
                )
            })
            .expect("la passata del cielo di monte");
        let svfalfa = svfalfa(&svf.svf, &svf.svf_veg);
        // `first_ht` and `second_ht` as the pyfunction `precompute_gvf_cache`
        // derives them, `rust/src/pipeline.rs:415-416`; the wall albedo of
        // `computation.py:326`.
        let gvf = pool.install(|| {
            monte::precompute_gvf_geometry_cpu(
                sc.edifici_di_monte.view(),
                sc.muri.orientamento_gradi.view(),
                sc.muri.altezza.view(),
                sc.suolo.albedo.view(),
                PASSO_M,
                PERSONA.height.round().max(1.0),
                PERSONA.height * 20.0,
                0.20,
            )
        });
        // `computation.py:323-327` for the wall material and `:410` for the
        // shadow reach.
        let config = monte::ConfigScalars {
            pixel_size: PASSO_M,
            max_height,
            albedo_wall: 0.20,
            emis_wall: 0.90,
            tgk_wall: 0.37,
            tstart_wall: -3.41,
            tmaxlst_wall: 15.0,
            use_veg: true,
            has_walls: true,
            conifer: false,
            use_anisotropic: anisotropo,
            max_shadow_distance_m: 1000.0,
        };
        Self {
            svf,
            svfalfa,
            gvf,
            config,
            // `computation.py:283-285`: zeros from the buffer pool.
            cespugli: Raster::zeros(FORMA),
            valida: Array2::ones(FORMA),
        }
    }

    /// One Ora of upstream's step with the scalar transmissivity `psi`.
    fn ora(
        &self,
        sc: &Scenario,
        pool: &ThreadPool,
        meteo: &WeatherScalars,
        psi: f32,
        stato: &StatoTermico,
    ) -> monte::TimestepResultRaw {
        // Without the anisotropic sky upstream's Python hands no shadow
        // matrices (`computation.py:497-499`).
        let aniso = self.config.use_anisotropic;
        // `adjust_svfbuveg_with_psi`, `svf_resolution.py:140-142`, with the
        // `psi` of the step, as `computation.py:318` passes it.
        let svfbuveg = Zip::from(&self.svf.svf)
            .and(&self.svf.svf_veg)
            .map_collect(|&svf, &svf_veg| (svf - (1.0 - svf_veg) * (1.0 - psi)).clamp(0.0, 1.0));
        let meteo = monte::WeatherScalars {
            sun_azimuth: meteo.sun_azimuth,
            sun_altitude: meteo.sun_altitude,
            sun_zenith: meteo.sun_zenith,
            ta: meteo.ta,
            rh: meteo.rh,
            global_rad: meteo.global_rad,
            direct_rad: meteo.direct_rad,
            diffuse_rad: meteo.diffuse_rad,
            altmax: meteo.altmax,
            clearness_index: meteo.clearness_index,
            dectime: meteo.dectime,
            snup: meteo.snup,
            rad_g0: meteo.rad_g0,
            zen_deg: meteo.zen_deg,
            psi,
            is_daytime: meteo.is_daytime,
            jday: meteo.jday,
            // The option of the shadow matrices with the anisotropic sky
            // (`computation.py:519`), 0 without (`:386`).
            patch_option: if aniso { 2 } else { 0 },
        };
        let persona = monte::HumanScalars {
            height: PERSONA.height,
            abs_k: PERSONA.abs_k,
            abs_l: PERSONA.abs_l,
            is_standing: PERSONA.is_standing,
        };
        let svf = &self.svf;
        let ingressi = monte::TimestepInputs {
            weather: &meteo,
            human: &persona,
            config: &self.config,
            gvf_cache: Some(&self.gvf),
            surface: monte::SurfaceViews {
                dsm: sc.dsm.view(),
                cdsm: Some(sc.chiome.view()),
                tdsm: Some(sc.zona_tronco.view()),
                bush: Some(self.cespugli.view()),
                wall_ht: Some(sc.muri.altezza.view()),
                wall_asp: Some(sc.muri.orientamento_gradi.view()),
            },
            svf: monte::SvfViews {
                svf: svf.svf.view(),
                svf_n: svf.svf_n.view(),
                svf_e: svf.svf_e.view(),
                svf_s: svf.svf_s.view(),
                svf_w: svf.svf_w.view(),
                svf_veg: svf.svf_veg.view(),
                svf_veg_n: svf.svf_veg_n.view(),
                svf_veg_e: svf.svf_veg_e.view(),
                svf_veg_s: svf.svf_veg_s.view(),
                svf_veg_w: svf.svf_veg_w.view(),
                svf_aveg: svf.svf_veg_blocks_bldg_sh.view(),
                svf_aveg_n: svf.svf_veg_blocks_bldg_sh_n.view(),
                svf_aveg_e: svf.svf_veg_blocks_bldg_sh_e.view(),
                svf_aveg_s: svf.svf_veg_blocks_bldg_sh_s.view(),
                svf_aveg_w: svf.svf_veg_blocks_bldg_sh_w.view(),
                svfbuveg: svfbuveg.view(),
                svfalfa: self.svfalfa.view(),
            },
            properties: monte::PropertyViews {
                alb_grid: sc.suolo.albedo.view(),
                emis_grid: sc.suolo.emissivita.view(),
                tgk_grid: sc.suolo.tgk.view(),
                tstart_grid: sc.suolo.tstart.view(),
                tmaxlst_grid: sc.suolo.tmaxlst.view(),
            },
            buildings: sc.edifici_di_monte.view(),
            lc_grid: None,
            shmat: aniso.then(|| svf.bldg_sh_matrix.view()),
            vegshmat: aniso.then(|| svf.veg_sh_matrix.view()),
            vbshmat: aniso.then(|| svf.veg_blocks_bldg_sh_matrix.view()),
            state: monte::ThermalStateViews {
                firstdaytime: stato.firstdaytime,
                timeadd: stato.timeadd,
                timestep_dec: stato.timestep_dec,
                tgmap1: stato.tgmap1.view(),
                tgmap1_e: stato.tgmap1_e.view(),
                tgmap1_s: stato.tgmap1_s.view(),
                tgmap1_w: stato.tgmap1_w.view(),
                tgmap1_n: stato.tgmap1_n.view(),
                tgout1: stato.tgout1.view(),
            },
            ground_scheme: None,
            valid_mask: self.valida.view(),
            output_mask: monte::OUT_ALL,
        };
        pool.install(|| monte::compute_timestep_rust(&ingressi))
            .expect("un'Ora del passo di monte")
    }
}

/// What upstream's Python does to the state after each step,
/// `pysrc/solweig/computation.py:723-727`: after a day Ora the next is not the
/// first of the day, so the step reads the history it carries; after a night
/// Ora it is, and the delay starts again. Applied on both sides.
fn dopo_l_ora(mut stato: StatoTermico, meteo: &WeatherScalars) -> StatoTermico {
    if meteo.is_daytime {
        stato.firstdaytime = 0;
    } else {
        stato.firstdaytime = 1;
        stato.timeadd = 0.0;
    }
    stato
}

/// Upstream's output carried into its next Ora, `firstdaytime` and
/// `timestep_dec` unchanged.
fn stato_di_monte(precedente: &StatoTermico, esito: &monte::TimestepResultRaw) -> StatoTermico {
    StatoTermico {
        firstdaytime: precedente.firstdaytime,
        timeadd: esito.timeadd,
        timestep_dec: precedente.timestep_dec,
        tgmap1: esito.tgmap1.clone(),
        tgmap1_e: esito.tgmap1_e.clone(),
        tgmap1_s: esito.tgmap1_s.clone(),
        tgmap1_w: esito.tgmap1_w.clone(),
        tgmap1_n: esito.tgmap1_n.clone(),
        tgout1: esito.tgout1.clone(),
    }
}

/// The optional outputs of upstream's step, which `OUT_ALL` asks for: a
/// missing one would compare nothing.
struct UsciteDiMonte<'a> {
    shadow: &'a Raster,
    kdown: &'a Raster,
    kup: &'a Raster,
    ldown: &'a Raster,
    lup: &'a Raster,
}

fn uscite(esito: &monte::TimestepResultRaw) -> UsciteDiMonte<'_> {
    fn chiesta<'a>(campo: &str, valore: Option<&'a Raster>) -> &'a Raster {
        valore.unwrap_or_else(|| panic!("il passo di monte non ha restituito {campo}"))
    }
    UsciteDiMonte {
        shadow: chiesta("shadow", esito.shadow.as_ref()),
        kdown: chiesta("kdown", esito.kdown.as_ref()),
        kup: chiesta("kup", esito.kup.as_ref()),
        ldown: chiesta("ldown", esito.ldown.as_ref()),
        lup: chiesta("lup", esito.lup.as_ref()),
    }
}

// --- The comparison ----------------------------------------------------------

/// Cell by cell on `to_bits`, so a NaN equals the same NaN and 0.0 differs
/// from -0.0: equal values are not enough, the bits are the claim.
fn stessi_bit(nome: &str, ombra: ArrayView2<f32>, monte: ArrayView2<f32>) {
    assert_eq!(ombra.dim(), monte.dim(), "{nome}: forme diverse");
    let diverse: Vec<_> = ombra
        .indexed_iter()
        .filter(|&(cella, o)| o.to_bits() != monte[cella].to_bits())
        .collect();
    if let Some(&(cella, o)) = diverse.first() {
        panic!(
            "{nome}: {} celle su {} con bit diversi; la prima {cella:?}, OMBRA {o} contro monte {}",
            diverse.len(),
            ombra.len(),
            monte[cella]
        );
    }
}

/// The cells of the canopy, where the sky transmissivity weighs.
fn sotto_la_chioma(sc: &Scenario) -> impl Iterator<Item = (usize, usize)> + '_ {
    sc.chiome
        .indexed_iter()
        .filter(|(_, cima)| cima.is_finite())
        .map(|(cella, _)| cella)
}

#[test]
fn con_trasmissivita_costanti_il_passo_e_monte_bit_per_bit() {
    let sc = scenario();
    let pool = pool_a_un_thread();
    let lato_ombra = LatoOmbra::nuovo(&sc, 0.03, 0.03);
    let raggio = Raster::from_elem(FORMA, 0.03);

    for anisotropo in [true, false] {
        let cielo = if anisotropo { "anisotropo" } else { "isotropo" };
        let passo = lato_ombra.passo(&sc, anisotropo);
        let lato_monte = LatoMonte::nuovo(&sc, &pool, anisotropo);
        let mut stato_ombra = stato_iniziale();
        let mut stato_monte = stato_iniziale();
        for (indice, meteo) in le_sei_ore().iter().enumerate() {
            let ora = indice + 1;
            let o = passo
                .ora(meteo, &raggio, &stato_ombra)
                .expect("un'Ora del passo di OMBRA");
            let esito = lato_monte.ora(&sc, &pool, meteo, 0.03, &stato_monte);
            let m = uscite(&esito);

            if ora == 1 {
                // Not a comparison of two flat fields: the first Ora has cells
                // in the sun and in the shade, and a Tmrt that varies.
                assert!(m.shadow.iter().any(|&v| v == 0.0) && m.shadow.iter().any(|&v| v == 1.0));
                let (minimo, massimo) = esito
                    .tmrt
                    .iter()
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &v| {
                        (a.min(v), b.max(v))
                    });
                assert!(
                    massimo - minimo > 1.0,
                    "cielo {cielo}: Tmrt quasi uniforme: {minimo}..{massimo}"
                );
            }

            for (nome, ombra, monte) in [
                ("tmrt", &o.temperatura_media_radiante, &esito.tmrt),
                ("shadow", &o.illuminata, m.shadow),
                ("kdown", &o.k_giu, m.kdown),
                ("kup", &o.k_su, m.kup),
                ("ldown", &o.l_giu, m.ldown),
                ("lup", &o.l_su, m.lup),
            ] {
                stessi_bit(
                    &format!("cielo {cielo}, Ora {ora}, {nome}"),
                    ombra.view(),
                    monte.view(),
                );
            }

            stato_monte = dopo_l_ora(stato_di_monte(&stato_monte, &esito), meteo);
            stato_ombra = dopo_l_ora(o.stato, meteo);
        }

        assert_eq!(
            stato_ombra.timeadd.to_bits(),
            stato_monte.timeadd.to_bits(),
            "cielo {cielo}: timeadd dopo la sesta Ora: OMBRA {} contro monte {}",
            stato_ombra.timeadd,
            stato_monte.timeadd
        );
        for (nome, ombra, monte) in [
            ("tgmap1", &stato_ombra.tgmap1, &stato_monte.tgmap1),
            ("tgmap1_e", &stato_ombra.tgmap1_e, &stato_monte.tgmap1_e),
            ("tgmap1_s", &stato_ombra.tgmap1_s, &stato_monte.tgmap1_s),
            ("tgmap1_w", &stato_ombra.tgmap1_w, &stato_monte.tgmap1_w),
            ("tgmap1_n", &stato_ombra.tgmap1_n, &stato_monte.tgmap1_n),
            ("tgout1", &stato_ombra.tgout1, &stato_monte.tgout1),
        ] {
            stessi_bit(
                &format!("cielo {cielo}, dopo la sesta Ora, {nome}"),
                ombra.view(),
                monte.view(),
            );
        }
    }
}

#[test]
fn raggio_e_cielo_non_si_scambiano() {
    let sc = scenario();
    let pool = pool_a_un_thread();
    // The layer carries the beam value, which OMBRA's sky pass also puts into
    // `svfbuveg`: upstream with `psi = 0.5` does the same, so the two
    // `svfbuveg` agree and a difference in `kdown` can only come from the
    // sky transmissivity the step reads patch by patch.
    let lato_ombra = LatoOmbra::nuovo(&sc, 0.5, 0.03);
    let passo = lato_ombra.passo(&sc, true);
    let lato_monte = LatoMonte::nuovo(&sc, &pool, true);
    let meteo = &le_sei_ore()[0];

    let o = passo
        .ora(meteo, &Raster::from_elem(FORMA, 0.5), &stato_iniziale())
        .expect("un'Ora del passo di OMBRA");
    let esito = lato_monte.ora(&sc, &pool, meteo, 0.5, &stato_iniziale());
    let m = uscite(&esito);

    assert!(
        m.shadow.iter().any(|&v| v > 0.0 && v < 1.0),
        "nessuna cella all'ombra della sola chioma: il raggio non attraversa niente"
    );
    stessi_bit("shadow, raggio a 0,5", o.illuminata.view(), m.shadow.view());

    let diverse = sotto_la_chioma(&sc)
        .filter(|&cella| o.k_giu[cella].to_bits() != m.kdown[cella].to_bits())
        .count();
    assert!(
        diverse > 0,
        "kdown sotto la chioma uguale a monte con psi = 0,5: il cielo a 0,03 non è stato letto"
    );
}

#[test]
fn la_trasmissivita_si_legge_cella_per_cella() {
    let sc = scenario();
    let pool = pool_a_un_thread();
    let lato_ombra = LatoOmbra::nuovo(&sc, 0.03, 0.03);
    let passo = lato_ombra.passo(&sc, true);
    let lato_monte = LatoMonte::nuovo(&sc, &pool, true);
    let meteo = &le_sei_ore()[0];

    let mut raggio = Raster::from_elem(FORMA, 0.03);
    raggio.slice_mut(s![.., MEZZERIA..]).fill(0.5);
    let o = passo
        .ora(meteo, &raggio, &stato_iniziale())
        .expect("un'Ora del passo di OMBRA");
    let monte_003 = lato_monte.ora(&sc, &pool, meteo, 0.03, &stato_iniziale());
    let monte_05 = lato_monte.ora(&sc, &pool, meteo, 0.5, &stato_iniziale());
    let (con_003, con_05) = (uscite(&monte_003).shadow, uscite(&monte_05).shadow);

    // Each half must have a cell the beam transmissivity changes, or that
    // half tests nothing.
    for (meta, colonne) in [("ovest", s![.., ..MEZZERIA]), ("est", s![.., MEZZERIA..])] {
        assert!(
            Zip::from(con_003.slice(colonne))
                .and(con_05.slice(colonne))
                .fold(false, |trovata, a, b| trovata || a != b),
            "nella metà {meta} nessuna cella dipende dalla trasmissività lungo il raggio"
        );
    }

    let illuminata = &o.illuminata;
    stessi_bit(
        "shadow, metà ovest a 0,03",
        illuminata.slice(s![.., ..MEZZERIA]),
        con_003.slice(s![.., ..MEZZERIA]),
    );
    stessi_bit(
        "shadow, metà est a 0,5",
        illuminata.slice(s![.., MEZZERIA..]),
        con_05.slice(s![.., MEZZERIA..]),
    );
}

/// 0.03 on the west half, 0.5 on the east.
fn a_meta() -> Raster {
    let mut raster = Raster::from_elem(FORMA, 0.03);
    raster.slice_mut(s![.., MEZZERIA..]).fill(0.5);
    raster
}

/// The west half of `ombra` against `con_003`, the east half against `con_05`.
fn per_meta(nome: &str, ombra: &Raster, con_003: &Raster, con_05: &Raster) {
    stessi_bit(
        &format!("{nome}, metà ovest a 0,03"),
        ombra.slice(s![.., ..MEZZERIA]),
        con_003.slice(s![.., ..MEZZERIA]),
    );
    stessi_bit(
        &format!("{nome}, metà est a 0,5"),
        ombra.slice(s![.., MEZZERIA..]),
        con_05.slice(s![.., MEZZERIA..]),
    );
}

/// Whether some cell of the half `colonne` differs between the two fields.
fn qualche_cella_diversa(colonne: Range<usize>, a: &Raster, b: &Raster) -> bool {
    Zip::from(a.slice(s![.., colonne.clone()]))
        .and(b.slice(s![.., colonne]))
        .fold(false, |trovata, x, y| trovata || x.to_bits() != y.to_bits())
}

#[test]
fn le_due_trasmissivita_si_leggono_cella_per_cella_in_entrambi_i_cieli() {
    let sc = scenario();
    let pool = pool_a_un_thread();
    let meta = a_meta();
    // OMBRA's `svfbuveg` takes the one transmissivity of the layer, upstream's
    // the scalar `psi`: each half gets the field of a sky pass at its own
    // value. The field is `svf - (1 - svf_veg) * (1 - psi)` cell by cell, and
    // `svf`, `svf_veg` do not depend on the transmissivity, so the splice is
    // exactly what a per-cell transmissivity gives.
    let mut lato_ombra = LatoOmbra::nuovo(&sc, 0.03, 0.03);
    lato_ombra.verso_il_cielo = meta.clone();
    let a_05 = LatoOmbra::nuovo(&sc, 0.5, 0.5);
    lato_ombra
        .cielo
        .sky_view_factor
        .con_le_chiome
        .slice_mut(s![.., MEZZERIA..])
        .assign(
            &a_05
                .cielo
                .sky_view_factor
                .con_le_chiome
                .slice(s![.., MEZZERIA..]),
        );

    for anisotropo in [true, false] {
        let cielo = if anisotropo { "anisotropo" } else { "isotropo" };
        let passo = lato_ombra.passo(&sc, anisotropo);
        let lato_monte = LatoMonte::nuovo(&sc, &pool, anisotropo);
        let mut stato_ombra = stato_iniziale();
        let (mut stato_003, mut stato_05) = (stato_iniziale(), stato_iniziale());
        let mut ore_senza_raggio = 0;

        for (indice, meteo) in le_sei_ore().iter().enumerate() {
            let ora = indice + 1;
            let o = passo
                .ora(meteo, &meta, &stato_ombra)
                .expect("un'Ora del passo di OMBRA");
            let esito_003 = lato_monte.ora(&sc, &pool, meteo, 0.03, &stato_003);
            let esito_05 = lato_monte.ora(&sc, &pool, meteo, 0.5, &stato_05);
            let (m_003, m_05) = (uscite(&esito_003), uscite(&esito_05));

            // Shadow and Kdown are computed cell by cell from that cell's
            // transmissivities: exact at every cell and every Ora.
            per_meta(
                &format!("cielo {cielo}, Ora {ora}, shadow"),
                &o.illuminata,
                m_003.shadow,
                m_05.shadow,
            );
            per_meta(
                &format!("cielo {cielo}, Ora {ora}, kdown"),
                &o.k_giu,
                m_003.kdown,
                m_05.kdown,
            );
            if ora == 1 {
                for (meta_nome, colonne) in [("ovest", 0..MEZZERIA), ("est", MEZZERIA..LATO)] {
                    assert!(
                        qualche_cella_diversa(colonne, m_003.kdown, m_05.kdown),
                        "cielo {cielo}: nella metà {meta_nome} kdown non dipende dalla \
                         trasmissività, e quella metà non prova niente"
                    );
                }
            }

            // Tmrt, Kup, Lup and the anisotropic Ldown also read the ground
            // around a cell, through the ground view factor, as far as
            // `second_ht` = 22 cells: the whole 16-cell domain. Measured with
            // the beam split too, at Ore 1-5 they differ from upstream at
            // either psi in up to every column of the half, so no distance
            // from the split column makes them exact here. They enter the
            // ground view factor only through the shadow: in an Ora whose
            // shadow does not depend on psi (the last, with the sun at 2.6
            // degrees, below the step's 3-degree shadow floor) they were
            // bit-equal on all 256 cells, in both skies.
            if m_003.shadow == m_05.shadow {
                ore_senza_raggio += 1;
                for (nome, ombra, con_003, con_05) in [
                    (
                        "tmrt",
                        &o.temperatura_media_radiante,
                        &esito_003.tmrt,
                        &esito_05.tmrt,
                    ),
                    ("kup", &o.k_su, m_003.kup, m_05.kup),
                    ("ldown", &o.l_giu, m_003.ldown, m_05.ldown),
                    ("lup", &o.l_su, m_003.lup, m_05.lup),
                ] {
                    per_meta(
                        &format!("cielo {cielo}, Ora {ora} senza raggio, {nome}"),
                        ombra,
                        con_003,
                        con_05,
                    );
                }
                for (meta_nome, colonne) in [("ovest", 0..MEZZERIA), ("est", MEZZERIA..LATO)] {
                    assert!(
                        qualche_cella_diversa(colonne, &esito_003.tmrt, &esito_05.tmrt),
                        "cielo {cielo}, Ora {ora}: nella metà {meta_nome} Tmrt non dipende \
                         dalla trasmissività"
                    );
                }
            }

            // `firstdaytime` stays 1, unlike test 1: carried as upstream does,
            // the history of Ore 1-5, not exact per half, enters Ora 6, and
            // Tmrt there was measured off in 97 of the 128 western cells.
            stato_ombra = o.stato;
            stato_003 = stato_di_monte(&stato_003, &esito_003);
            stato_05 = stato_di_monte(&stato_05, &esito_05);
        }
        assert!(
            ore_senza_raggio > 0,
            "cielo {cielo}: nessuna Ora con l'ombra indipendente dalla trasmissività, \
             Tmrt non è stata confrontata"
        );
    }
}
