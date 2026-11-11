//! The Motore and the sun that drives it. Degenerate inputs first: each test
//! below fails if the defence it names is removed from `src/motore.rs` or
//! `src/sole.rs`.
//!
//! Row 0 of every raster is the northernmost, and the azimuth is degrees from
//! north clockwise, as everywhere else in the project.

use ombra::derivazione::{
    ChiomeRiunite, Raster, Stagione, StratoDiChioma, TRASMISSIVITA_CON_FOGLIE,
    TRASMISSIVITA_SENZA_FOGLIE,
};
use ombra::dominio::Data;
use ombra::motore::{ombre, sky_view_factor, versione, SkyViewFactor, ALTEZZA_MINIMA_GRADI};
use ombra::sole::{posizione, PosizioneSolare};

/// Bastia Umbra, the reference case: the coordinates read from the model, not
/// the ones of the weather station.
const LATITUDINE: f64 = 43.07;
const LONGITUDINE: f64 = 12.56;
/// +1 all year, daylight saving included, and that is not an oversight: an EPW
/// file carries its hours in local standard time and never shifts them, so the
/// clock these tests read is the clock the weather data is written against. A
/// July hour here is one hour behind the wall clock an Italian would read, and
/// that is the correct reading of the measurement.
const FUSO: f64 = 1.0;

fn luglio() -> Data {
    Data {
        anno: 2026,
        mese: 7,
        giorno: 15,
    }
}

fn sole_a(altezza_gradi: f64, azimut_gradi: f64) -> PosizioneSolare {
    PosizioneSolare {
        altezza_gradi,
        azimut_gradi,
    }
}

/// The highest the sun gets in the day, and the local hour it gets there.
fn culminazione(data: Data) -> (f64, PosizioneSolare) {
    (0..1440)
        .map(|minuto| {
            let ora = minuto as f64 / 60.0;
            (
                ora,
                posizione(data, ora, FUSO, LATITUDINE, LONGITUDINE).unwrap(),
            )
        })
        .fold(
            (0.0, sole_a(f64::NEG_INFINITY, 0.0)),
            |migliore, corrente| {
                if corrente.1.altezza_gradi > migliore.1.altezza_gradi {
                    corrente
                } else {
                    migliore
                }
            },
        )
}

#[test]
fn a_mezzogiorno_solare_di_luglio_il_sole_e_alto_e_a_sud() {
    let (_, culmine) = culminazione(luglio());
    // 90 - 43.07 + 21.4 degrees of declination in mid-July.
    assert!(
        (culmine.altezza_gradi - 68.3).abs() < 1.0,
        "altezza al culmine: {}",
        culmine.altezza_gradi
    );
    assert!(
        (culmine.azimut_gradi - 180.0).abs() < 1.0,
        "azimut al culmine: {}",
        culmine.azimut_gradi
    );
}

#[test]
fn a_un_ora_dell_orologio_fissa_il_sole_sta_dove_deve() {
    // The one assertion in this file tied to the clock rather than to the shape
    // of the day. Everything that scans for a culmination is blind to a time
    // error by construction: shift the clock and the culmination moves with it.
    // 170.17 degrees is where the sun is over Bastia Umbra at 12:00 of the local
    // solar clock on 15 July. Drop the equation of time and it reads 173.80,
    // drop the longitude and it reads 142.19, get the offset wrong by an hour
    // and it reads 137.72: half a degree of tolerance refuses all three.
    let mezzogiorno = posizione(luglio(), 12.0, FUSO, LATITUDINE, LONGITUDINE).unwrap();
    assert!(
        (mezzogiorno.azimut_gradi - 170.17).abs() < 0.5,
        "azimut alle 12:00: {}",
        mezzogiorno.azimut_gradi
    );
    assert!(
        (mezzogiorno.altezza_gradi - 68.35).abs() < 0.5,
        "altezza alle 12:00: {}",
        mezzogiorno.altezza_gradi
    );
}

#[test]
fn l_alba_e_il_tramonto_cadono_all_ora_giusta_dell_orologio() {
    // Sunrise at 04:48 and sunset at 19:42 of the local solar clock, twelve
    // minutes either side of each crossing. The sun climbs about ten degrees an
    // hour here, so the box below is two degrees tall: any time error worth the
    // name — a quarter of an hour of equation of time, fifty minutes of
    // longitude, a whole hour of offset — pushes the sun out of it.
    for (ora, atteso) in [(4.7, -2.0..0.0), (4.9, 0.0..2.0)] {
        let sole = posizione(luglio(), ora, FUSO, LATITUDINE, LONGITUDINE).unwrap();
        assert!(
            atteso.contains(&sole.altezza_gradi),
            "all'alba, alle {ora}, altezza {} fuori da {atteso:?}",
            sole.altezza_gradi
        );
    }
    for (ora, atteso) in [(19.6, 0.0..2.0), (19.8, -2.0..0.0)] {
        let sole = posizione(luglio(), ora, FUSO, LATITUDINE, LONGITUDINE).unwrap();
        assert!(
            atteso.contains(&sole.altezza_gradi),
            "al tramonto, alle {ora}, altezza {} fuori da {atteso:?}",
            sole.altezza_gradi
        );
    }
}

#[test]
fn il_sole_gira_da_est_a_ovest_passando_per_il_sud() {
    let (ora_del_culmine, _) = culminazione(luglio());
    let mattina = posizione(
        luglio(),
        ora_del_culmine - 3.0,
        FUSO,
        LATITUDINE,
        LONGITUDINE,
    )
    .unwrap();
    let sera = posizione(
        luglio(),
        ora_del_culmine + 3.0,
        FUSO,
        LATITUDINE,
        LONGITUDINE,
    )
    .unwrap();
    assert!(
        (60.0..180.0).contains(&mattina.azimut_gradi),
        "azimut del mattino: {}",
        mattina.azimut_gradi
    );
    assert!(
        (180.0..300.0).contains(&sera.azimut_gradi),
        "azimut della sera: {}",
        sera.azimut_gradi
    );
}

#[test]
fn un_ora_oltre_la_ventiquattresima_e_la_stessa_ora_del_giorno_dopo() {
    // A Corsa of forty-eight Ore asks for the sun of 30:30 and never rewrites
    // the date: the argument has to carry the day forward by itself.
    let domani = Data {
        giorno: 16,
        ..luglio()
    };
    let oltre = posizione(luglio(), 30.5, FUSO, LATITUDINE, LONGITUDINE).unwrap();
    let dopo = posizione(domani, 6.5, FUSO, LATITUDINE, LONGITUDINE).unwrap();
    assert!(
        (oltre.altezza_gradi - dopo.altezza_gradi).abs() < 1e-9
            && (oltre.azimut_gradi - dopo.azimut_gradi).abs() < 1e-9,
        "30:30 del 15 dà {oltre:?}, 06:30 del 16 dà {dopo:?}"
    );
}

#[test]
fn una_data_che_non_e_una_data_non_ha_posizione_solare() {
    let inesistente = Data {
        anno: 2026,
        mese: 2,
        giorno: 30,
    };
    assert!(posizione(inesistente, 12.0, FUSO, LATITUDINE, LONGITUDINE).is_none());
}

/// A flat domain at `quota`, with a 10 m tower at the centre.
fn torre(lato: usize, quota: f32) -> Raster {
    let mut dsm = Raster::from_elem((lato, lato), quota);
    dsm[[lato / 2, lato / 2]] = quota + 10.0;
    dsm
}

#[test]
fn il_sole_sotto_l_orizzonte_lascia_tutto_in_ombra() {
    // Ours, not the kernel's: below the horizon the kernel finds no obstruction
    // and reports every cell lit. That a place at night is not in the sun is a
    // modelling decision, and it is taken here.
    let illuminazione = ombre(&torre(11, 0.0), 1.0, sole_a(-10.0, 180.0), &[]);
    assert!(
        illuminazione.iter().all(|&v| v == 0.0),
        "una cella al sole di notte: {illuminazione}"
    );
}

#[test]
fn un_dominio_piatto_col_sole_alto_e_tutto_al_sole() {
    let piatto = Raster::zeros((21, 21));
    let illuminazione = ombre(&piatto, 1.0, sole_a(60.0, 180.0), &[]);
    assert!(
        illuminazione.iter().all(|&v| v > 0.99),
        "ombra dal nulla su un dominio piatto: {illuminazione}"
    );
}

#[test]
fn la_torre_getta_la_sua_ombra_a_nord_col_sole_a_sud_a_quarantacinque_gradi() {
    let lato = 25;
    let c = lato / 2;
    let illuminazione = ombre(&torre(lato, 0.0), 1.0, sole_a(45.0, 180.0), &[]);

    // A 10 m tower with the sun at 45 degrees casts 10 m of shadow, and it falls
    // to the north because the sun is to the south.
    for distanza in 1..=9 {
        assert!(
            illuminazione[[c - distanza, c]] < 0.5,
            "a {distanza} m a nord la torre non fa ombra: {illuminazione}"
        );
    }
    assert!(
        illuminazione[[c - 11, c]] > 0.5,
        "l'ombra arriva a 11 m a nord, piu' lunga della torre: {illuminazione}"
    );
    assert!(
        illuminazione[[c + 1, c]] > 0.5,
        "ombra a sud, dalla parte del sole: {illuminazione}"
    );
    assert!(
        illuminazione[[c, c - 1]] > 0.5,
        "ombra a ovest, di traverso al sole: {illuminazione}"
    );
}

#[test]
fn il_sole_all_alba_non_fa_panico_ne_ombre_infinite() {
    let lato = 21;
    let c = lato / 2;
    // Just above the threshold, where the shadow of a 10 m tower is some 285 m
    // and runs off a 21 m domain: the longest march the Motore ever takes.
    let illuminazione = ombre(
        &torre(lato, 0.0),
        1.0,
        sole_a(ALTEZZA_MINIMA_GRADI + 0.01, 90.0),
        &[],
    );
    assert!(
        illuminazione
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
        "illuminazione fuori scala all'alba: {illuminazione}"
    );
    // A domain entirely in shadow would satisfy the range too, and it is the
    // wrong answer: with the sun in the east the cells east of the tower see it.
    assert!(
        illuminazione[[c, lato - 1]] > 0.5,
        "all'alba il bordo est, lontano dalla torre, non e' al sole: {illuminazione}"
    );
    assert!(
        illuminazione[[0, 0]] > 0.5,
        "all'alba l'angolo nord-ovest, che nessuna ombra raggiunge, e' spento: {illuminazione}"
    );
}

#[test]
fn il_terreno_a_quota_costante_da_le_stesse_ombre_del_terreno_a_zero() {
    // What the kernel is owed is the relief, max minus min, not the maximum:
    // the height the ray must climb to clear every obstacle. Feed it the
    // maximum and a domain sitting below the datum stops casting shadows at all.
    let riferimento = ombre(&torre(21, 0.0), 1.0, sole_a(45.0, 180.0), &[]);
    for quota in [100.0, -100.0] {
        let sollevato = ombre(&torre(21, quota), 1.0, sole_a(45.0, 180.0), &[]);
        assert_eq!(
            sollevato, riferimento,
            "a quota {quota} le ombre cambiano: {sollevato}"
        );
    }
}

#[test]
fn una_griglia_di_una_cella_funziona() {
    let illuminazione = ombre(&Raster::zeros((1, 1)), 1.0, sole_a(45.0, 180.0), &[]);
    assert_eq!(illuminazione.dim(), (1, 1));
    assert!(illuminazione[[0, 0]] > 0.5);
}

#[test]
fn la_versione_riporta_il_commit_vendorato() {
    let v = versione();
    assert_eq!(v.commit, "c31c5758236078b6b5f7bc9a931e2fce264bfc88");
    assert_eq!(v.data_presa, "2026-11-10");
}

#[test]
fn alla_soglia_di_altezza_e_tutto_in_ombra_e_appena_sopra_no() {
    // The threshold is a modelling decision, so it is the threshold and not the
    // horizon that has to be exact: at the declared elevation the domain is
    // shaded, and the smallest step above it starts computing again.
    let alla_soglia = ombre(
        &torre(11, 0.0),
        1.0,
        sole_a(ALTEZZA_MINIMA_GRADI, 180.0),
        &[],
    );
    assert!(
        alla_soglia.iter().all(|&v| v == 0.0),
        "alla soglia di {ALTEZZA_MINIMA_GRADI} gradi una cella e' al sole: {alla_soglia}"
    );

    let appena_sopra = ombre(
        &torre(11, 0.0),
        1.0,
        sole_a(ALTEZZA_MINIMA_GRADI + 0.001, 180.0),
        &[],
    );
    assert!(
        appena_sopra.iter().any(|&v| v > 0.5),
        "appena sopra la soglia il Motore non calcola piu' niente: {appena_sopra}"
    );
}

#[test]
fn col_passo_di_due_metri_l_ombra_della_torre_e_lunga_cinque_celle() {
    // The same 10 m tower and the same 45 degrees as above, on a coarser grid:
    // 10 m of shadow is five cells here and ten there. Every other call in this
    // file passes a step of 1.0, where a step ignored and a step of one metre
    // give the same picture.
    let lato = 25;
    let c = lato / 2;
    let illuminazione = ombre(&torre(lato, 0.0), 2.0, sole_a(45.0, 180.0), &[]);

    for distanza in 1..=4 {
        assert!(
            illuminazione[[c - distanza, c]] < 0.5,
            "a {} m a nord la torre non fa ombra: {illuminazione}",
            distanza * 2
        );
    }
    assert!(
        illuminazione[[c - 6, c]] > 0.5,
        "l'ombra arriva a 12 m a nord, piu' lunga della torre: {illuminazione}"
    );
}

/// A canopy layer with one canopy on the centre cell of a `lato` x `lato`
/// domain: top at `chioma` metres, trunk zone up to half of it.
fn chioma_al_centro(
    nome: &'static str,
    trasmissivita: f32,
    lato: usize,
    chioma: f32,
) -> StratoDiChioma {
    let mut chiome = Raster::from_elem((lato, lato), f32::NAN);
    let mut zona_tronco = Raster::from_elem((lato, lato), f32::NAN);
    chiome[[lato / 2, lato / 2]] = chioma;
    zona_tronco[[lato / 2, lato / 2]] = chioma / 2.0;
    StratoDiChioma {
        nome,
        trasmissivita,
        chiome,
        zona_tronco,
    }
}

fn minimo(illuminazione: &Raster) -> f32 {
    illuminazione.iter().copied().fold(f32::INFINITY, f32::min)
}

#[test]
fn una_chioma_lascia_passare_la_sua_trasmissivita_e_non_spegne_la_cella() {
    let lato = 25;
    let c = lato / 2;
    let piatto = Raster::zeros((lato, lato));
    let strati = [chioma_al_centro(
        "chiome",
        TRASMISSIVITA_CON_FOGLIE,
        lato,
        10.0,
    )];
    let illuminazione = ombre(&piatto, 1.0, sole_a(45.0, 180.0), &strati);

    // A canopy is not a wall: the darkest cell of a domain with no building in
    // it is the transmissivity, never zero. Getting a zero here would mean the
    // canopy had been treated as opaque.
    // `1 - (1 - 0) * (1 - 0.03)` in f32 lands a few ulp short of 0.03, so the
    // comparison is on the tolerance and not on the bit pattern.
    assert!(
        (minimo(&illuminazione) - TRASMISSIVITA_CON_FOGLIE).abs() < 1e-6,
        "una chioma ha spento una cella invece di ombreggiarla: minimo {}",
        minimo(&illuminazione)
    );
    // The shade falls to the north, between the shadow of the trunk top and the
    // shadow of the canopy top: sun at 45 degrees, canopy at 10 m, trunk at 5.
    assert!(
        (c - 9..c - 5).any(|riga| illuminazione[[riga, c]] < 1.0),
        "nessuna ombra di chioma fra 5 e 9 m a nord: {illuminazione}"
    );
    assert_eq!(
        illuminazione[[c + 3, c]],
        1.0,
        "ombra a sud, dalla parte del sole: {illuminazione}"
    );
}

#[test]
fn due_strati_sulla_stessa_cella_moltiplicano_le_trasmissivita() {
    // What a beam crossing two canopies does. It is also the reason the layers
    // exist: the Motore says a cell is in the shade of a canopy, never of which
    // canopy, so two transmissivities need two marches.
    let lato = 25;
    let piatto = Raster::zeros((lato, lato));
    let strati = [
        chioma_al_centro("chiome", TRASMISSIVITA_CON_FOGLIE, lato, 10.0),
        chioma_al_centro("chiome spoglie", TRASMISSIVITA_SENZA_FOGLIE, lato, 10.0),
    ];
    let illuminazione = ombre(&piatto, 1.0, sole_a(45.0, 180.0), &strati);
    let atteso = TRASMISSIVITA_CON_FOGLIE * TRASMISSIVITA_SENZA_FOGLIE;
    assert!(
        (minimo(&illuminazione) - atteso).abs() < 1e-7,
        "due strati non si moltiplicano: minimo {}, atteso {atteso}",
        minimo(&illuminazione)
    );
}

#[test]
fn una_chioma_nell_ombra_di_un_edificio_non_la_schiarisce() {
    // The building is opaque and the canopy is not, but a canopy cannot put
    // light back where a wall has taken it away.
    let lato = 25;
    let c = lato / 2;
    let mut strati = [chioma_al_centro(
        "chiome",
        TRASMISSIVITA_CON_FOGLIE,
        lato,
        8.0,
    )];
    // Move the canopy three cells north of the tower, inside its shadow.
    strati[0].chiome = Raster::from_elem((lato, lato), f32::NAN);
    strati[0].zona_tronco = Raster::from_elem((lato, lato), f32::NAN);
    strati[0].chiome[[c - 3, c]] = 8.0;
    strati[0].zona_tronco[[c - 3, c]] = 4.0;

    let illuminazione = ombre(&torre(lato, 0.0), 1.0, sole_a(45.0, 180.0), &strati);
    for distanza in 1..=9 {
        assert_eq!(
            illuminazione[[c - distanza, c]],
            0.0,
            "a {distanza} m a nord l'ombra della torre non è più nera: {illuminazione}"
        );
    }
}

#[test]
fn uno_strato_senza_chiome_non_cambia_niente() {
    // The Derivazione does not build an empty layer, and this says what would
    // happen if it did: nothing. It is the guard that keeps the vegetation path
    // from moving a result on a Scenario with no trees.
    let lato = 21;
    let piatto = torre(lato, 0.0);
    let vuoto = StratoDiChioma {
        nome: "chiome",
        trasmissivita: TRASMISSIVITA_CON_FOGLIE,
        chiome: Raster::from_elem((lato, lato), f32::NAN),
        zona_tronco: Raster::from_elem((lato, lato), f32::NAN),
    };
    assert_eq!(
        ombre(&piatto, 1.0, sole_a(45.0, 180.0), &[vuoto]),
        ombre(&piatto, 1.0, sole_a(45.0, 180.0), &[])
    );
}

#[test]
fn di_notte_nemmeno_una_chioma_illumina() {
    let lato = 11;
    let strati = [chioma_al_centro(
        "chiome",
        TRASMISSIVITA_CON_FOGLIE,
        lato,
        10.0,
    )];
    let illuminazione = ombre(&torre(lato, 0.0), 1.0, sole_a(-10.0, 180.0), &strati);
    assert!(
        illuminazione.iter().all(|&v| v == 0.0),
        "una cella al sole di notte: {illuminazione}"
    );
}

#[test]
fn nel_caso_di_riferimento_i_due_scenari_non_danno_piu_la_stessa_ombra() {
    // The defect this whole layer exists to close. With the vegetation outside
    // the shadow calculation, 616 trees and 846 trees produced fields identical
    // to the last digit — and two identical results read as "the planting does
    // nothing", which is a claim the model was not entitled to make.
    let progetto = ombra::progetto::leggi("casi/bastia/progetto").unwrap();
    let sole = sole_a(45.0, 180.0);
    let campo = |nome: &str, periodo: &ombra::dominio::Periodo| {
        let scenario = progetto
            .scenari
            .iter()
            .find(|s| s.nome == nome)
            .expect("il caso di riferimento porta questo Scenario");
        let raster = ombra::derivazione::deriva(&progetto.griglia, scenario, periodo).unwrap();
        ombre(
            &raster.modello_di_superficie,
            progetto.griglia.passo_m,
            sole,
            &raster.strati_di_chioma,
        )
    };
    let sole_totale = |r: &Raster| r.iter().map(|&v| f64::from(v)).sum::<f64>();

    assert_eq!(progetto.periodi.len(), 2, "estate e inverno");
    for periodo in &progetto.periodi {
        let fatto = campo("stato-di-fatto", periodo);
        let interventi = campo("interventi", periodo);
        assert_ne!(
            fatto, interventi,
            "nel Periodo «{}» i due Scenari danno la stessa ombra: gli alberi non stanno \
             entrando nel calcolo",
            periodo.nome
        );
        // 846 trees against 616: the planted Scenario is the darker one.
        assert!(
            sole_totale(&interventi) < sole_totale(&fatto),
            "nel Periodo «{}» lo Scenario con più alberi prende più sole: {} contro {}",
            periodo.nome,
            sole_totale(&interventi),
            sole_totale(&fatto)
        );
    }
}

// --- Sky view factor -------------------------------------------------------
//
// The fraction of the sky hemisphere a cell can see. Unlike a shadow it does
// not depend on the sun, so it is a property of the Scenario and not of the
// hour, and it is the first quantity of the radiative chain OMBRA can report
// on its own.

fn cielo(dsm: &Raster, strati: &[StratoDiChioma]) -> SkyViewFactor {
    sky_view_factor(dsm, 1.0, strati).expect("il sky view factor del dominio di prova")
}

fn media(campo: &Raster) -> f64 {
    campo.iter().map(|&v| f64::from(v)).sum::<f64>() / campo.len() as f64
}

#[test]
fn un_dominio_piatto_vede_tutto_il_cielo() {
    let visibile = cielo(&Raster::zeros((21, 21)), &[]);
    assert!(
        visibile.senza_chiome.iter().all(|&v| v > 0.99),
        "cielo tolto dal nulla su un dominio piatto: {}",
        visibile.senza_chiome
    );
    // The defence this test exists for: with no vegetation the kernel leaves
    // its `svf_veg` at the zero it was allocated with — it never runs the pass
    // that fills it — and the published combination `svf - (1 - svf_veg) * (1 -
    // psi)` would then subtract a whole hemisphere that no tree is blocking.
    // Without a Strato di chioma the two fields have to be the same field.
    assert_eq!(
        visibile.con_le_chiome, visibile.senza_chiome,
        "senza chiome i due campi devono coincidere"
    );
}

#[test]
fn una_torre_toglie_cielo_alle_celle_ai_suoi_piedi() {
    let lato = 21;
    let c = lato / 2;
    let visibile = cielo(&torre(lato, 0.0), &[]);
    assert!(
        visibile.senza_chiome[[c, c + 1]] < visibile.senza_chiome[[0, 0]] - 0.05,
        "la cella ai piedi della torre vede quanto l'angolo: {} contro {}",
        visibile.senza_chiome[[c, c + 1]],
        visibile.senza_chiome[[0, 0]]
    );
}

#[test]
fn il_cielo_visibile_non_esce_dall_intervallo() {
    let lato = 21;
    let strati = [chioma_al_centro(
        "chiome",
        TRASMISSIVITA_CON_FOGLIE,
        lato,
        10.0,
    )];
    let visibile = cielo(&torre(lato, 0.0), &strati);
    for campo in [&visibile.senza_chiome, &visibile.con_le_chiome] {
        assert!(
            campo.iter().all(|&v| (0.0..=1.0).contains(&v)),
            "un sky view factor fuori da [0, 1]: {campo}"
        );
    }
}

#[test]
fn una_chioma_non_cambia_il_cielo_degli_edifici() {
    // The two fields answer two questions, and mixing them would make the
    // vegetation look like masonry. Buildings and terrain are what
    // `senza_chiome` counts, and a tree is neither.
    let lato = 21;
    let dsm = torre(lato, 0.0);
    let strati = [chioma_al_centro(
        "chiome",
        TRASMISSIVITA_CON_FOGLIE,
        lato,
        10.0,
    )];
    assert_eq!(
        cielo(&dsm, &strati).senza_chiome,
        cielo(&dsm, &[]).senza_chiome,
        "una chioma ha cambiato il cielo che gli edifici lasciano"
    );
}

/// A canopy layer with a square crown of side `2 * raggio + 1` at the centre.
///
/// A crown of one cell is not a tree: the kernel shades the ring where the ray
/// crosses the shell between the trunk top and the canopy top, so a single cell
/// takes no sky from itself and none from its neighbour either. Every test that
/// wants a canopy to take away sky needs a crown wide enough to cover itself.
fn bosco(
    nome: &'static str,
    trasmissivita: f32,
    lato: usize,
    raggio: usize,
    chioma: f32,
) -> StratoDiChioma {
    let c = lato / 2;
    let mut chiome = Raster::from_elem((lato, lato), f32::NAN);
    let mut zona_tronco = Raster::from_elem((lato, lato), f32::NAN);
    for riga in c - raggio..=c + raggio {
        for colonna in c - raggio..=c + raggio {
            chiome[[riga, colonna]] = chioma;
            zona_tronco[[riga, colonna]] = chioma / 2.0;
        }
    }
    StratoDiChioma {
        nome,
        trasmissivita,
        chiome,
        zona_tronco,
    }
}

#[test]
fn una_chioma_ombreggia_a_corona_e_non_sopra_di_se() {
    // The reference implementation's vegetation shadow is a shell, not a solid:
    // a cell is in canopy shade where the ray crosses the crown between the
    // trunk top and the canopy top, and passes freely under the trunk top. A
    // cell directly beneath an isolated crown therefore loses no sky at all,
    // and neither does the cell beside it — the ring starts further out.
    //
    // This is upstream's geometry and not a defect to repair. It is pinned here
    // because it looks like one: the obvious reading of "there is a tree over
    // this cell" is that the cell sees less sky, and on a real crown, which
    // covers many cells, it does. On one cell it does not, and a future reader
    // who finds a zero there needs to know it was measured and expected.
    let lato = 21;
    let c = lato / 2;
    let strati = [chioma_al_centro(
        "chiome",
        TRASMISSIVITA_CON_FOGLIE,
        lato,
        10.0,
    )];
    let visibile = cielo(&Raster::zeros((lato, lato)), &strati);
    let detrazione =
        |d: usize| visibile.senza_chiome[[c, c + d]] - visibile.con_le_chiome[[c, c + d]];
    // The patch weights sum to one to within a few ulp of f32, so "took away
    // nothing" is a tolerance and not a bit pattern.
    assert!(
        detrazione(0) < 1e-6,
        "una chioma isolata si toglie il cielo da sola: {}",
        detrazione(0)
    );
    assert!(
        detrazione(1) < 1e-6,
        "la corona comincia gia' alla cella accanto: {}",
        detrazione(1)
    );
    assert!(
        detrazione(3) > 0.01,
        "la corona non toglie cielo a tre celle: {}",
        detrazione(3)
    );
}

#[test]
fn una_chioma_toglie_cielo_in_proporzione_a_quanto_non_lascia_passare() {
    // `svfbuveg = svf - (1 - svf_veg) * (1 - psi)` is upstream's own formula,
    // and the term it subtracts is linear in `1 - psi`. The same crown with two
    // transmissivities therefore has to take away sky in the ratio of the two
    // opacities — which is what tells a canopy apart from a wall, and a
    // deciduous winter apart from a deciduous summer.
    let lato = 21;
    let c = lato / 2;
    let piatto = Raster::zeros((lato, lato));
    let sotto = |trasmissivita| {
        let visibile = cielo(&piatto, &[bosco("chiome", trasmissivita, lato, 2, 10.0)]);
        f64::from(visibile.senza_chiome[[c, c]] - visibile.con_le_chiome[[c, c]])
    };

    let con_foglie = sotto(TRASMISSIVITA_CON_FOGLIE);
    let senza_foglie = sotto(TRASMISSIVITA_SENZA_FOGLIE);
    assert!(
        con_foglie > 0.05,
        "una chioma sopra la testa non toglie cielo: {con_foglie}"
    );
    let atteso =
        (1.0 - f64::from(TRASMISSIVITA_SENZA_FOGLIE)) / (1.0 - f64::from(TRASMISSIVITA_CON_FOGLIE));
    assert!(
        (senza_foglie / con_foglie - atteso).abs() < 1e-3,
        "il rapporto fra le due detrazioni e' {}, atteso {atteso}",
        senza_foglie / con_foglie
    );
}

#[test]
fn un_secondo_strato_toglie_altro_cielo_e_non_ne_restituisce() {
    // A Periodo invernale carries two layers, evergreen and leafless, and each
    // has to be counted: a second layer that changed nothing would mean the
    // deciduous trees had quietly left the calculation, and one that gave sky
    // back would mean the deductions were being combined the wrong way round.
    let lato = 21;
    let piatto = Raster::zeros((lato, lato));
    let primo = bosco("chiome", TRASMISSIVITA_CON_FOGLIE, lato, 2, 10.0);

    let mut secondo = bosco("spogliate", TRASMISSIVITA_SENZA_FOGLIE, lato, 2, 10.0);
    // Displaced to the north-west corner, so the two crowns do not overlap and
    // the second layer's deduction is its own.
    let mut chiome = Raster::from_elem((lato, lato), f32::NAN);
    let mut zona_tronco = Raster::from_elem((lato, lato), f32::NAN);
    for riga in 1..6 {
        for colonna in 1..6 {
            chiome[[riga, colonna]] = 10.0;
            zona_tronco[[riga, colonna]] = 5.0;
        }
    }
    secondo.chiome = chiome;
    secondo.zona_tronco = zona_tronco;

    let uno = cielo(&piatto, std::slice::from_ref(&primo));
    let due = cielo(&piatto, &[primo, secondo]);
    // With two layers the deduction is rebuilt from three fields, each a sum
    // of patch weights in f32: where the second layer takes nothing the two
    // answers agree to rounding, not to the bit.
    assert!(
        due.con_le_chiome
            .iter()
            .zip(uno.con_le_chiome.iter())
            .all(|(&d, &u)| d <= u + 1e-6),
        "un secondo strato ha restituito cielo invece di toglierne"
    );
    assert!(
        due.con_le_chiome[[3, 3]] < uno.con_le_chiome[[3, 3]] - 0.05,
        "il secondo strato non toglie cielo sotto di se': {} contro {}",
        due.con_le_chiome[[3, 3]],
        uno.con_le_chiome[[3, 3]]
    );
}

#[test]
fn nel_caso_di_riferimento_lo_scenario_piantumato_vede_meno_cielo() {
    let progetto = ombra::progetto::leggi("casi/bastia/progetto").unwrap();
    let periodo = &progetto.periodi[0];
    let campo = |nome: &str| {
        let scenario = progetto
            .scenari
            .iter()
            .find(|s| s.nome == nome)
            .expect("il caso di riferimento porta questo Scenario");
        let raster = ombra::derivazione::deriva(&progetto.griglia, scenario, periodo).unwrap();
        sky_view_factor(
            &raster.modello_di_superficie,
            progetto.griglia.passo_m,
            &raster.strati_di_chioma,
        )
        .unwrap()
    };
    let fatto = campo("stato-di-fatto");
    let interventi = campo("interventi");
    assert_eq!(
        media(&fatto.senza_chiome),
        media(&interventi.senza_chiome),
        "i due Scenari hanno gli stessi edifici: il cielo che lasciano è lo stesso"
    );
    assert!(
        media(&interventi.con_le_chiome) < media(&fatto.con_le_chiome),
        "lo Scenario con più alberi vede più cielo: {} contro {}",
        media(&interventi.con_le_chiome),
        media(&fatto.con_le_chiome)
    );
}

/// The bits of a field folded into one number, in row order: two fields that
/// fold to the same number are, for the purpose of a test, the same bits.
fn bit_ripiegati(campo: &Raster) -> u64 {
    campo.iter().fold(0u64, |h, v| {
        h.wrapping_mul(31).wrapping_add(u64::from(v.to_bits()))
    })
}

#[test]
#[cfg_attr(
    not(all(target_os = "macos", target_arch = "aarch64")),
    ignore = "valori catturati su aarch64-apple-darwin: il nucleo chiama sin, cos e tan della libm di sistema, e su un'altra piattaforma i bit possono differire senza che il calcolo sia cambiato. Forzato con --include-ignored fuori da lì, può fallire per questo soltanto"
)]
fn con_uno_strato_e_con_nessuno_i_bit_sono_quelli_della_0_2_0() {
    // ADR 0006 promises that one canopy layer, and none, compute what they
    // computed at 0.2.0: the third pass, over the Chiome riunite, is for two
    // layers only. The four numbers below were captured on
    // aarch64-apple-darwin from the code at commit `080ddf8`, which is 0.2.0
    // and has no such pass. The test runs on that platform alone: elsewhere
    // it is reported as ignored, never as passed. If this fails after a
    // change to `sky_view_factor`, the single-layer arithmetic has moved, and
    // every summer Corsa already published has moved with it.
    let lato = 31;
    let mut dsm = torre(lato, 0.0);
    for riga in 4..9 {
        for colonna in 20..27 {
            dsm[[riga, colonna]] = 8.0;
        }
    }
    let strato = bosco("chiome", TRASMISSIVITA_CON_FOGLIE, lato, 3, 12.0);

    let nessuno = cielo(&dsm, &[]);
    let uno = cielo(&dsm, std::slice::from_ref(&strato));
    assert_eq!(
        [
            bit_ripiegati(&nessuno.senza_chiome),
            bit_ripiegati(&nessuno.con_le_chiome),
            bit_ripiegati(&uno.senza_chiome),
            bit_ripiegati(&uno.con_le_chiome),
        ],
        [
            15_638_548_197_624_121_658,
            15_638_548_197_624_121_658,
            15_638_548_197_624_121_658,
            8_000_138_042_159_783_221,
        ],
        "i bit dello sky view factor con nessuno Strato o con uno solo non sono più quelli \
         della 0.2.0"
    );
}

// --- Two canopy layers and the Chiome riunite -------------------------------
//
// ADR 0006. The expected means with two layers are the measurements of
// `docs/ricerca/03-chiome-riunite.md`, table M3, row «differenza fra i tre
// campi»: they agree with a patch-by-patch count on the kernel's own bits to
// better than 1e-6. They were measured on aarch64-apple-darwin, and the
// tolerances leave room for a patch or two that another libm might flip.

#[test]
fn nel_caso_di_riferimento_il_cielo_con_le_chiome_ha_i_valori_misurati() {
    // The winter numbers are the ones a sum of the two layers' deductions
    // cannot give: it counts twice the sky both layers cover and lands on
    // 0.706417 and 0.593398, row «somma per Strato» of the same table. The
    // summer ones have a single layer and must not have moved from 0.2.0:
    // they are what the code at commit `080ddf8` gives.
    let progetto = ombra::progetto::leggi("casi/bastia/progetto").unwrap();
    let media_con_le_chiome = |nome: &str, stagione: Stagione| {
        let periodo = progetto
            .periodi
            .iter()
            .find(|periodo| Stagione::da_periodo(periodo) == stagione)
            .expect("il caso di riferimento porta un Periodo per stagione");
        let scenario = progetto
            .scenari
            .iter()
            .find(|s| s.nome == nome)
            .expect("il caso di riferimento porta questo Scenario");
        let raster = ombra::derivazione::deriva(&progetto.griglia, scenario, periodo).unwrap();
        let visibile = sky_view_factor(
            &raster.modello_di_superficie,
            progetto.griglia.passo_m,
            &raster.strati_di_chioma,
        )
        .unwrap();
        media(&visibile.con_le_chiome)
    };
    for (nome, stagione, atteso) in [
        ("stato-di-fatto", Stagione::SenzaFoglie, 0.710361),
        ("interventi", Stagione::SenzaFoglie, 0.607712),
        ("stato-di-fatto", Stagione::ConFoglie, 0.627570),
        ("interventi", Stagione::ConFoglie, 0.429054),
    ] {
        let misurato = media_con_le_chiome(nome, stagione);
        assert!(
            (misurato - atteso).abs() < 5e-5,
            "Scenario «{nome}», {stagione:?}: sky view factor medio con le chiome {misurato:.6}, \
             atteso {atteso}"
        );
    }
}

/// A canopy layer with one crown on rows 18..=22 and the given columns of the
/// 41 x 41 test domains of `docs/ricerca/03-chiome-riunite.md`.
fn blocco(
    nome: &'static str,
    trasmissivita: f32,
    colonne: std::ops::RangeInclusive<usize>,
    chioma: f32,
    tronco: f32,
) -> StratoDiChioma {
    let mut chiome = Raster::from_elem((41, 41), f32::NAN);
    let mut zona_tronco = Raster::from_elem((41, 41), f32::NAN);
    for riga in 18..=22 {
        for colonna in colonne.clone() {
            chiome[[riga, colonna]] = chioma;
            zona_tronco[[riga, colonna]] = tronco;
        }
    }
    StratoDiChioma {
        nome,
        trasmissivita,
        chiome,
        zona_tronco,
    }
}

#[test]
fn due_strati_staccati_non_contano_due_volte_il_cielo_che_coprono_entrambi() {
    // Grid «Sin»: an evergreen crown and a bare one, apart. No cell carries
    // both, and still 330 pairs of cell and patch have both crowns in the
    // way. Such a patch lets through the product of the two transmissivities,
    // and the sum of the two deductions takes it away twice.
    let piatto = Raster::zeros((41, 41));
    let sempreverde = blocco("chiome", TRASMISSIVITA_CON_FOGLIE, 12..=16, 10.0, 3.0);
    let spoglio = blocco(
        "chiome spoglie",
        TRASMISSIVITA_SENZA_FOGLIE,
        24..=28,
        10.0,
        3.0,
    );

    let solo_il_primo = cielo(&piatto, std::slice::from_ref(&sempreverde));
    let solo_il_secondo = cielo(&piatto, std::slice::from_ref(&spoglio));
    let due = cielo(&piatto, &[sempreverde, spoglio]);

    // What 0.2.0 computed, rebuilt from the two single-layer answers.
    let somma_per_strato = ndarray::Zip::from(&due.senza_chiome)
        .and(&solo_il_primo.con_le_chiome)
        .and(&solo_il_secondo.con_le_chiome)
        .map_collect(|&senza, &primo, &secondo| senza - (senza - primo) - (senza - secondo));

    let mut restituito_al_massimo = f32::NEG_INFINITY;
    for ((&con, &senza), &somma) in due
        .con_le_chiome
        .iter()
        .zip(&due.senza_chiome)
        .zip(&somma_per_strato)
    {
        // Never less sky than the double count leaves, to rounding: where no
        // patch has both crowns in the way the two agree. And less than the
        // buildings leave, because every cell of this grid sees both crowns.
        assert!(
            somma - 1e-6 <= con && con < senza,
            "sky view factor con le chiome {con} fuori da [{somma}, {senza})"
        );
        restituito_al_massimo = restituito_al_massimo.max(con - somma);
    }
    assert!(
        (media(&due.con_le_chiome) - 0.945348).abs() < 5e-5,
        "sky view factor medio con le chiome {:.6}, atteso 0.945348: la somma per Strato dà \
         0.944868",
        media(&due.con_le_chiome)
    );
    // How far the sum per layer sits below, where it is furthest. The table
    // gives 0.008214 as its largest distance from the patch-by-patch count,
    // which the new field matches to 1e-6. One patch flipped in that cell
    // moves this by a few thousandths, hence the loose tolerance: the sum
    // itself would read zero.
    assert!(
        (f64::from(restituito_al_massimo) - 0.008214).abs() < 4e-3,
        "la somma per Strato sta sotto di {restituito_al_massimo:.6} al massimo, atteso \
         0.008214"
    );
}

#[test]
fn tre_strati_sono_un_errore_e_non_un_approssimazione() {
    // The difference between three fields tells apart what two layers cover.
    // With a third it would need seven fields, and falling back to the sum
    // would put the double count back without saying so.
    let strati = [
        bosco("chiome", TRASMISSIVITA_CON_FOGLIE, 11, 1, 10.0),
        bosco("chiome spoglie", TRASMISSIVITA_SENZA_FOGLIE, 11, 1, 8.0),
        bosco("un terzo", TRASMISSIVITA_SENZA_FOGLIE, 11, 1, 6.0),
    ];
    let errore = sky_view_factor(&Raster::zeros((11, 11)), 1.0, &strati)
        .expect_err("tre Strati di chioma hanno dato uno sky view factor");
    assert!(
        errore.contains('3'),
        "l'errore non dice quanti Strati sono arrivati: {errore}"
    );
}

#[test]
fn dove_due_strati_condividono_delle_celle_nessuna_quota_di_volta_esce_negativa() {
    // Grid «Sov»: the two crowns overlap on fifteen cells, the bare one taller
    // and with a higher trunk zone. The Chiome riunite keep the tallest top
    // and the lowest trunk zone there. With the rule of one layer — the
    // taller canopy and its own trunk zone — their pass leaves open sky that
    // the evergreen layer covers, and a share of the sky comes out negative
    // in 312 cells (`docs/ricerca/03-chiome-riunite.md`, under table M3).
    let piatto = Raster::zeros((41, 41));
    let sempreverde = blocco("chiome", TRASMISSIVITA_CON_FOGLIE, 16..=20, 10.0, 3.0);
    let spoglio = blocco(
        "chiome spoglie",
        TRASMISSIVITA_SENZA_FOGLIE,
        18..=22,
        14.0,
        8.0,
    );
    let riunite = ChiomeRiunite::da_strati(&[sempreverde.clone(), spoglio.clone()])
        .expect("due Strati di chioma hanno delle Chiome riunite");
    // Opaque, so that its deduction is the sky the Chiome riunite cover.
    let opache = StratoDiChioma {
        nome: "chiome riunite",
        trasmissivita: 0.0,
        chiome: riunite.chiome,
        zona_tronco: riunite.zona_tronco,
    };

    // The sky one layer covers, read back from the public answer: what it
    // takes away, over how little it lets through.
    let volta_tolta = |strato: &StratoDiChioma| {
        let visibile = cielo(&piatto, std::slice::from_ref(strato));
        (&visibile.senza_chiome - &visibile.con_le_chiome) / (1.0 - strato.trasmissivita)
    };
    let del_primo = volta_tolta(&sempreverde);
    let del_secondo = volta_tolta(&spoglio);
    let delle_riunite = volta_tolta(&opache);
    for ((&a, &b), &u) in del_primo.iter().zip(&del_secondo).zip(&delle_riunite) {
        assert!(
            u - b >= -1e-5 && u - a >= -1e-5 && a + b - u >= -1e-5,
            "una quota di volta negativa: solo il primo {}, solo il secondo {}, entrambi {}",
            u - b,
            u - a,
            a + b - u
        );
    }

    let due = cielo(&piatto, &[sempreverde, spoglio]);
    assert!(
        (media(&due.con_le_chiome) - 0.956943).abs() < 5e-5,
        "sky view factor medio con le chiome {:.6}, atteso 0.956943",
        media(&due.con_le_chiome)
    );
}
