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

/// A whole year of records, one per hour, so the wrap-around has something to
/// wrap onto. Two days is enough to be a year for the reader: it counts hours
/// from the first record and knows nothing about the calendar.
fn epw(righe: Vec<String>) -> String {
    format!("{}\n{}\n", intestazione(), righe.join("\n"))
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
    let f = leggi(&epw(vec![riga(
        7, 15, 13, "31.4", "42", "880", "790", "160", "225", "3.5",
    )]))
    .unwrap();
    let ora = f.ore_dal(data(7, 15), 1).unwrap()[0];
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
    // 29 February is the case that brings anyone here: an EPW has 8760 hours
    // and no leap day. Silently serving 28 February or 1 March would move a
    // whole Corsa by a day without saying so.
    let f = leggi(&epw(vec![riga_piena(2, 28, 1), riga_piena(3, 1, 1)])).unwrap();
    let errore = f.ore_dal(data(2, 29), 1).unwrap_err().to_string();
    assert!(errore.contains("29/2"), "{errore}");
    assert!(errore.contains("29 febbraio"), "{errore}");
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
    assert!(errore.contains("«30[2K»"), "{errore}");
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
    // months taken from different years.
    let f = leggi(&epw(vec![
        riga_piena(7, 15, 13),
        riga_piena(7, 15, 13).replacen("2005,", "2006,", 1),
    ]))
    .unwrap();
    assert_eq!(f.ore_totali(), 2);
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
