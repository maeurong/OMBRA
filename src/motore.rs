//! Where the program meets the vendored kernel: shadows, the sky pass and the
//! fused timestep.
//!
//! One other module names the copy in `vendor/solweig/`: `src/comfort.rs`,
//! for the UTCI of one point. Everything else asks this module and gets
//! Rasters back. The step's own scalar inputs, [`WeatherScalars`] and
//! [`HumanScalars`], are upstream's types re-exported as they are, so a field
//! added or removed upstream shows up at every caller.

use crate::derivazione::{ChiomeRiunite, Raster, StratoDiChioma};
use crate::sole::PosizioneSolare;
use ndarray::{Array2, Array3, Zip};
use serde::Deserialize;
use solweig_vendorato::api::{
    compute_timestep_rust, precompute_gvf_geometry_cpu, ConfigScalars, GvfGeometryCache,
    PropertyViews, SurfaceViews, SvfIntermediate, SvfViews, ThermalStateViews, TimestepInputs,
    OUT_ALL,
};
use std::sync::OnceLock;

pub use solweig_vendorato::api::{HumanScalars, WeatherScalars};

/// Which copy of the kernel produced a result. The Giornale cites it.
///
/// Deserialised straight from `PROVENIENZA.toml`: the keys the file carries and
/// this struct does not are ignored, so a second struct to map across would be
/// two names for one thing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersioneMotore {
    pub commit: String,
    pub data_presa: String,
}

/// The provenance of the vendored copy, read at compile time from the file the
/// vendoring wrote. Reading it at run time would make the answer depend on the
/// working directory, and the answer is a property of the binary.
const PROVENIENZA: &str = include_str!("../vendor/solweig/PROVENIENZA.toml");

pub fn versione() -> VersioneMotore {
    toml::from_str(PROVENIENZA).expect("PROVENIENZA.toml della copia vendorata illeggibile")
}

/// Below this solar elevation the Motore calls the whole domain shaded.
///
/// Not the horizon: at 2 degrees the beam crosses about twenty-five air masses,
/// the direct normal irradiance left is a few W/m2, and the shortwave a comfort
/// model cares about is already carried by the diffuse. The ray march, meanwhile,
/// gets more expensive the lower the sun: at a grazing elevation it walks the
/// whole domain from every cell, and it does it to decide the shading of a beam
/// that is not there.
pub const ALTEZZA_MINIMA_GRADI: f64 = 2.0;

/// The lit fraction of every cell of the surface model: 1 in the sun, 0 in the
/// shadow of a building or of the terrain, and the transmissivity of a canopy
/// under a tree.
///
/// Buildings and terrain are opaque, so their shadow is 0 and nothing a canopy
/// does can lighten it. A canopy is not: it lets [`StratoDiChioma::trasmissivita`]
/// of the beam through, and two canopies over the same cell let through the
/// product of the two. Which is why the layers are marched one at a time — the
/// kernel says whether a cell is shaded by vegetation, never by which
/// vegetation.
///
/// At or below [`ALTEZZA_MINIMA_GRADI`] everything comes back in shadow. That is
/// ours and not the kernel's: with the sun that low the kernel's ray meets no
/// obstruction worth the name and it reports every cell lit, which is
/// geometrically true and says nothing about a place at night. Where the
/// shortwave goes to zero the lit fraction must go with it.
pub fn ombre(
    modello_di_superficie: &Raster,
    passo_m: f64,
    sole: PosizioneSolare,
    strati: &[StratoDiChioma],
) -> Raster {
    if sole.altezza_gradi <= ALTEZZA_MINIMA_GRADI {
        return Raster::zeros(modello_di_superficie.dim());
    }

    let rilievo = rilievo(modello_di_superficie, strati);

    // The kernel treats vegetation as present only when canopy, trunk zone *and*
    // bushes are all there: with one of the three missing it computes a canopy
    // shadow and then skips the pass that normalises it, and the array that
    // comes back means something else. OMBRA has no Cespuglio in its
    // vocabulary, so the third raster is zeros — the kernel calls a bush a bush
    // above one metre, and zero is below it everywhere.
    let cespugli: Raster = Array2::zeros(modello_di_superficie.dim());

    let marcia = |strato: Option<&StratoDiChioma>| {
        pool_a_un_thread().install(|| {
            solweig_vendorato::shadowing::calculate_shadows_rust(
                sole.azimut_gradi as f32,
                sole.altezza_gradi as f32,
                passo_m as f32,
                rilievo,
                modello_di_superficie.view(),
                strato.map(|s| s.chiome.view()),
                strato.map(|s| s.zona_tronco.view()),
                strato.map(|_| cespugli.view()),
                // Muri, esposizioni e i due schemi: servono alla temperatura
                // delle pareti, che questa versione non calcola.
                None,
                None,
                None,
                None,
                false, // need_full_wall_outputs
                // No floor under the sun elevation: the guard above has already
                // ended the day. No cap on the shadow either, because the march
                // stops when it walks off the domain and the domain is what we were
                // asked about.
                //
                // Which leaves the kernel with `rilievo / tan(0)` for its index
                // bound: infinite when there is relief, and 0/0 when the domain is
                // flat. The march still ends, on `rilievo >= dz` and on walking off
                // the domain — but the flat case ends only because `f32::min` hands
                // back the operand that is not NaN, so the infinite cap survives the
                // NaN and the cast to a pixel count saturates instead of trapping.
                // That is a property of `f32::min`, not a decision anybody took: if
                // a future release of the kernel or of the standard library
                // propagated the NaN instead, the bound would silently become zero
                // and every shadow would disappear. `un_dominio_piatto_col_sole_alto`
                // in `tests/motore.rs` is what notices.
                0.0,
                f32::INFINITY,
            )
        })
    };

    let Some((primo, resto)) = strati.split_first() else {
        return marcia(None).bldg_sh;
    };
    let esito = marcia(Some(primo));
    let mut illuminata = esito.bldg_sh;
    illuminata *= &attenuazione(&esito.veg_sh, primo.trasmissivita);
    for strato in resto {
        illuminata *= &attenuazione(&marcia(Some(strato)).veg_sh, strato.trasmissivita);
    }
    illuminata
}

/// The `max_local_dsm_ht` every march of the kernel climbs: the relief, not the
/// maximum, with the canopies included.
///
/// The relief is the height the ray has to climb before it is above every
/// obstacle. Handing over the maximum marches further than needed on any
/// terrain that does not sit at zero, and on terrain below the datum it is
/// negative and no shadow is cast at all.
///
/// Every obstacle includes the canopies, which stand above the surface model
/// and are not in it. Leaving them out is not a march cut short by a little:
/// over flat ground the relief is zero, the ray stops at the first step, and no
/// canopy casts anything at all. `una_chioma_lascia_passare_la_sua_
/// trasmissivita_e_non_spegne_la_cella` in `tests/motore.rs` is what notices.
fn rilievo(modello_di_superficie: &Raster, strati: &[StratoDiChioma]) -> f32 {
    let (minimo, massimo) = modello_di_superficie.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(minimo, massimo), &quota| (minimo.min(quota), massimo.max(quota)),
    );
    let cima_delle_chiome = strati
        .iter()
        .flat_map(|strato| strato.chiome.iter())
        .copied()
        .filter(|quota| quota.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    massimo.max(cima_delle_chiome) - minimo
}

/// How much of the sky hemisphere a cell can see, with and without the canopies.
///
/// Two fields and not one because they answer two questions, and a mitigation
/// study needs both: masonry is permanent and a tree is a decision.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyViewFactor {
    /// What buildings and terrain leave. 1 on open flat ground. `svf` upstream.
    pub senza_chiome: Raster,
    /// The same, less the part of the sky the canopies cover and do not let
    /// through. Sky covered by one layer lets through that layer's
    /// transmissivity; sky covered by two lets through the product of the two.
    /// `svfbuveg` upstream.
    pub con_le_chiome: Raster,
}

/// The sky is divided into 153 patches: option 2 of the kernel, and the number
/// SOLWEIG's own callers pass.
///
/// A finer division (option 3 or 4) costs a march per patch and buys a
/// resolution the rest of the chain does not use.
const PATCH_DEL_CIELO: u8 = 2;

/// The sky view factor of every cell of the surface model: [`cielo`] without
/// the shadow matrices, and with only the two published fields kept.
///
/// # Errors
///
/// As [`cielo`].
///
/// # Panics
///
/// As [`cielo`].
pub fn sky_view_factor(
    modello_di_superficie: &Raster,
    passo_m: f64,
    strati: &[StratoDiChioma],
) -> Result<SkyViewFactor, String> {
    cielo(modello_di_superficie, passo_m, strati, false).map(|c| c.sky_view_factor)
}

/// What the sky pass yields beyond the two published Sky view factor fields.
#[derive(Debug, Clone)]
pub struct PassataDelCielo {
    pub sky_view_factor: SkyViewFactor,
    /// The kernel's directional fields, named as in upstream `SvfViews`
    /// (svf_n/e/s/w, svf_veg + n/e/s/w, svf_aveg + n/e/s/w). `svfbuveg` is
    /// `sky_view_factor.con_le_chiome` and is not repeated.
    pub direzionali: Direzionali,
    /// The three bit-packed shadow matrices, (rows, cols, 20); `None` unless asked for.
    pub matrici: Option<MatriciDOmbra>,
    /// Per cell: the opaque part the canopies take off the sky (what
    /// `detrazione` gives for one layer, the three shares by difference for
    /// two), and the vault the Chiome riunite cover, 1 − svf_veg.
    pub detrazione: Raster,
    pub volta_coperta: Raster,
}

/// The directional sky view factors of one pass, one Raster per field of
/// upstream's `SvfViews`. The `svf_aveg` fields are the kernel's
/// `svf_veg_blocks_bldg_sh`: the same quantity under two names.
#[derive(Debug, Clone)]
pub struct Direzionali {
    pub svf_n: Raster,
    pub svf_e: Raster,
    pub svf_s: Raster,
    pub svf_w: Raster,
    pub svf_veg: Raster,
    pub svf_veg_n: Raster,
    pub svf_veg_e: Raster,
    pub svf_veg_s: Raster,
    pub svf_veg_w: Raster,
    pub svf_aveg: Raster,
    pub svf_aveg_n: Raster,
    pub svf_aveg_e: Raster,
    pub svf_aveg_s: Raster,
    pub svf_aveg_w: Raster,
}

/// The kernel's shadow matrices: one bit per sky patch, set where the cell
/// sees the patch. Upstream's `shmat`, `vegshmat` and `vbshmat`.
#[derive(Debug, Clone)]
pub struct MatriciDOmbra {
    pub edifici: Array3<u8>,
    pub chiome: Array3<u8>,
    pub chiome_davanti_agli_edifici: Array3<u8>,
}

/// The sky pass over every cell of the surface model: the sky view factor, the
/// directional fields the step reads and, when `conserva_matrici`, the shadow
/// matrices of the anisotropic sky.
///
/// Unlike a shadow this does not depend on the sun: it is a property of the
/// Scenario and of the canopy layers the Periodo leaves on it. A Corsa
/// computes it once, good for every hour. Nothing caches it between Corse, so
/// the reference case computes it four times.
///
/// Every canopy layer costs one pass over the whole sky, because the kernel
/// reports whether a cell's view of a patch is blocked by vegetation, never by
/// which vegetation — the same reason [`ombre`] marches the layers one at a
/// time. Two layers cost a third pass, over their [`ChiomeRiunite`]. The three
/// fields give, by difference, the sky only the first layer covers, the sky
/// only the second covers and the sky both cover; the last lets through the
/// product of the two transmissivities. The building field that comes back is
/// the same on every pass, so one is kept and the rest discarded. The
/// directional fields and the matrices are those of the pass over the
/// [`ChiomeRiunite`], as `docs/adr/0006-chiome-riunite.md` decides.
///
/// # Errors
///
/// With more than two canopy layers, before any pass is run, and with whatever
/// the kernel refuses.
///
/// # Panics
///
/// If two canopy layers differ in shape, which the Derivazione never produces.
pub fn cielo(
    modello_di_superficie: &Raster,
    passo_m: f64,
    strati: &[StratoDiChioma],
    conserva_matrici: bool,
) -> Result<PassataDelCielo, String> {
    let rilievo = rilievo(modello_di_superficie, strati);

    // One pass over the sky, with a canopy top and a trunk zone or with neither.
    let passata = |chiome: Option<(&Raster, &Raster)>| {
        pool_a_un_thread().install(|| {
            solweig_vendorato::skyview::calculate_svf_inner(
                modello_di_superficie.to_owned(),
                chiome.map_or_else(
                    || Array2::zeros(modello_di_superficie.dim()),
                    |(cime, _)| cime.to_owned(),
                ),
                chiome.map_or_else(
                    || Array2::zeros(modello_di_superficie.dim()),
                    |(_, zona_tronco)| zona_tronco.to_owned(),
                ),
                passo_m as f32,
                chiome.is_some(),
                rilievo,
                PATCH_DEL_CIELO,
                // No floor on the patch elevation and no cap on the march: the
                // kernel's own default of 3 degrees sits below the lowest patch
                // of option 2, which is at 6, so it never truncates a patch that
                // counts. Zero means uncapped, and the domains this program
                // works on are two hundred metres across.
                None,
                None,
                None,
                0.0,
            )
        })
    };

    let Some(riunite) = ChiomeRiunite::da_strati(strati) else {
        // With no vegetation the kernel never runs the pass that fills
        // `svf_veg`: it comes back as the zeros it was allocated with, and the
        // published combination would subtract a hemisphere no tree is
        // blocking. Without a canopy the two fields are the same field.
        //
        // The ten vegetation fields the step reads go to one, as upstream's
        // Python sets them (`pysrc/solweig/models/precomputed.py:143-152`):
        // no canopy, nothing blocked.
        let mut esito = passata(None)?;
        for campo in [
            &mut esito.svf_veg,
            &mut esito.svf_veg_n,
            &mut esito.svf_veg_e,
            &mut esito.svf_veg_s,
            &mut esito.svf_veg_w,
            &mut esito.svf_veg_blocks_bldg_sh,
            &mut esito.svf_veg_blocks_bldg_sh_n,
            &mut esito.svf_veg_blocks_bldg_sh_e,
            &mut esito.svf_veg_blocks_bldg_sh_s,
            &mut esito.svf_veg_blocks_bldg_sh_w,
        ] {
            campo.fill(1.0);
        }
        let con_le_chiome = esito.svf.clone();
        let tolta = Raster::zeros(modello_di_superficie.dim());
        return Ok(dalla_passata(esito, con_le_chiome, tolta, conserva_matrici));
    };

    match strati {
        [strato] => {
            // The Chiome riunite of one layer are that layer, and upstream's
            // formula on its pass is the whole of the answer.
            let esito = passata(Some((&riunite.chiome, &riunite.zona_tronco)))?;
            let tolta = detrazione(&esito.svf_veg, strato.trasmissivita);
            let mut con_le_chiome = esito.svf.clone();
            con_le_chiome -= &tolta;
            // The arithmetic of 0.2.0 down to its floor at zero, so that what
            // a single layer gives does not move by a bit.
            con_le_chiome.mapv_inplace(|v| v.max(0.0));
            Ok(dalla_passata(esito, con_le_chiome, tolta, conserva_matrici))
        }
        [primo, secondo] => {
            let esito = passata(Some((&riunite.chiome, &riunite.zona_tronco)))?;
            let del_primo = passata(Some((&primo.chiome, &primo.zona_tronco)))?.svf_veg;
            let del_secondo = passata(Some((&secondo.chiome, &secondo.zona_tronco)))?.svf_veg;
            let (psi_a, psi_b) = (primo.trasmissivita, secondo.trasmissivita);
            let mut con_le_chiome = esito.svf.clone();
            let mut tolta = Raster::zeros(modello_di_superficie.dim());
            Zip::from(&mut con_le_chiome)
                .and(&mut tolta)
                .and(&del_primo)
                .and(&del_secondo)
                .and(&esito.svf_veg)
                .for_each(|cielo, tolta, &veg_a, &veg_b, &veg_u| {
                    // The sky each pass found covered: by the first layer, by
                    // the second, by the two together. What the union has and
                    // one layer has not is covered by the other alone, and
                    // what the two sum to beyond the union is covered by both.
                    // Each is a sum of patch weights in f32, so a share that
                    // is zero patch by patch can come out slightly negative:
                    // hence the floors.
                    let (a, b, u) = (1.0 - veg_a, 1.0 - veg_b, 1.0 - veg_u);
                    let solo_a = (u - b).max(0.0);
                    let solo_b = (u - a).max(0.0);
                    let entrambi = (a + b - u).max(0.0);
                    // Subtracted term by term and not as `tolta`, so that the
                    // published field keeps the bits of 0.3.0.
                    *cielo = (*cielo
                        - solo_a * (1.0 - psi_a)
                        - solo_b * (1.0 - psi_b)
                        - entrambi * (1.0 - psi_a * psi_b))
                        .clamp(0.0, 1.0);
                    *tolta = solo_a * (1.0 - psi_a)
                        + solo_b * (1.0 - psi_b)
                        + entrambi * (1.0 - psi_a * psi_b);
                });
            Ok(dalla_passata(esito, con_le_chiome, tolta, conserva_matrici))
        }
        // The difference between three fields tells two layers apart and no
        // more. Summing the deductions instead would count twice the sky two
        // layers share, without saying so. The Derivazione produces two layers
        // at most, so no Progetto gets here.
        _ => Err(format!(
            "sono arrivati {} Strati di chioma, e lo sky view factor con le chiome ne regge al \
             più due: quanta volta copre ciascuno si ricava per differenza fra tre campi, e \
             con un terzo Strato non bastano",
            strati.len()
        )),
    }
}

/// The pass over the Chiome riunite, or the one pass without canopy, laid out
/// as [`PassataDelCielo`].
fn dalla_passata(
    esito: SvfIntermediate,
    con_le_chiome: Raster,
    detrazione: Raster,
    conserva_matrici: bool,
) -> PassataDelCielo {
    let volta_coperta = esito.svf_veg.mapv(|visibile| 1.0 - visibile);
    let matrici = conserva_matrici.then(|| MatriciDOmbra {
        edifici: esito.bldg_sh_matrix,
        chiome: esito.veg_sh_matrix,
        chiome_davanti_agli_edifici: esito.veg_blocks_bldg_sh_matrix,
    });
    PassataDelCielo {
        sky_view_factor: SkyViewFactor {
            senza_chiome: esito.svf,
            con_le_chiome,
        },
        direzionali: Direzionali {
            svf_n: esito.svf_n,
            svf_e: esito.svf_e,
            svf_s: esito.svf_s,
            svf_w: esito.svf_w,
            svf_veg: esito.svf_veg,
            svf_veg_n: esito.svf_veg_n,
            svf_veg_e: esito.svf_veg_e,
            svf_veg_s: esito.svf_veg_s,
            svf_veg_w: esito.svf_veg_w,
            svf_aveg: esito.svf_veg_blocks_bldg_sh,
            svf_aveg_n: esito.svf_veg_blocks_bldg_sh_n,
            svf_aveg_e: esito.svf_veg_blocks_bldg_sh_e,
            svf_aveg_s: esito.svf_veg_blocks_bldg_sh_s,
            svf_aveg_w: esito.svf_veg_blocks_bldg_sh_w,
        },
        matrici,
        detrazione,
        volta_coperta,
    }
}

/// The sky one canopy layer takes away: nothing where it blocks nothing, and
/// what it blocks weighted by how little it lets through.
///
/// `svf - (1 - svf_veg) * (1 - psi)` is upstream's own formula, in
/// `pysrc/solweig/components/svf_resolution.py`; this is the term it subtracts.
/// `svf_veg` comes back already cleared of the sky the buildings were blocking,
/// so the two deductions do not overlap.
fn detrazione(svf_veg: &Raster, trasmissivita: f32) -> Raster {
    svf_veg.mapv(|visibile| (1.0 - visibile) * (1.0 - trasmissivita))
}

/// What one canopy layer leaves of the beam: 1 where it casts nothing, its
/// transmissivity where it does.
///
/// `veg_sh` comes back from the kernel with 1 meaning lit, and already cleared
/// of the building shadow — the kernel zeroes its canopy shadow wherever a
/// building is already shading, so a cell in a building's shadow gets a factor
/// of 1 here and stays at the zero the building put there.
fn attenuazione(veg_sh: &Raster, trasmissivita: f32) -> Raster {
    veg_sh.mapv(|illuminata| 1.0 - (1.0 - illuminata) * (1.0 - trasmissivita))
}

/// The walls of a Scenario as the step reads them: their height above the
/// ground, and the aspect in degrees that upstream's `wall_aspect` computes.
#[derive(Debug, Clone)]
pub struct Muri {
    pub altezza: Raster,
    pub orientamento_gradi: Raster,
}

/// The ground properties of every cell, upstream's `PropertyViews`.
#[derive(Debug, Clone)]
pub struct ProprietaDelSuolo {
    pub albedo: Raster,
    pub emissivita: Raster,
    pub tgk: Raster,
    pub tstart: Raster,
    pub tmaxlst: Raster,
}

/// The thermal state one Ora hands to the next, upstream's `ThermalStateViews`.
#[derive(Debug, Clone)]
pub struct StatoTermico {
    pub firstdaytime: i32,
    pub timeadd: f32,
    pub timestep_dec: f32,
    pub tgmap1: Raster,
    pub tgmap1_e: Raster,
    pub tgmap1_s: Raster,
    pub tgmap1_w: Raster,
    pub tgmap1_n: Raster,
    pub tgout1: Raster,
}

/// What one Ora of the fused step yields.
#[derive(Debug, Clone)]
pub struct UscitaDellOra {
    pub temperatura_media_radiante: Raster,
    /// The step's own lit fraction, 1 in the sun.
    pub illuminata: Raster,
    pub k_giu: Raster,
    pub k_su: Raster,
    pub l_giu: Raster,
    pub l_su: Raster,
    /// `timeadd` and the six rasters from the step; `firstdaytime` and
    /// `timestep_dec` carried from the input, since the step does not return
    /// them (upstream `rust/src/pipeline.rs:249-267`). Updating them is not
    /// this module's work.
    pub stato: StatoTermico,
}

/// What one Scenario hands to the step, the same for every Ora.
pub struct IngressiDelloScenario<'a> {
    pub modello_di_superficie: &'a Raster,
    pub passo_m: f64,
    pub strati: &'a [StratoDiChioma],
    /// Building heights above the terrain, > 0 under an Edificio: the
    /// `costruite` of `src/corsa.rs`. The Motore turns it into upstream's
    /// mask, 1 on ground.
    pub costruite: &'a Raster,
    pub muri: &'a Muri,
    pub cielo: &'a PassataDelCielo,
    pub svfalfa: &'a Raster,
    pub suolo: &'a ProprietaDelSuolo,
    pub trasmissivita_verso_il_cielo: &'a Raster,
    pub persona: HumanScalars,
    pub anisotropo: bool,
}

/// The wall material of upstream's defaults, `pysrc/solweig/computation.py:323-327`
/// at the pin; `pysrc/solweig/data/default_materials.json` repeats them under
/// `Walls`.
const ALBEDO_DEI_MURI: f32 = 0.20;
const EMISSIVITA_DEI_MURI: f32 = 0.90;
const TGK_DEI_MURI: f32 = 0.37;
const TSTART_DEI_MURI: f32 = -3.41;
const TMAXLST_DEI_MURI: f32 = 15.0;

/// Upstream's default reach of a shadow, `pysrc/solweig/computation.py:410` and
/// `pysrc/solweig/models/config.py:89` at the pin. Past the project's 200 m
/// ceiling, so on our domains it caps nothing.
const DISTANZA_MASSIMA_DELL_OMBRA_M: f32 = 1000.0;

/// One Scenario ready to step through its Ore: the inputs checked against the
/// surface model once, and what the step needs that no Ora changes.
pub struct PassoDelloScenario<'a> {
    ingressi: IngressiDelloScenario<'a>,
    config: ConfigScalars,
    edifici: Raster,
    valida: Array2<u8>,
    gvf: GvfGeometryCache,
    riunite: Option<ChiomeRiunite>,
    cespugli: Raster,
}

impl<'a> PassoDelloScenario<'a> {
    /// Checks every raster against the surface model and builds what the step
    /// reuses: the configuration, the building mask, the Chiome riunite and
    /// the ground view factor geometry.
    ///
    /// # Errors
    ///
    /// When a raster or a canopy layer does not have the surface model's
    /// shape, naming it, and when the anisotropic sky is asked for and the sky
    /// pass kept no shadow matrices.
    pub fn nuovo(ingressi: IngressiDelloScenario<'a>) -> Result<Self, String> {
        let forma = ingressi.modello_di_superficie.dim();
        let cielo = ingressi.cielo;
        let d = &cielo.direzionali;
        let suolo = ingressi.suolo;
        [
            ("costruite", ingressi.costruite),
            ("muri.altezza", &ingressi.muri.altezza),
            ("muri.orientamento_gradi", &ingressi.muri.orientamento_gradi),
            ("svfalfa", ingressi.svfalfa),
            ("suolo.albedo", &suolo.albedo),
            ("suolo.emissivita", &suolo.emissivita),
            ("suolo.tgk", &suolo.tgk),
            ("suolo.tstart", &suolo.tstart),
            ("suolo.tmaxlst", &suolo.tmaxlst),
            (
                "trasmissivita_verso_il_cielo",
                ingressi.trasmissivita_verso_il_cielo,
            ),
            (
                "sky_view_factor.senza_chiome",
                &cielo.sky_view_factor.senza_chiome,
            ),
            (
                "sky_view_factor.con_le_chiome",
                &cielo.sky_view_factor.con_le_chiome,
            ),
            ("svf_n", &d.svf_n),
            ("svf_e", &d.svf_e),
            ("svf_s", &d.svf_s),
            ("svf_w", &d.svf_w),
            ("svf_veg", &d.svf_veg),
            ("svf_veg_n", &d.svf_veg_n),
            ("svf_veg_e", &d.svf_veg_e),
            ("svf_veg_s", &d.svf_veg_s),
            ("svf_veg_w", &d.svf_veg_w),
            ("svf_aveg", &d.svf_aveg),
            ("svf_aveg_n", &d.svf_aveg_n),
            ("svf_aveg_e", &d.svf_aveg_e),
            ("svf_aveg_s", &d.svf_aveg_s),
            ("svf_aveg_w", &d.svf_aveg_w),
        ]
        .into_iter()
        .chain(ingressi.strati.iter().flat_map(|strato| {
            [
                ("le chiome di uno Strato", &strato.chiome),
                ("la zona-tronco di uno Strato", &strato.zona_tronco),
            ]
        }))
        .try_for_each(|(nome, raster)| stessa_forma(nome, raster, forma))?;

        if ingressi.anisotropo && cielo.matrici.is_none() {
            return Err(
                "il cielo anisotropo legge le matrici d'ombra, e la passata del cielo \
                        non le ha conservate: va chiesta con conserva_matrici"
                    .to_owned(),
            );
        }

        let edifici = maschera_degli_edifici(ingressi.costruite);
        let passo_m = ingressi.passo_m as f32;
        // Upstream `rust/src/pipeline.rs:415-416` at the pin, inside the
        // pyfunction `precompute_gvf_cache` that the extraction leaves out.
        let altezza = ingressi.persona.height;
        let (first_ht, second_ht) = (altezza.round().max(1.0), altezza * 20.0);
        let gvf = pool_a_un_thread().install(|| {
            precompute_gvf_geometry_cpu(
                edifici.view(),
                ingressi.muri.orientamento_gradi.view(),
                ingressi.muri.altezza.view(),
                suolo.albedo.view(),
                passo_m,
                first_ht,
                second_ht,
                ALBEDO_DEI_MURI,
            )
        });
        let config = ConfigScalars {
            pixel_size: passo_m,
            max_height: rilievo(ingressi.modello_di_superficie, ingressi.strati),
            albedo_wall: ALBEDO_DEI_MURI,
            emis_wall: EMISSIVITA_DEI_MURI,
            tgk_wall: TGK_DEI_MURI,
            tstart_wall: TSTART_DEI_MURI,
            tmaxlst_wall: TMAXLST_DEI_MURI,
            use_veg: !ingressi.strati.is_empty(),
            has_walls: true,
            use_anisotropic: ingressi.anisotropo,
            max_shadow_distance_m: DISTANZA_MASSIMA_DELL_OMBRA_M,
        };
        Ok(Self {
            valida: ingressi
                .modello_di_superficie
                .mapv(|quota| u8::from(quota.is_finite())),
            riunite: ChiomeRiunite::da_strati(ingressi.strati),
            // Zeros for the reason `ombre` gives: OMBRA has no Cespuglio.
            cespugli: Raster::zeros(forma),
            ingressi,
            config,
            edifici,
            gvf,
        })
    }

    /// One Ora of the fused step, on one thread.
    ///
    /// `meteo.patch_option` is not read: the step gets `PATCH_DEL_CIELO`,
    /// the option of the pass that produced the shadow matrices.
    ///
    /// # Errors
    ///
    /// When `trasmissivita_lungo_il_raggio` or a raster of `stato` does not
    /// have the surface model's shape, and with whatever the step refuses,
    /// unchanged.
    pub fn ora(
        &self,
        meteo: &WeatherScalars,
        trasmissivita_lungo_il_raggio: &Raster,
        stato: &StatoTermico,
    ) -> Result<UscitaDellOra, String> {
        let i = &self.ingressi;
        let forma = i.modello_di_superficie.dim();
        // Inside the copy a (1, 1) or (1, cols) beam raster broadcasts at the
        // beam expression without complaint.
        [
            (
                "trasmissivita_lungo_il_raggio",
                trasmissivita_lungo_il_raggio,
            ),
            ("stato.tgmap1", &stato.tgmap1),
            ("stato.tgmap1_e", &stato.tgmap1_e),
            ("stato.tgmap1_s", &stato.tgmap1_s),
            ("stato.tgmap1_w", &stato.tgmap1_w),
            ("stato.tgmap1_n", &stato.tgmap1_n),
            ("stato.tgout1", &stato.tgout1),
        ]
        .into_iter()
        .try_for_each(|(nome, raster)| stessa_forma(nome, raster, forma))?;

        let meteo = WeatherScalars {
            patch_option: i32::from(PATCH_DEL_CIELO),
            ..meteo.clone()
        };
        let cielo = i.cielo;
        let d = &cielo.direzionali;
        let matrici = cielo.matrici.as_ref();
        let ingressi = TimestepInputs {
            weather: &meteo,
            human: &i.persona,
            config: &self.config,
            gvf_cache: Some(&self.gvf),
            surface: SurfaceViews {
                dsm: i.modello_di_superficie.view(),
                cdsm: self.riunite.as_ref().map(|r| r.chiome.view()),
                tdsm: self.riunite.as_ref().map(|r| r.zona_tronco.view()),
                bush: self.riunite.as_ref().map(|_| self.cespugli.view()),
                wall_ht: Some(i.muri.altezza.view()),
                wall_asp: Some(i.muri.orientamento_gradi.view()),
            },
            svf: SvfViews {
                svf: cielo.sky_view_factor.senza_chiome.view(),
                svf_n: d.svf_n.view(),
                svf_e: d.svf_e.view(),
                svf_s: d.svf_s.view(),
                svf_w: d.svf_w.view(),
                svf_veg: d.svf_veg.view(),
                svf_veg_n: d.svf_veg_n.view(),
                svf_veg_e: d.svf_veg_e.view(),
                svf_veg_s: d.svf_veg_s.view(),
                svf_veg_w: d.svf_veg_w.view(),
                svf_aveg: d.svf_aveg.view(),
                svf_aveg_n: d.svf_aveg_n.view(),
                svf_aveg_e: d.svf_aveg_e.view(),
                svf_aveg_s: d.svf_aveg_s.view(),
                svf_aveg_w: d.svf_aveg_w.view(),
                svfbuveg: cielo.sky_view_factor.con_le_chiome.view(),
                svfalfa: i.svfalfa.view(),
            },
            properties: PropertyViews {
                alb_grid: i.suolo.albedo.view(),
                emis_grid: i.suolo.emissivita.view(),
                tgk_grid: i.suolo.tgk.view(),
                tstart_grid: i.suolo.tstart.view(),
                tmaxlst_grid: i.suolo.tmaxlst.view(),
            },
            buildings: self.edifici.view(),
            lc_grid: None,
            shmat: matrici.map(|m| m.edifici.view()),
            vegshmat: matrici.map(|m| m.chiome.view()),
            vbshmat: matrici.map(|m| m.chiome_davanti_agli_edifici.view()),
            trasmissivita_raggio: trasmissivita_lungo_il_raggio.view(),
            trasmissivita_cielo: i.trasmissivita_verso_il_cielo.view(),
            state: ThermalStateViews {
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
            output_mask: OUT_ALL,
        };
        let esito = pool_a_un_thread().install(|| compute_timestep_rust(&ingressi))?;
        let (Some(illuminata), Some(k_giu), Some(k_su), Some(l_giu), Some(l_su)) =
            (esito.shadow, esito.kdown, esito.kup, esito.ldown, esito.lup)
        else {
            return Err("il passo non ha restituito uno dei campi che OUT_ALL chiede".to_owned());
        };
        Ok(UscitaDellOra {
            temperatura_media_radiante: esito.tmrt,
            illuminata,
            k_giu,
            k_su,
            l_giu,
            l_su,
            stato: StatoTermico {
                firstdaytime: stato.firstdaytime,
                timeadd: esito.timeadd,
                timestep_dec: stato.timestep_dec,
                tgmap1: esito.tgmap1,
                tgmap1_e: esito.tgmap1_e,
                tgmap1_s: esito.tgmap1_s,
                tgmap1_w: esito.tgmap1_w,
                tgmap1_n: esito.tgmap1_n,
                tgout1: esito.tgout1,
            },
        })
    }
}

fn stessa_forma(nome: &str, raster: &Raster, forma: (usize, usize)) -> Result<(), String> {
    if raster.dim() == forma {
        return Ok(());
    }
    Err(format!(
        "{nome} è di {} × {} celle e il modello di superficie di {} × {}: ogni raster del \
         passo ha la forma del modello di superficie",
        raster.nrows(),
        raster.ncols(),
        forma.0,
        forma.1
    ))
}

/// Upstream's `buildings`: 1 on the ground and 0 under an Edificio
/// (`rust/src/pipeline_inputs.rs:107` at the pin), the reverse of what the name
/// suggests. A cell with no height to read is ground.
fn maschera_degli_edifici(costruite: &Raster) -> Raster {
    costruite.mapv(|altezza| if altezza > 0.0 { 0.0 } else { 1.0 })
}

/// The kernel parallelises its own inner loops with `rayon`, and on a domain
/// this size the fork-join costs far more than the arithmetic it splits.
///
/// Measured on the reference case, 50x50 cells, one call to [`ombre`]: 35.94 ms
/// with rayon's default pool against 0.39 ms on a single thread with the sun at
/// 45 degrees, and 144.78 ms against 1.49 ms with the sun at 5. A reviewer
/// measured 500x500 at 48.2 ms against 3.8 ms, so the break-even sits well above
/// the 100x100 ceiling this project works to: there is no size we run at where
/// the threads pay for themselves.
///
/// Built once and kept, because building it per call would put back exactly the
/// cost this removes.
fn pool_a_un_thread() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("un pool rayon da un thread")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_maschera_degli_edifici_ha_il_verso_di_monte() {
        let costruite = ndarray::array![[6.0, 0.0, f32::NAN]];
        let maschera = maschera_degli_edifici(&costruite);
        assert_eq!(maschera, ndarray::array![[0.0, 1.0, 1.0]]);
    }
}
