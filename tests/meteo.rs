//! The EPW reader. Degenerate inputs first: the file comes from outside the
//! program, and every defence below is one written in `src/meteo.rs`.
//!
//! The reference weather file is not in the repository and never will be — it
//! belongs to a course and is not ours to redistribute — so every test here
//! builds the text it reads. The one test that opens the real file is
//! `#[ignore]`, because a test that quietly passes when its input is missing is
//! worse than no test.

use ombra::dominio::Data;
use ombra::meteo::Epw;
use std::path::Path;

const LUOGO: &str = "LOCATION,Perugia,-,ITA,IGDG,161810,43.08,12.50,1.0,213.0";

/// Seven filler header lines: an EPW has eight before the data, and only the
/// first says anything OMBRA reads.
fn intestazione() -> String {
    let mut testo = String::from(LUOGO);
    for _ in 0..7 {
        testo.push_str("\nRIGA DI INTESTAZIONE,ignorata");
    }
    testo
}

/// One data record. The columns OMBRA reads are placed where the format puts
/// them; everything else is filler.
#[allow(clippy::too_many_arguments)]
fn riga(
    mese: u32,
    giorno: u32,
    ora: u32,
    temperatura: &str,
    umidita: &str,
    globale: &str,
    diretta: &str,
    diffusa: &str,
    direzione_vento: &str,
    vento: &str,
) -> String {
    format!(
        "2005,{mese},{giorno},{ora},0,?9?9,{temperatura},7.2,{umidita},98792,9999,9999,290,\
         {globale},{diretta},{diffusa},999900,999900,999900,99990,{direzione_vento},{vento},\
         1,1,999.0,999,9,999999999,0,0.0000,0,88,999.000,999.0,99.0"
    )
}

fn riga_piena(mese: u32, giorno: u32, ora: u32) -> String {
    riga(
        mese, giorno, ora, "7.6", "97", "120", "300", "80", "45", "4.0",
    )
}

/// A weather file with these records after the header. The reader wants every
/// record to be the hour after the one before it, so they have to be
/// consecutive. It does not ask for a whole year: a few hours are a file.
fn epw(righe: Vec<String>) -> String {
    format!("{}\n{}\n", intestazione(), righe.join("\n"))
}

/// The twenty-four records of one day.
fn giorno_intero(mese: u32, giorno: u32) -> impl Iterator<Item = String> {
    (1..=24).map(move |ora| riga_piena(mese, giorno, ora))
}

/// Every hour of a year from 1 January, with a February of 28 or 29 days.
fn anno(febbraio: u32) -> Vec<String> {
    [31, febbraio, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        .into_iter()
        .zip(1..)
        .flat_map(|(giorni, mese)| (1..=giorni).flat_map(move |giorno| giorno_intero(mese, giorno)))
        .collect()
}

/// A record with the month, day and hour fields written as given, numbers or
/// not. `riga` only writes whole numbers there.
fn con_data(mese: &str, giorno: &str, ora: &str) -> String {
    riga_piena(1, 1, 1).replacen("2005,1,1,1,", &format!("2005,{mese},{giorno},{ora},"), 1)
}

fn leggi(testo: &str) -> Result<Epw, ombra::meteo::MeteoError> {
    Epw::da_testo(Path::new("prova.epw"), testo)
}

fn data(mese: u32, giorno: u32) -> Data {
    Data {
        anno: 2021,
        mese,
        giorno,
    }
}

#[test]
fn il_fuso_orario_viene_dal_file_e_non_dalla_longitudine() {
    // The whole reason this module was written before the radiation: 12.50
    // degrees east divided by fifteen rounds to 1, and it would have been right
    // here by luck. The file states the zone, so nothing has to be lucky.
    let f = leggi(&epw(vec![riga_piena(1, 1, 1)])).unwrap();
    assert_eq!(f.fuso_ore, 1.0);
    assert_eq!(f.latitudine_gradi, 43.08);
    assert_eq!(f.longitudine_gradi, 12.50);
    assert_eq!(f.quota_m, 213.0);
    assert_eq!(f.stazione, "Perugia");
}

#[test]
fn un_fuso_che_non_e_un_multiplo_dell_ora_si_legge_come_sta_scritto() {
    // India is at +5.5 and Nepal at +5.75. Rounding a zone to the whole hour is
    // a defect the longitude guess had and the file does not.
    let testo =
        epw(vec![riga_piena(1, 1, 1)]).replace("43.08,12.50,1.0,213.0", "27.70,85.33,5.75,1300.0");
    assert_eq!(leggi(&testo).unwrap().fuso_ore, 5.75);
}

#[test]
fn le_colonne_lette_sono_quelle_giuste() {
    // A whole day, because a Periodo starts at hour 1 of its first day: the
    // record looked at is the thirteenth, and it is the only one with these
    // values.
    let mut righe: Vec<String> = giorno_intero(7, 15).collect();
    righe[12] = riga(7, 15, 13, "31.4", "42", "880", "790", "160", "225", "3.5");
    let f = leggi(&epw(righe)).unwrap();
    let ora = f.ore_dal(data(7, 15), 13).unwrap()[12];
    assert_eq!(ora.mese, 7);
    assert_eq!(ora.giorno, 15);
    assert_eq!(ora.ora, 13);
    assert_eq!(ora.temperatura_c, Some(31.4));
    assert_eq!(ora.umidita_relativa, Some(42.0));
    assert_eq!(ora.globale_orizzontale_wm2, Some(880.0));
    assert_eq!(ora.diretta_normale_wm2, Some(790.0));
    assert_eq!(ora.diffusa_orizzontale_wm2, Some(160.0));
    assert_eq!(ora.direzione_vento_gradi, Some(225.0));
    assert_eq!(ora.vento_ms, Some(3.5));
}

#[test]
fn un_valore_assente_non_diventa_un_valore_misurato() {
    // 99.9 degrees, 999 per cent, 9999 W/m2 and 999 m/s are what an EPW writes
    // when nobody measured. Every one of them is inside the range a reader
    // would otherwise accept, and 9999 W/m2 summed over a day would be a
    // radiation nobody has ever seen.
    let f = leggi(&epw(vec![riga(
        1, 1, 1, "99.9", "999", "9999", "9999", "9999", "999", "999",
    )]))
    .unwrap();
    let ora = f.ore_dal(data(1, 1), 1).unwrap()[0];
    assert_eq!(ora.temperatura_c, None);
    assert_eq!(ora.umidita_relativa, None);
    assert_eq!(ora.globale_orizzontale_wm2, None);
    assert_eq!(ora.diretta_normale_wm2, None);
    assert_eq!(ora.diffusa_orizzontale_wm2, None);
    assert_eq!(ora.vento_ms, None);
    assert_eq!(ora.direzione_vento_gradi, None);
}

#[test]
fn una_temperatura_di_novantanove_e_nove_e_assente_ma_una_di_novantanove_e_otto_no() {
    // The sentinel is one number, not a range: a reader that treated everything
    // above a threshold as missing would throw away a real measurement.
    let f = leggi(&epw(vec![riga(
        1, 1, 1, "99.8", "97", "0", "0", "0", "45", "4.0",
    )]))
    .unwrap();
    assert_eq!(
        f.ore_dal(data(1, 1), 1).unwrap()[0].temperatura_c,
        Some(99.8)
    );
}

#[test]
fn le_ore_di_un_periodo_escono_in_ordine_dalla_data_di_inizio() {
    let righe: Vec<String> = (1..=24)
        .map(|ora| riga_piena(1, 1, ora))
        .chain((1..=24).map(|ora| riga_piena(1, 2, ora)))
        .collect();
    let f = leggi(&epw(righe)).unwrap();
    let ore = f.ore_dal(data(1, 2), 3).unwrap();
    assert_eq!(
        ore.iter().map(|v| (v.giorno, v.ora)).collect::<Vec<_>>(),
        vec![(2, 1), (2, 2), (2, 3)]
    );
}

#[test]
fn un_periodo_che_supera_la_fine_dell_anno_torna_alla_prima_ora() {
    // An EPW is a typical year and has no next year to read. The hour after the
    // last is the first, which is a modelling claim and not an accident: the
    // Corsa has to be able to say it happened.
    let righe: Vec<String> = (1..=24)
        .map(|ora| riga_piena(12, 31, ora))
        .chain((1..=24).map(|ora| riga_piena(1, 1, ora)))
        .collect();
    let f = leggi(&epw(righe)).unwrap();
    assert!(!f.si_avvolge(data(12, 31), 48));
    assert!(f.si_avvolge(data(1, 1), 25));

    let ore = f.ore_dal(data(1, 1), 26).unwrap();
    assert_eq!(ore.len(), 26);
    // Hour 25 of a Periodo starting on the last day of the file is hour 1 of
    // its first day.
    assert_eq!((ore[24].mese, ore[24].giorno, ore[24].ora), (12, 31, 1));
    assert_eq!((ore[25].mese, ore[25].giorno, ore[25].ora), (12, 31, 2));
}

#[test]
fn un_giorno_che_il_file_non_ha_e_un_errore_e_non_un_giorno_vicino() {
    // 29 February is the case that brings anyone here: a typical year has no
    // leap day. Silently serving 28 February or 1 March would move a whole
    // Corsa by a day without saying so.
    let f = leggi(&epw(giorno_intero(2, 28)
        .chain(giorno_intero(3, 1))
        .collect()))
    .unwrap();
    let errore = f.ore_dal(data(2, 29), 1).unwrap_err();
    // The file is hourly: it is the Periodo that asks for a day it lacks.
    assert!(!errore.non_orario());
    let errore = errore.to_string();
    assert!(errore.contains("29/2"), "{errore}");
    assert!(errore.contains("29 febbraio"), "{errore}");

    // The same typical year serves a Periodo across the day it does not have:
    // the hour after the last of 28 February is the first of 1 March.
    let ore = f.ore_dal(data(2, 28), 48).unwrap();
    assert_eq!((ore[24].mese, ore[24].giorno, ore[24].ora), (3, 1, 1));
}

#[test]
fn un_file_senza_riga_location_e_rifiutato() {
    let testo = epw(vec![riga_piena(1, 1, 1)]).replace("LOCATION", "POSIZIONE");
    let errore = leggi(&testo).unwrap_err().to_string();
    assert!(errore.contains("fuso orario"), "{errore}");
}

#[test]
fn una_riga_location_troppo_corta_e_rifiutata() {
    // Nine fields instead of ten: the elevation is missing, and with one field
    // gone every index after it reads the wrong column. Refusing beats reading
    // the longitude as a time zone.
    let testo = epw(vec![riga_piena(1, 1, 1)])
        .replace(LUOGO, "LOCATION,Perugia,-,ITA,IGDG,161810,43.08,12.50,1.0");
    assert!(leggi(&testo).is_err());
}

#[test]
fn un_fuso_che_non_e_un_numero_e_un_errore_leggibile() {
    let testo = epw(vec![riga_piena(1, 1, 1)]).replace(",1.0,213.0", ",CET,213.0");
    let errore = leggi(&testo).unwrap_err().to_string();
    assert!(
        errore.contains("CET") && errore.contains("fuso orario"),
        "{errore}"
    );
}

#[test]
fn una_riga_di_dati_troppo_corta_dice_quale() {
    let testo = format!("{}\n2005,1,1,1,0,?9?9,7.6,7.2,97\n", intestazione());
    let errore = leggi(&testo).unwrap_err().to_string();
    assert!(errore.contains("riga 9"), "{errore}");
    assert!(errore.contains("9 campi"), "{errore}");
}

#[test]
fn un_campo_presente_che_non_e_un_numero_ferma_la_lettura() {
    // Missing is `None`; unreadable is an error. Confusing the two would let a
    // corrupt file pass as a file with holes in it.
    let testo = epw(vec![riga(
        1, 1, 1, "sette", "97", "0", "0", "0", "45", "4.0",
    )]);
    let errore = leggi(&testo).unwrap_err().to_string();
    assert!(errore.contains("sette"), "{errore}");
    assert!(errore.contains("temperatura"), "{errore}");
}

#[test]
fn un_file_di_sola_intestazione_non_ha_ore() {
    let errore = leggi(&format!("{}\n", intestazione()))
        .unwrap_err()
        .to_string();
    assert!(errore.contains("nessuna riga di dati"), "{errore}");
}

#[test]
fn le_righe_vuote_in_coda_non_sono_ore() {
    let f = leggi(&format!("{}\n\n\n", epw(vec![riga_piena(1, 1, 1)]))).unwrap();
    assert_eq!(f.ore_totali(), 1);
}

#[test]
fn un_file_che_non_esiste_dice_quale() {
    let errore = Epw::leggi(Path::new("questo-file-non-esiste.epw"))
        .unwrap_err()
        .to_string();
    assert!(errore.contains("questo-file-non-esiste.epw"), "{errore}");
}

/// The same record with another minute field. `riga` writes a zero there.
fn con_minuto(riga: String, minuto: &str) -> String {
    riga.replacen(",0,?9?9", &format!(",{minuto},?9?9"), 1)
}

#[test]
fn un_file_con_il_minuto_a_sessanta_su_tutte_le_righe_e_orario() {
    // The EPW dictionary gives the minute as 1..60, so an hourly file written
    // to the letter carries 60, and files in the wild write 0. The hour comes
    // from the hour field either way, so OMBRA takes both as hourly.
    let sessanta: Vec<String> = (1..=24)
        .map(|ora| con_minuto(riga_piena(1, 1, ora), "60"))
        .collect();
    let f = leggi(&epw(sessanta)).unwrap();
    assert_eq!(f.ore_totali(), 24);
    assert_eq!(f.minuti, vec![60]);
    assert_eq!(f.ore_dal(data(1, 1), 1).unwrap()[0].ora, 1);

    let zero: Vec<String> = (1..=24).map(|ora| riga_piena(1, 1, ora)).collect();
    assert_eq!(leggi(&epw(zero)).unwrap().minuti, vec![0]);
}

#[test]
fn un_file_che_scrive_zero_su_un_ora_e_sessanta_su_un_altra_dice_tutti_e_due() {
    // Every record is still one hour, so the file is hourly. The Giornale gets
    // both values.
    let f = leggi(&epw(vec![
        con_minuto(riga_piena(1, 1, 1), "60"),
        riga_piena(1, 1, 2),
        con_minuto(riga_piena(1, 1, 3), "60"),
    ]))
    .unwrap();
    assert_eq!(f.minuti, vec![0, 60]);
}

#[test]
fn un_minuto_che_non_e_ne_zero_ne_sessanta_e_rifiutato() {
    // A record at minute 30 is half of an hour, or an instant: either way it is
    // not the hourly mean the sun of an Ora is centred on, and a reader that
    // looked at the hour field alone would take it for one.
    let errore = leggi(&epw(vec![
        riga_piena(1, 1, 1),
        con_minuto(riga_piena(1, 1, 2), "30"),
    ]))
    .unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::MinutoNonOrario { .. }),
        "variante: {errore:?}"
    );
    assert!(errore.non_orario());
    let errore = errore.to_string();
    assert!(errore.contains("riga 10"), "{errore}");
    assert!(errore.contains("«30»"), "{errore}");
}

#[test]
fn un_minuto_che_non_e_esattamente_zero_o_sessanta_non_passa() {
    // Truncating before comparing would let 0.5 through as 0 and 60.9 as 60,
    // and a cast would turn NaN into 0. A field that is empty or is not a
    // number is not 0 or 60 either: it stops the Corsa like a 30 does, instead
    // of making the file unreadable, which the Corsa survives.
    for minuto in [
        "0.5", "60.9", "-60", "15", "45", "NaN", "inf", "", " ", "x", "30x",
    ] {
        let esito = leggi(&epw(vec![con_minuto(riga_piena(1, 1, 1), minuto)]));
        match esito {
            Err(errore @ ombra::meteo::MeteoError::MinutoNonOrario { .. }) => {
                assert!(errore.non_orario());
                assert!(errore.to_string().contains("riga 9"), "{errore}");
            }
            altro => panic!("il minuto «{minuto}» è passato: {altro:?}"),
        }
    }
    // Minus zero is zero, and sixty is sixty however it is spelled.
    for (minuto, atteso) in [("-0", 0), ("60.0", 60), (" 60 ", 60), ("6e1", 60)] {
        let f = leggi(&epw(vec![con_minuto(riga_piena(1, 1, 1), minuto)])).unwrap();
        assert_eq!(f.minuti, vec![atteso], "minuto «{minuto}»");
    }
}

#[test]
fn il_minuto_rifiutato_arriva_nel_messaggio_corto_e_senza_caratteri_di_controllo() {
    // The message is printed on a terminal and written into every Giornale of
    // the Progetto, and the file may come from somebody else: a field of a
    // million digits is a number to the parser, and an escape sequence would
    // rewrite the line that says the Corsa failed.
    let lungo = "9".repeat(1_000_000);
    let errore = leggi(&epw(vec![con_minuto(riga_piena(1, 1, 1), &lungo)]))
        .unwrap_err()
        .to_string();
    assert!(errore.chars().count() < 400, "{} caratteri", errore.len());
    assert!(errore.contains('…'), "il taglio va dichiarato: {errore}");

    let errore = leggi(&epw(vec![con_minuto(
        riga_piena(1, 1, 1),
        "30\u{1b}[2K\u{7}",
    )]))
    .unwrap_err()
    .to_string();
    assert!(!errore.chars().any(char::is_control), "{errore:?}");
    // Shown as U+FFFD and not dropped: the reader of the message sees that
    // something was there.
    assert!(errore.contains("«30\u{fffd}[2K\u{fffd}»"), "{errore}");

    // Unicode has characters that are not control characters and still
    // reorder or hide the text around them: right-to-left override, isolates,
    // zero width space, line separator, byte order mark.
    for nascosto in ["\u{202e}", "\u{2066}", "\u{200b}", "\u{2028}", "\u{feff}"] {
        let errore = leggi(&epw(vec![con_minuto(
            riga_piena(1, 1, 1),
            &format!("3{nascosto}0"),
        )]))
        .unwrap_err()
        .to_string();
        assert!(!errore.contains(nascosto), "{errore:?}");
        assert!(errore.contains("«3\u{fffd}0»"), "{errore}");
    }
}

/// The message a minute field that is not 0 or 60 gives, which quotes it.
fn minuto_citato(minuto: &str) -> String {
    leggi(&epw(vec![con_minuto(riga_piena(1, 1, 1), minuto)]))
        .unwrap_err()
        .to_string()
}

#[test]
fn il_campo_citato_e_tagliato_a_trentadue_caratteri() {
    let errore = minuto_citato(&"9".repeat(33));
    assert!(
        errore.contains(&format!("«{}…»", "9".repeat(32))),
        "{errore}"
    );
    // Thirty-two fit, and nothing says they were cut.
    let errore = minuto_citato(&"9".repeat(32));
    assert!(
        errore.contains(&format!("«{}»", "9".repeat(32))),
        "{errore}"
    );
}

#[test]
fn una_lettera_che_non_e_ascii_non_arriva_nel_messaggio() {
    // A letter, not a format character: printable ASCII is all that passes.
    let errore = minuto_citato("3\u{5d0}0");
    assert!(errore.contains("«3\u{fffd}0»"), "{errore}");
}

#[test]
fn il_campo_citato_perde_gli_spazi_ai_bordi_e_tiene_quelli_in_mezzo() {
    let errore = minuto_citato(" 3 0 ");
    assert!(errore.contains("«3 0»"), "{errore}");
}

#[test]
fn due_record_per_la_stessa_ora_sono_rifiutati() {
    // What a sub-hourly file looks like once its minutes are 0 or 60, and what
    // a file with a pasted line looks like. `ore_dal` counts records and calls
    // them hours: with two records for one hour every hour after it would be
    // the wrong one.
    for secondo in ["0", "60"] {
        let errore = leggi(&epw(vec![
            riga_piena(7, 15, 13),
            con_minuto(riga_piena(7, 15, 13), secondo),
            riga_piena(7, 15, 14),
        ]))
        .unwrap_err();
        assert!(
            matches!(errore, ombra::meteo::MeteoError::OraRipetuta { .. }),
            "variante: {errore:?}"
        );
        assert!(errore.non_orario());
        let errore = errore.to_string();
        assert!(errore.contains("riga 10"), "{errore}");
        assert!(errore.contains("15/7"), "{errore}");
        assert!(errore.contains("ora 13"), "{errore}");
        assert!(errore.contains("alla riga 9"), "{errore}");
    }
}

#[test]
fn un_ora_ripetuta_lontano_dalla_prima_e_rifiutata_lo_stesso() {
    // A day pasted twice repeats nothing next to itself: record 25 follows
    // record 24, and a check against the record before would let it through.
    let giorno = || (1..=24).map(|ora| riga_piena(7, 15, ora));
    let errore = leggi(&epw(giorno().chain(giorno()).collect())).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::OraRipetuta { .. }),
        "variante: {errore:?}"
    );
    let errore = errore.to_string();
    // Eight header lines, twenty-four records, and the twenty-fifth.
    assert!(errore.contains("riga 33"), "{errore}");
    assert!(errore.contains("alla riga 9"), "{errore}");
}

#[test]
fn la_stessa_ora_di_un_altro_anno_non_e_un_ora_ripetuta() {
    // A file of two years has one record per hour and is hourly. The year is
    // read as the file writes it, so this stays true of a typical year made of
    // months taken from different years. Two whole years, because the same
    // hour comes back only after every other hour of the year has gone by.
    let secondo = || {
        anno(28)
            .into_iter()
            .map(|riga| riga.replacen("2005,", "2006,", 1))
    };
    let f = leggi(&epw(anno(28).into_iter().chain(secondo()).collect())).unwrap();
    assert_eq!(f.ore_totali(), 2 * 8760);

    // The same two years with the same year written on both: every hour of the
    // second is a repeat, and the first of them is the one reported.
    let errore = leggi(&epw(anno(28).into_iter().chain(anno(28)).collect())).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::OraRipetuta { .. }),
        "variante: {errore:?}"
    );
    let errore = errore.to_string();
    assert!(errore.contains("riga 8769"), "{errore}");
    assert!(errore.contains("alla riga 9"), "{errore}");
}

/// The error a month, day or hour outside the calendar has to give, on the
/// one data line of the file.
fn fuori_calendario(mese: &str, giorno: &str, ora: &str) -> String {
    match leggi(&epw(vec![con_data(mese, giorno, ora)])) {
        Err(errore @ ombra::meteo::MeteoError::FuoriCalendario { .. }) => {
            assert!(errore.non_orario(), "{errore}");
            let errore = errore.to_string();
            assert!(errore.contains("riga 9"), "{errore}");
            errore
        }
        altro => panic!("«{mese}», «{giorno}», «{ora}» è passato: {altro:?}"),
    }
}

#[test]
fn un_mese_un_giorno_o_un_ora_con_la_frazione_non_sono_troncati() {
    // 13.5 is not hour 13. Truncated, it would be read as hour 13 and nobody
    // would be told.
    let errore = fuori_calendario("1", "1", "13.5");
    assert!(errore.contains("l'ora è «13.5»"), "{errore}");
    assert!(errore.contains("da 1 a 24"), "{errore}");

    let errore = fuori_calendario("1", "1.5", "1");
    assert!(errore.contains("il giorno è «1.5»"), "{errore}");
    assert!(errore.contains("da 1 a 31"), "{errore}");

    let errore = fuori_calendario("1.5", "1", "1");
    assert!(errore.contains("il mese è «1.5»"), "{errore}");
    assert!(errore.contains("da 1 a 12"), "{errore}");
}

#[test]
fn un_ora_si_conta_da_uno_a_ventiquattro() {
    // A file that numbers its hours 0 to 23 is not an EPW: read as one, every
    // record would land an hour away from where it belongs.
    for ora in ["0", "25"] {
        let errore = fuori_calendario("1", "1", ora);
        assert!(errore.contains(&format!("l'ora è «{ora}»")), "{errore}");
        assert!(errore.contains("da 1 a 24"), "{errore}");
    }
    for ora in ["1", "24"] {
        assert!(leggi(&epw(vec![con_data("1", "1", ora)])).is_ok(), "{ora}");
    }
}

#[test]
fn un_mese_o_un_giorno_che_il_calendario_non_ha_sono_rifiutati() {
    for mese in ["0", "13"] {
        let errore = fuori_calendario(mese, "1", "1");
        assert!(errore.contains(&format!("il mese è «{mese}»")), "{errore}");
        assert!(errore.contains("da 1 a 12"), "{errore}");
    }
    // The last day is the one of that month.
    for (mese, giorno, ultimo) in [
        ("1", "0", 31),
        ("1", "32", 31),
        ("4", "31", 30),
        ("2", "30", 29),
    ] {
        let errore = fuori_calendario(mese, giorno, "1");
        assert!(
            errore.contains(&format!("il giorno è «{giorno}»")),
            "{errore}"
        );
        assert!(errore.contains(&format!("da 1 a {ultimo}")), "{errore}");
    }
    // 29 February is in the file of a leap year. The year of a typical year is
    // a different one month by month, so the reader does not ask which it is.
    let f = leggi(&epw(vec![con_data("2", "29", "1")])).unwrap();
    assert_eq!(f.ore_dal(data(2, 29), 1).unwrap()[0].giorno, 29);
}

#[test]
fn un_mese_un_giorno_o_un_ora_che_non_sono_un_numero_del_calendario_fermano_la_corsa() {
    // An `as u32` cast would turn a negative number and NaN into 0 and infinity
    // into `u32::MAX`. A field that is empty or is not a number is not in the
    // calendar either: it stops the Corsa as a fraction does, instead of
    // making the file unreadable, which the Corsa survives.
    for testo in ["-1", "-13", "NaN", "inf", "-inf", "", " ", "x", "13x"] {
        let errore = fuori_calendario(testo, "1", "1");
        assert!(errore.contains("il mese è «"), "«{testo}»: {errore}");
        let errore = fuori_calendario("1", testo, "1");
        assert!(errore.contains("il giorno è «"), "«{testo}»: {errore}");
        let errore = fuori_calendario("1", "1", testo);
        assert!(errore.contains("l'ora è «"), "«{testo}»: {errore}");
    }
}

#[test]
fn un_intero_passa_comunque_sia_scritto() {
    let mut righe: Vec<String> = giorno_intero(1, 1).collect();
    for ora in ["13.0", " 13 ", "1.3e1"] {
        righe[12] = con_data("1.0", " 1 ", ora);
        let f = leggi(&epw(righe.clone())).unwrap();
        let tredicesima = f.ore_dal(data(1, 1), 13).unwrap()[12];
        assert_eq!(
            (tredicesima.mese, tredicesima.giorno, tredicesima.ora),
            (1, 1, 13),
            "ora «{ora}»"
        );
    }
}

#[test]
fn il_campo_fuori_calendario_arriva_nel_messaggio_corto_e_senza_caratteri_di_controllo() {
    // As for the minute: a million digits are a number to the parser, and the
    // message goes to a terminal and into every Giornale.
    let lungo = "9".repeat(1_000_000);
    for errore in [
        fuori_calendario(&lungo, "1", "1"),
        fuori_calendario("1", &lungo, "1"),
        fuori_calendario("1", "1", &lungo),
    ] {
        assert!(errore.chars().count() < 400, "{} caratteri", errore.len());
        assert!(errore.contains('…'), "il taglio va dichiarato: {errore}");
    }

    let errore = fuori_calendario("1", "1", "25\u{1b}[2K\u{7}");
    assert!(!errore.chars().any(char::is_control), "{errore:?}");
    assert!(errore.contains("«25\u{fffd}[2K\u{fffd}»"), "{errore}");
}

/// The error a record that is not the hour after the one before it has to
/// give.
fn fuori_posto(righe: Vec<String>) -> String {
    match leggi(&epw(righe)) {
        Err(errore @ ombra::meteo::MeteoError::OraFuoriPosto { .. }) => {
            assert!(errore.non_orario(), "{errore}");
            errore.to_string()
        }
        altro => panic!("il file è passato: {altro:?}"),
    }
}

#[test]
fn un_ora_che_manca_dentro_il_giorno_ferma_la_lettura() {
    // `ore_dal` counts records and calls them hours: after a missing one every
    // Ora would get the record of the hour after its own.
    let errore = fuori_posto(vec![
        riga_piena(1, 1, 1),
        riga_piena(1, 1, 2),
        riga_piena(1, 1, 4),
    ]);
    assert!(errore.contains("riga 11"), "{errore}");
    // One string and not two: nothing may slip in between what goes there and
    // what is there, least of all the 1 March that only February allows.
    assert!(
        errore.contains("ci va il 1/1, ora 3, e c'è il 1/1, ora 4"),
        "{errore}"
    );
}

#[test]
fn un_buco_ferma_la_corsa_anche_se_la_riga_ha_un_campo_che_non_si_legge() {
    // The order of the checks is what the Corsa sees: a missing hour stops
    // it, a temperature that is not a number does not. On a line with both,
    // the one that stops it has to be the one reported.
    let errore = leggi(&epw(vec![
        riga_piena(1, 1, 1),
        riga(1, 1, 3, "sette", "97", "0", "0", "0", "45", "4.0"),
    ]))
    .unwrap_err();
    assert!(errore.non_orario(), "{errore}");
}

#[test]
fn un_minuto_sbagliato_si_dice_prima_dell_ora_fuori_posto() {
    let errore = leggi(&epw(vec![
        riga_piena(1, 1, 1),
        con_minuto(riga_piena(1, 1, 3), "30"),
    ]))
    .unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::MinutoNonOrario { .. }),
        "variante: {errore:?}"
    );
}

#[test]
fn un_ora_che_manca_a_cavallo_di_mezzanotte_ferma_la_lettura() {
    for (prima, dopo, attesa) in [
        // Hour 24 is missing.
        ((1, 1, 23), (1, 2, 1), "ci va il 1/1, ora 24"),
        // Hour 1 of the day after is missing.
        ((1, 1, 24), (1, 2, 2), "ci va il 2/1, ora 1"),
        // A whole day is missing.
        ((1, 1, 24), (1, 3, 1), "ci va il 2/1, ora 1"),
        // And so is the last day of a month.
        ((1, 30, 24), (2, 1, 1), "ci va il 31/1, ora 1"),
        // The whole file from the last hour of the year to the second day.
        ((12, 31, 24), (1, 2, 1), "ci va il 1/1, ora 1"),
    ] {
        let errore = fuori_posto(vec![
            riga_piena(prima.0, prima.1, prima.2),
            riga_piena(dopo.0, dopo.1, dopo.2),
        ]);
        assert!(errore.contains("riga 10"), "{errore}");
        assert!(errore.contains(attesa), "{errore}");
        assert!(
            errore.contains(&format!("c'è il {}/{}, ora {}", dopo.1, dopo.0, dopo.2)),
            "{errore}"
        );
    }
}

#[test]
fn due_record_scambiati_di_posto_fermano_la_lettura() {
    let errore = fuori_posto(vec![
        riga_piena(1, 1, 1),
        riga_piena(1, 1, 3),
        riga_piena(1, 1, 2),
        riga_piena(1, 1, 4),
    ]);
    assert!(errore.contains("riga 10"), "{errore}");
    assert!(errore.contains("ci va il 1/1, ora 2"), "{errore}");
    assert!(errore.contains("c'è il 1/1, ora 3"), "{errore}");
}

#[test]
fn la_fine_di_un_mese_e_la_fine_dell_anno_non_sono_un_buco() {
    for (prima, dopo) in [
        ((1, 31), (2, 1)),
        ((4, 30), (5, 1)),
        // A typical year has no leap day, the file of a leap year has it.
        ((2, 28), (3, 1)),
        ((2, 28), (2, 29)),
        ((2, 29), (3, 1)),
        // A file of several years, and a typical year that goes round.
        ((12, 31), (1, 1)),
    ] {
        let f = leggi(&epw(vec![
            riga_piena(prima.0, prima.1, 24),
            riga_piena(dopo.0, dopo.1, 1),
        ]));
        assert!(f.is_ok(), "{prima:?} → {dopo:?}: {f:?}");
    }
    // The year is not part of the sequence: the months of a typical year come
    // from different years, and January may carry an earlier one than December.
    let f = leggi(&epw(vec![
        riga_piena(12, 31, 24),
        riga_piena(1, 1, 1).replacen("2005,", "1999,", 1),
    ]));
    assert!(f.is_ok(), "{f:?}");
}

#[test]
fn dopo_il_ventotto_febbraio_il_messaggio_nomina_tutti_e_due_i_giorni_ammessi() {
    let errore = fuori_posto(vec![riga_piena(2, 28, 24), riga_piena(3, 2, 1)]);
    assert!(
        errore.contains("ci va il 29/2, ora 1 (o il 1/3, ora 1), e c'è il 2/3, ora 1"),
        "{errore}"
    );
    // Only the first hour of 1 March: the second is an hour lost.
    let errore = fuori_posto(vec![riga_piena(2, 28, 24), riga_piena(3, 1, 2)]);
    assert!(
        errore.contains("ci va il 29/2, ora 1 (o il 1/3, ora 1), e c'è il 1/3, ora 2"),
        "{errore}"
    );
}

#[test]
fn un_febbraio_che_manca_non_passa_per_l_eccezione_del_ventotto() {
    // 1 March is the hour after the last of 28 February and of nothing else:
    // not of the 27th, and not of a January with no February after it.
    let errore = fuori_posto(vec![riga_piena(2, 27, 24), riga_piena(3, 1, 1)]);
    assert!(
        errore.contains("ci va il 28/2, ora 1, e c'è il 1/3, ora 1"),
        "{errore}"
    );
    let errore = fuori_posto(vec![riga_piena(1, 31, 24), riga_piena(3, 1, 1)]);
    assert!(
        errore.contains("ci va il 1/2, ora 1, e c'è il 1/3, ora 1"),
        "{errore}"
    );
}

#[test]
fn un_anno_intero_si_legge() {
    // The file every Corsa is run on: 8760 consecutive hours, and 8784 for the
    // measured year that has a leap day.
    assert_eq!(leggi(&epw(anno(28))).unwrap().ore_totali(), 8760);
    assert_eq!(leggi(&epw(anno(29))).unwrap().ore_totali(), 8784);
}

#[test]
fn un_periodo_su_un_file_che_comincia_a_meta_giornata_e_un_errore() {
    // The file reads: it is hourly, and its second day is whole. A Periodo on
    // its first day would count hour 13 as Ora 0.
    // 15 July and not 1 January: a day and a month that differ, so the message
    // cannot have them the wrong way round.
    let f = leggi(&epw((13..=24)
        .map(|ora| riga_piena(7, 15, ora))
        .chain(giorno_intero(7, 16))
        .collect()))
    .unwrap();
    let errore = f.ore_dal(data(7, 15), 24).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::GiornoMonco { .. }),
        "variante: {errore:?}"
    );
    // The file is hourly: it is this Periodo that does not fit in it.
    assert!(!errore.non_orario());
    let errore = errore.to_string();
    assert!(errore.contains("chiede il 15/7, ora 1"), "{errore}");
    assert!(errore.contains("porta il 15/7, ora 13"), "{errore}");

    assert_eq!(f.ore_dal(data(7, 16), 24).unwrap().len(), 24);
}

#[test]
fn due_mezze_giornate_ai_due_capi_del_file_non_fanno_un_giorno() {
    // A file cut from noon to noon. A Periodo on its last day runs off the end
    // at hour 12 and comes back at hour 13 of the first day: the hours are in
    // a row, and the day is made of two halves two days apart.
    let f = leggi(&epw((13..=24)
        .map(|ora| riga_piena(7, 15, ora))
        .chain(giorno_intero(7, 16))
        .chain((1..=12).map(|ora| riga_piena(7, 17, ora)))
        .collect()))
    .unwrap();
    let errore = f.ore_dal(data(7, 17), 24).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::GiornoMonco { .. }),
        "variante: {errore:?}"
    );
    // Back at the top of the file a day has to begin, and the message says
    // which: hour 13 after hour 12 would read as nothing wrong.
    let errore = errore.to_string();
    assert!(errore.contains("chiede il 15/7, ora 1"), "{errore}");
    assert!(errore.contains("porta il 15/7, ora 13"), "{errore}");

    // The whole day in the middle is still a day.
    assert_eq!(f.ore_dal(data(7, 16), 24).unwrap().len(), 24);
}

#[test]
fn un_anno_che_comincia_dall_ora_due_non_ha_un_primo_gennaio_intero() {
    // A year turned by one record: it opens at hour 2 of 1 January and closes
    // at hour 1 of the same day. The hours are in a row across the wrap, and
    // on the same day, and the file still opens in the middle of a day.
    let mut righe = anno(28);
    righe.rotate_left(1);
    let f = leggi(&epw(righe)).unwrap();
    let errore = f.ore_dal(data(1, 1), 24).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::GiornoMonco { .. }),
        "variante: {errore:?}"
    );
    let errore = errore.to_string();
    assert!(
        errore.contains("chiede il 1/1, ora 1, e il file a quel punto porta il 1/1, ora 2"),
        "{errore}"
    );
}

#[test]
fn un_periodo_piu_lungo_del_file_fa_piu_di_un_giro() {
    let f = leggi(&epw(giorno_intero(7, 15).collect())).unwrap();
    let ore = f.ore_dal(data(7, 15), 49).unwrap();
    assert_eq!(ore.len(), 49);
    assert_eq!(ore[48].ora, 1);
}

#[test]
fn un_periodo_su_giorni_interi_di_un_file_ritagliato_ha_le_ore_in_fila() {
    let f = leggi(&epw(giorno_intero(7, 15)
        .chain(giorno_intero(7, 16))
        .collect()))
    .unwrap();
    let ore = f.ore_dal(data(7, 15), 48).unwrap();
    for (passo, valori) in ore.iter().enumerate() {
        assert_eq!(valori.ora as usize, passo % 24 + 1, "passo {passo}");
    }
}

#[test]
fn un_periodo_che_si_avvolge_su_un_file_che_finisce_a_meta_giornata_e_un_errore() {
    let f = leggi(&epw(giorno_intero(7, 15)
        .chain((1..=12).map(|ora| riga_piena(7, 16, ora)))
        .collect()))
    .unwrap();
    // Up to the last record there is nothing to say.
    assert_eq!(f.ore_dal(data(7, 15), 36).unwrap().len(), 36);

    // One more, and the hour after 12 would be hour 1 of the first day.
    let errore = f.ore_dal(data(7, 15), 37).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::GiornoMonco { .. }),
        "variante: {errore:?}"
    );
    let errore = errore.to_string();
    assert!(errore.contains("chiede il 16/7, ora 13"), "{errore}");
    assert!(errore.contains("porta il 15/7, ora 1"), "{errore}");
}

#[test]
fn un_periodo_monco_non_nomina_il_primo_marzo() {
    // «(o il 1/3, ora 1)» belongs to the reader, which lets 1 March follow 28
    // February. A Periodo that starts on 29 February starts there and nowhere
    // else.
    let f = leggi(&epw((13..=24)
        .map(|ora| riga_piena(2, 29, ora))
        .chain(giorno_intero(3, 1))
        .collect()))
    .unwrap();
    let errore = f.ore_dal(data(2, 29), 24).unwrap_err().to_string();
    assert!(
        errore.contains("chiede il 29/2, ora 1, e il file a quel punto porta il 29/2, ora 13"),
        "{errore}"
    );

    // A Periodo that wraps after 28 February does not name 29 February
    // either: the owner of a typical year has no such day. Back at the top of
    // the file it asks for hour 1 of the day the file opens with.
    let f = leggi(&epw((13..=24)
        .map(|ora| riga_piena(2, 27, ora))
        .chain(giorno_intero(2, 28))
        .collect()))
    .unwrap();
    let errore = f.ore_dal(data(2, 28), 25).unwrap_err().to_string();
    assert!(
        errore.contains("chiede il 27/2, ora 1, e il file a quel punto porta il 27/2, ora 13"),
        "{errore}"
    );
}

#[test]
fn in_un_file_di_piu_anni_il_periodo_comincia_dal_primo_giorno_intero() {
    // Two years, the first from noon of 1 January. The file has a whole
    // 1 January, in its second year, and a Periodo on that day fits.
    let secondo = anno(28)
        .into_iter()
        .map(|riga| riga.replacen("2005,", "2006,", 1));
    let f = leggi(&epw(anno(28).into_iter().skip(12).chain(secondo).collect())).unwrap();
    assert_eq!(f.ore_totali(), 2 * 8760 - 12);

    let ore = f.ore_dal(data(1, 1), 24).unwrap();
    for (passo, valori) in ore.iter().enumerate() {
        assert_eq!((valori.mese, valori.giorno), (1, 1), "passo {passo}");
        assert_eq!(valori.ora as usize, passo % 24 + 1, "passo {passo}");
    }

    // `si_avvolge` counts from the same record: the second year is 8760 hours
    // long, and one more runs off the end, onto the half day the file opens
    // with.
    assert!(!f.si_avvolge(data(1, 1), 8760));
    assert!(f.si_avvolge(data(1, 1), 8761));
    let errore = f.ore_dal(data(1, 1), 8761).unwrap_err();
    assert!(
        matches!(errore, ombra::meteo::MeteoError::GiornoMonco { .. }),
        "variante: {errore:?}"
    );
}

#[test]
fn un_file_che_non_si_legge_non_e_un_file_non_orario() {
    // The Corsa tells the two apart: a file it could not read leaves it without
    // a time zone, a file it read and found sub-hourly stops it.
    let errore = Epw::leggi(Path::new("questo-file-non-esiste.epw")).unwrap_err();
    assert!(!errore.non_orario());
}

#[test]
#[ignore = "legge il file EPW del corso, che non è nel repository"]
fn il_file_di_perugia_si_legge_intero() {
    let f = Epw::leggi(Path::new(
        "materiale università/ITA_Perugia.161810_IGDG.epw",
    ))
    .unwrap();
    assert_eq!(f.ore_totali(), 8760, "un anno tipo, ora per ora");
    assert_eq!(f.fuso_ore, 1.0);
    assert_eq!(f.stazione, "Perugia");

    // The two Periodi of the reference case, 48 hours each, neither of which
    // wraps.
    for inizio in [data(7, 15), data(1, 15)] {
        assert!(!f.si_avvolge(inizio, 48));
        let ore = f.ore_dal(inizio, 48).unwrap();
        assert_eq!(ore.len(), 48);
        assert!(
            ore.iter().all(|v| v.temperatura_c.is_some()),
            "il file di riferimento non ha buchi nella temperatura"
        );
        // Noon is brighter than midnight, which is the cheapest proof that the
        // radiation columns are the radiation columns.
        let mezzogiorno = ore[12].globale_orizzontale_wm2.unwrap();
        let mezzanotte = ore[0].globale_orizzontale_wm2.unwrap();
        assert!(
            mezzogiorno > mezzanotte,
            "a mezzogiorno {mezzogiorno} W/m2, a mezzanotte {mezzanotte}"
        );
    }
}
