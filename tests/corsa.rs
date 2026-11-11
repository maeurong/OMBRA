//! The Giornale and the Corsa. Degenerate inputs first: every test below fails
//! if the defence it names is removed from `src/giornale.rs` or `src/corsa.rs`.
//!
//! The Giornale is read back **with the TOML parser**, never by looking for a
//! substring: a Giornale that does not re-read does not exist, because the page
//! and the print sheet are renderings of that one file.

use ombra::giornale::{
    conta_provenienza, inviluppo, somma_di_controllo, Giornale, Impronta, Ingresso, ASSENTE,
};
use ombra::motore;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn tempdir_di_prova(nome: &str) -> PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(nome);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn rileggi(percorso: &std::path::Path) -> toml::Table {
    let testo = fs::read_to_string(percorso).unwrap();
    toml::from_str(&testo).unwrap_or_else(|e| panic!("il Giornale non si rilegge: {e}\n{testo}"))
}

/// Quotes, a backslash and a line break, which is what a user types into an
/// etichetta and what breaks a file written by hand.
const TESTO_OSTILE: &str = "corsa \"buona\" \\ del\nlunedì";

/// The Impronta of the inputs `l_impronta_di_ingressi_fissati_vale_questa`
/// builds. A golden value: it moves only when what goes into an Impronta is
/// meant to move.
const IMPRONTA_FISSATA: &str = "7631d0b8bdebc89ba216960546891b38da17c9a6ddaeaecf8ce0b7e11b01203b";

fn riga(chiave: &str, valore: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(chiave.to_owned(), valore.to_owned())])
}

#[test]
fn la_somma_di_controllo_riproduce_i_vettori_noti() {
    // The three vectors of FIPS 180-4: empty, one block, and the message that
    // needs a second block for its padding.
    assert_eq!(
        somma_di_controllo(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        somma_di_controllo(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        somma_di_controllo(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

/// The lengths the padding turns on. At 55 bytes the length field is the last
/// eight of one block; at 56 it no longer fits and a whole second block appears;
/// 63, 64, 119 and 120 are the same two steps one block further on. A padding
/// loop that is off by one block is right on every other length, so these are
/// the only lengths that notice.
///
/// Reference values from `hashlib.sha256` on `b"a" * n`.
#[test]
fn la_somma_di_controllo_riproduce_i_bordi_del_riempimento() {
    for (quanti, atteso) in [
        (
            55,
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
        ),
        (
            56,
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
        ),
        (
            63,
            "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
        ),
        (
            64,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
        ),
        (
            119,
            "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb",
        ),
        (
            120,
            "2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c",
        ),
    ] {
        assert_eq!(
            somma_di_controllo(&vec![b'a'; quanti]),
            atteso,
            "{quanti} byte"
        );
    }
}

#[test]
fn il_giornale_resta_toml_valido_con_virgolette_barre_e_ritorni_a_capo() {
    let dir = tempdir_di_prova("giornale-ostile");
    let mut giornale = Giornale::apri(&dir, "giornale.toml").unwrap();
    let percorso = giornale.percorso().to_path_buf();
    giornale
        .annota("corsa", &riga("etichetta", TESTO_OSTILE))
        .unwrap();
    giornale.concludi(None).unwrap();

    let tabella = rileggi(&percorso);
    assert_eq!(
        tabella["corsa"]["etichetta"].as_str(),
        Some(TESTO_OSTILE),
        "il testo deve tornare identico, non solo essere presente"
    );
}

#[test]
fn un_giornale_interrotto_non_ha_conclusione_ne_un_esito_al_livello_superiore() {
    let dir = tempdir_di_prova("giornale-interrotto");
    let mut giornale = Giornale::apri(&dir, "giornale.toml").unwrap();
    let percorso = giornale.percorso().to_path_buf();
    giornale
        .annota("corsa", &riga("etichetta", "interrotta"))
        .unwrap();
    giornale
        .annota("derivazione", &riga("celle_costruite", "12"))
        .unwrap();
    drop(giornale);

    let tabella = rileggi(&percorso);
    assert!(tabella.contains_key("corsa"), "i passi già fatti restano");
    assert!(
        tabella.contains_key("derivazione"),
        "i passi già fatti restano"
    );
    assert!(
        !tabella.contains_key("conclusione"),
        "finché [conclusione] non c'è la Corsa non è finita: l'assenza è lo stato"
    );
    assert!(
        !tabella.contains_key("esito"),
        "nessun esito al livello superiore: un file appeso non può riscrivere una riga di sopra"
    );
}

#[test]
fn una_corsa_fallita_conclude_con_esito_fallita_e_l_errore_leggibile() {
    let dir = tempdir_di_prova("giornale-fallito");
    let mut giornale = Giornale::apri(&dir, "giornale.toml").unwrap();
    let percorso = giornale.percorso().to_path_buf();
    giornale
        .annota("corsa", &riga("etichetta", "che fallisce"))
        .unwrap();
    giornale
        .concludi(Some(
            "il periodo «inverno» comincia il 30 febbraio 2021, che non è una data",
        ))
        .unwrap();

    let tabella = rileggi(&percorso);
    let conclusione = tabella["conclusione"].as_table().unwrap();
    assert_eq!(conclusione["esito"].as_str(), Some("fallita"));
    assert!(conclusione["errore"]
        .as_str()
        .unwrap()
        .contains("30 febbraio"));
    assert!(!tabella.contains_key("esito"));
}

#[test]
fn una_corsa_riuscita_conclude_con_esito_riuscita_e_nessun_errore() {
    let dir = tempdir_di_prova("giornale-riuscito");
    let mut giornale = Giornale::apri(&dir, "giornale.toml").unwrap();
    let percorso = giornale.percorso().to_path_buf();
    giornale
        .annota("corsa", &riga("etichetta", "che riesce"))
        .unwrap();
    giornale.concludi(None).unwrap();

    let tabella = rileggi(&percorso);
    let conclusione = tabella["conclusione"].as_table().unwrap();
    assert_eq!(conclusione["esito"].as_str(), Some("riuscita"));
    assert!(!conclusione.contains_key("errore"));
}

#[test]
fn un_campo_tutto_nan_non_si_inventa_un_minimo() {
    let inv = inviluppo("chiome", "m", [f32::NAN; 4], (0.0, 100.0), "");
    assert_eq!(inv.minimo, None);
    assert_eq!(inv.massimo, None);
    assert_eq!(inv.media, None);
    assert_eq!(inv.frazione_senza_dato, 1.0);
    assert!(
        !inv.fuori_intervallo,
        "senza dato non c'è niente da segnalare"
    );
}

#[test]
fn l_inviluppo_conta_solo_le_celle_con_dato() {
    let inv = inviluppo(
        "chiome",
        "m",
        [f32::NAN, 2.0, 4.0, f32::NAN],
        (0.0, 100.0),
        "",
    );
    assert_eq!(inv.minimo, Some(2.0));
    assert_eq!(inv.massimo, Some(4.0));
    assert_eq!(inv.media, Some(3.0));
    assert_eq!(inv.frazione_senza_dato, 0.5);
}

#[test]
fn un_valore_fuori_dall_intervallo_plausibile_alza_la_bandiera_e_viene_riportato() {
    let inv = inviluppo("ore di sole", "h", [0.5, 3.0], (0.0, 1.0), "");
    assert!(inv.fuori_intervallo);
    assert_eq!(inv.massimo, Some(3.0), "il valore si riporta comunque");

    let dentro = inviluppo("ore di sole", "h", [0.0, 1.0], (0.0, 1.0), "");
    assert!(!dentro.fuori_intervallo, "gli estremi sono dentro");
}

#[test]
fn l_impronta_cambia_se_cambia_un_solo_byte_di_un_ingresso() {
    let dir = tempdir_di_prova("impronta-byte");
    let file = dir.join("meteo.epw");
    fs::write(&file, "abc").unwrap();
    let motore = motore::versione();
    let calcola = || {
        Impronta::calcola(
            &[Ingresso::leggi(&file, &dir, "meteo")],
            "0.1.0",
            &motore,
            "p",
        )
    };

    let prima = calcola();
    fs::write(&file, "abd").unwrap();
    assert_ne!(prima, calcola(), "un byte diverso, Impronta diversa");
}

#[test]
fn l_impronta_cambia_se_cambiano_i_parametri_o_il_binario() {
    let dir = tempdir_di_prova("impronta-parametri");
    let file = dir.join("meteo.epw");
    fs::write(&file, "abc").unwrap();
    let motore = motore::versione();
    let ingressi = [Ingresso::leggi(&file, &dir, "meteo")];

    let base = Impronta::calcola(&ingressi, "0.1.0", &motore, "p");
    assert_ne!(base, Impronta::calcola(&ingressi, "0.2.0", &motore, "p"));
    assert_ne!(base, Impronta::calcola(&ingressi, "0.1.0", &motore, "q"));
}

#[test]
fn un_ingresso_assente_non_e_mai_identico_a_uno_presente() {
    let dir = tempdir_di_prova("impronta-assente");
    let file = dir.join("mai-scritto.epw");
    let motore = motore::versione();

    let mancante = Ingresso::leggi(&file, &dir, "meteo");
    assert_eq!(mancante.sha256, ASSENTE);
    let senza = Impronta::calcola(&[mancante], "0.1.0", &motore, "p");

    // Including the text an absent input writes: a file whose contents are the
    // word "assente" is still a file that is there.
    for contenuto in ["", "assente", ASSENTE] {
        fs::write(&file, contenuto).unwrap();
        let presente = Impronta::calcola(
            &[Ingresso::leggi(&file, &dir, "meteo")],
            "0.1.0",
            &motore,
            "p",
        );
        assert_ne!(senza, presente, "contenuto «{contenuto}»");
    }
}

#[test]
fn la_provenienza_si_conta_per_anello_della_catena() {
    use ombra::dominio::*;
    let provenienza = |altezza| Provenienza {
        origine: "prova".into(),
        altezza,
    };
    let scenario = Scenario {
        nome: "prova".into(),
        derivato_da: None,
        terreno_m: vec![0.0; 4],
        provenienza: provenienza(FonteAltezza::Predefinito),
        edifici: vec![
            Edificio {
                altezza_m: 6.0,
                provenienza: Some(provenienza(FonteAltezza::Rilievo)),
                impronta: vec![],
            },
            Edificio {
                altezza_m: 3.0,
                provenienza: Some(provenienza(FonteAltezza::NumeroDiPiani)),
                impronta: vec![],
            },
            Edificio {
                altezza_m: 3.0,
                provenienza: Some(provenienza(FonteAltezza::ModelloDiSuperficie)),
                impronta: vec![],
            },
        ],
        // No provenienza of its own: it inherits the Scenario's link.
        alberi: vec![Albero {
            posizione_m: (0.5, 0.5),
            specie: "020027".into(),
            altezza_m: 12.0,
            frazione_tronco: 0.45,
            provenienza: None,
        }],
        superfici: vec![],
    };

    let conteggio = conta_provenienza(&scenario);
    assert_eq!(conteggio.rilievo, 1);
    assert_eq!(conteggio.numero_di_piani, 1);
    assert_eq!(conteggio.modello_di_superficie, 1);
    assert_eq!(
        conteggio.predefinito, 1,
        "l'albero eredita l'anello dello Scenario"
    );
}

/// The branch a value *below* the minimum takes. Nothing in this project ever
/// produced one, so the branch was never walked: a plausible range with only its
/// upper half enforced is half a check.
#[test]
fn un_valore_sotto_il_minimo_plausibile_alza_la_bandiera_e_viene_riportato() {
    let inv = inviluppo("quota", "m", [-3.0, 5.0], (0.0, 10.0), "");
    assert!(inv.fuori_intervallo);
    assert_eq!(inv.minimo, Some(-3.0), "il valore si riporta comunque");
}

/// The Giornale is read by a person. The reproducibility lives in the Impronta,
/// not in the fifteenth decimal of a mean.
#[test]
fn l_inviluppo_arrotonda_le_cifre_che_scrive() {
    let inv = inviluppo("quota", "m", [10.0, 18.0, 15.0, f32::NAN], (0.0, 100.0), "");
    assert_eq!(inv.media, Some(14.3333));
    assert_eq!(inv.frazione_senza_dato, 0.25);

    let terzi = inviluppo("quota", "m", [f32::NAN, f32::NAN, 1.0], (0.0, 100.0), "");
    assert_eq!(terzi.frazione_senza_dato, 0.6667);
}

/// A field of a Giornale that a reader can misread has to carry the sentence
/// that stops them: `frazione_senza_dato` on the canopies is not missing data.
#[test]
fn ogni_campo_porta_la_sua_nota() {
    let inv = inviluppo("chiome", "m", [1.0], (0.0, 100.0), "una nota qualunque");
    assert_eq!(inv.nota, "una nota qualunque");
}

/// Two files with the same bytes under two different names are two different
/// inputs, and a Corsa that read one did not read the other.
#[test]
fn l_impronta_cambia_se_cambia_il_percorso_di_un_ingresso() {
    let dir = tempdir_di_prova("impronta-percorso");
    fs::write(dir.join("uno.epw"), "abc").unwrap();
    fs::write(dir.join("due.epw"), "abc").unwrap();
    let motore = motore::versione();
    let impronta = |nome: &str| {
        Impronta::calcola(
            &[Ingresso::leggi(dir.join(nome), &dir, "meteo")],
            "0.1.0",
            &motore,
            "p",
        )
    };
    assert_ne!(impronta("uno.epw"), impronta("due.epw"));
}

/// The pinned kernel is half of what makes the answer what it is, and the
/// Giornale cites it. An Impronta blind to it would call two Corse the same
/// Corsa across a change of engine.
#[test]
fn l_impronta_cambia_se_cambia_il_motore() {
    let motore = |commit: &str, data: &str| ombra::motore::VersioneMotore {
        commit: commit.to_owned(),
        data_presa: data.to_owned(),
    };
    let base = Impronta::calcola(&[], "0.1.0", &motore("aaa", "2026-01-01"), "p");
    assert_ne!(
        base,
        Impronta::calcola(&[], "0.1.0", &motore("bbb", "2026-01-01"), "p"),
        "commit diverso"
    );
    assert_ne!(
        base,
        Impronta::calcola(&[], "0.1.0", &motore("aaa", "2026-01-02"), "p"),
        "data di presa diversa"
    );
}

/// A fixed value, so that a change to what goes into the Impronta or to the
/// order it goes in has somewhere to fail. Regenerate it on purpose, never to
/// make a red test green.
#[test]
fn l_impronta_di_ingressi_fissati_vale_questa() {
    let motore = ombra::motore::VersioneMotore {
        commit: "0000000000000000000000000000000000000000".into(),
        data_presa: "2026-01-01".into(),
    };
    let ingresso = |percorso: &str| Ingresso {
        percorso: percorso.to_owned(),
        ruolo: "scenario".to_owned(),
        sha256: somma_di_controllo(percorso.as_bytes()),
        usato: true,
    };
    let ingressi = [ingresso("scenari/a.toml"), ingresso("scenari/b.toml")];
    assert_eq!(
        Impronta::calcola(&ingressi, "0.1.0", &motore, "parametri").testo(),
        IMPRONTA_FISSATA
    );

    let invertiti = [ingressi[1].clone(), ingressi[0].clone()];
    assert_ne!(
        Impronta::calcola(&invertiti, "0.1.0", &motore, "parametri").testo(),
        IMPRONTA_FISSATA,
        "l'ordine degli ingressi è parte dell'Impronta"
    );
}

/// A Progetto is a folder that gets copied, moved and unzipped somewhere else.
/// Its Corse are the same Corse afterwards, or the reproducibility this project
/// promises at the head of `src/corsa.rs` is not true.
#[test]
fn lo_stesso_progetto_in_due_cartelle_ha_la_stessa_impronta() {
    let motore = motore::versione();
    let impronta_di = |nome: &str| {
        let dir = tempdir_di_prova(nome);
        fs::create_dir_all(dir.join("scenari")).unwrap();
        fs::write(dir.join("scenari/stato-di-fatto.toml"), "abc").unwrap();
        Impronta::calcola(
            &[Ingresso::leggi(
                dir.join("scenari/stato-di-fatto.toml"),
                &dir,
                "scenario",
            )],
            "0.1.0",
            &motore,
            "p",
        )
    };
    assert_eq!(impronta_di("progetto-qui"), impronta_di("progetto-altrove"));
}

/// The Giornale refuses to write through a link. `giornale.toml` was covered and
/// its folders were not, which is the half that a `corse -> /etc` uses.
#[cfg(unix)]
#[test]
fn un_giornale_sotto_una_cartella_collegata_e_rifiutato() {
    let dir = tempdir_di_prova("corse-collegate");
    fs::create_dir_all(dir.join("altrove")).unwrap();
    std::os::unix::fs::symlink(dir.join("altrove"), dir.join("corse")).unwrap();
    let err = Giornale::apri(&dir, "corse/abc/giornale.toml").expect_err("deve fallire");
    assert!(
        matches!(err, ombra::giornale::GiornaleError::Collegamento(_)),
        "variante: {err:?}"
    );
    assert!(
        !dir.join("altrove/abc").exists(),
        "nessuna scrittura attraverso il collegamento"
    );
}

#[cfg(unix)]
#[test]
fn un_giornale_che_e_un_collegamento_e_rifiutato() {
    let dir = tempdir_di_prova("giornale-collegato");
    fs::write(dir.join("fuori.toml"), "").unwrap();
    std::os::unix::fs::symlink(dir.join("fuori.toml"), dir.join("giornale.toml")).unwrap();
    let err = Giornale::apri(&dir, "giornale.toml").expect_err("deve fallire");
    assert!(
        matches!(err, ombra::giornale::GiornaleError::Collegamento(_)),
        "variante: {err:?}"
    );
    assert_eq!(
        fs::read_to_string(dir.join("fuori.toml")).unwrap(),
        "",
        "nessuna scrittura attraverso il collegamento"
    );
}

// ---------------------------------------------------------------------------
// La Corsa
// ---------------------------------------------------------------------------

use ombra::corsa;
use ombra::dominio::{
    Albero, Data, Edificio, FonteAltezza, Griglia, Periodo, Progetto, Provenienza,
    PuntoDiOsservazione, Rettangolo, Scenario,
};

/// A Progetto with one flat Scenario and a tower in the middle of it.
fn progetto_di_prova(lato: usize, periodi: Vec<Periodo>) -> Progetto {
    let mezzo = (lato / 2) as f64;
    Progetto {
        nome: "prova".into(),
        griglia: Griglia {
            nx: lato,
            ny: lato,
            passo_m: 1.0,
            crs: "EPSG:4326".into(),
            origine: (12.56, 43.07),
            rotazione_gradi: 0.0,
        },
        punti: vec![],
        scenari: vec![Scenario {
            nome: "stato-di-fatto".into(),
            derivato_da: None,
            terreno_m: vec![0.0; lato * lato],
            provenienza: Provenienza {
                origine: "prova".into(),
                altezza: FonteAltezza::Predefinito,
            },
            edifici: vec![Edificio {
                altezza_m: 10.0,
                provenienza: Some(Provenienza {
                    origine: "prova".into(),
                    altezza: FonteAltezza::Rilievo,
                }),
                impronta: vec![Rettangolo {
                    x_min_m: mezzo - 1.0,
                    y_min_m: mezzo - 1.0,
                    x_max_m: mezzo + 1.0,
                    y_max_m: mezzo + 1.0,
                }],
            }],
            alberi: vec![],
            superfici: vec![],
        }],
        periodi,
    }
}

fn albero_di_prova(specie: &str, posizione_m: (f64, f64)) -> Albero {
    Albero {
        posizione_m,
        specie: specie.into(),
        altezza_m: 12.0,
        frazione_tronco: 0.35,
        provenienza: Some(Provenienza {
            origine: "prova".into(),
            altezza: FonteAltezza::Predefinito,
        }),
    }
}

fn periodo(nome: &str, mese: u32, giorno: u32, ore: u32) -> Periodo {
    Periodo {
        nome: nome.into(),
        meteo: "materiale università/ITA_Perugia.161810_IGDG.epw".into(),
        ore,
        direzione_vento_gradi: Some(45.0),
        inizio: Data {
            anno: 2021,
            mese,
            giorno,
        },
    }
}

fn scrivi_ed_esegui(nome: &str, progetto: &Progetto) -> (PathBuf, corsa::Rapporto) {
    let dir = tempdir_di_prova(nome);
    ombra::progetto::scrivi(&dir, progetto).unwrap();
    let rapporto = corsa::esegui_progetto(&dir).unwrap();
    (dir, rapporto)
}

#[test]
fn due_periodi_nella_stessa_stagione_condividono_una_sola_derivazione() {
    let progetto = progetto_di_prova(
        6,
        vec![periodo("luglio", 7, 15, 1), periodo("agosto", 8, 15, 1)],
    );
    let (_, rapporto) = scrivi_ed_esegui("una-derivazione", &progetto);
    assert_eq!(rapporto.corse.len(), 2);
    assert_eq!(
        rapporto.derivazioni, 1,
        "stesso Scenario e stessa Stagione: la Derivazione si fa una volta sola"
    );
}

#[test]
fn due_periodi_di_stagioni_diverse_derivano_due_volte() {
    let progetto = progetto_di_prova(
        6,
        vec![periodo("estate", 7, 15, 1), periodo("inverno", 1, 15, 1)],
    );
    let (_, rapporto) = scrivi_ed_esegui("due-derivazioni", &progetto);
    assert_eq!(rapporto.corse.len(), 2);
    assert_eq!(
        rapporto.derivazioni, 2,
        "il Periodo senza foglie toglie le chiome decidue: il raster cambia davvero"
    );
}

#[test]
fn un_periodo_con_una_data_impossibile_fa_fallire_la_corsa_invece_di_andare_in_panico() {
    let progetto = progetto_di_prova(6, vec![periodo("impossibile", 2, 30, 1)]);
    let (_, rapporto) = scrivi_ed_esegui("data-impossibile", &progetto);

    let esito = &rapporto.corse[0];
    let errore = esito.errore.as_deref().expect("la Corsa deve fallire");
    assert!(errore.contains("30"), "l'errore nomina la data: {errore}");

    let tabella = rileggi(&esito.giornale);
    assert_eq!(tabella["conclusione"]["esito"].as_str(), Some("fallita"));
    assert!(
        tabella.contains_key("derivazione"),
        "i passi già fatti restano nel Giornale"
    );
    assert!(
        !tabella.contains_key("campo"),
        "nessun campo è stato calcolato"
    );
    assert!(!tabella.contains_key("esito"));
}

#[test]
fn una_griglia_senza_latitudine_fa_fallire_la_corsa_invece_di_indovinarla() {
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 1)]);
    progetto.griglia.crs = "EPSG:32633".into();
    let (_, rapporto) = scrivi_ed_esegui("crs-proiettato", &progetto);

    let errore = rapporto.corse[0]
        .errore
        .as_deref()
        .expect("senza latitudine non c'è sole");
    assert!(errore.contains("EPSG:32633"), "{errore}");
}

#[test]
fn il_giornale_di_una_corsa_si_rilegge_col_parser_anche_con_testo_ostile() {
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    progetto.nome = TESTO_OSTILE.into();
    progetto.scenari[0].provenienza.origine = TESTO_OSTILE.into();
    let (_, rapporto) = scrivi_ed_esegui("corsa-ostile", &progetto);

    let esito = &rapporto.corse[0];
    assert_eq!(esito.errore, None);
    let tabella = rileggi(&esito.giornale);
    assert_eq!(tabella["corsa"]["progetto"].as_str(), Some(TESTO_OSTILE));
    assert_eq!(
        tabella["scenario"]["provenienza_origine"].as_str(),
        Some(TESTO_OSTILE)
    );
}

#[test]
fn il_giornale_registra_ingressi_versioni_scelte_e_inviluppi() {
    let progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    let (_, rapporto) = scrivi_ed_esegui("giornale-completo", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);

    assert_eq!(
        tabella["corsa"]["impronta"].as_str(),
        Some(rapporto.corse[0].impronta.testo())
    );
    assert!(!tabella["corsa"]["citazione"].as_str().unwrap().is_empty());
    assert_eq!(
        tabella["motore"]["commit"].as_str(),
        Some(motore::versione().commit.as_str())
    );
    // Written out and not read from the build: the Impronta takes the version
    // and not the way a field is computed, so the version that stopped
    // counting twice the sky two canopy layers share is a number this test
    // has to name.
    assert_eq!(tabella["binario"]["versione"].as_str(), Some("0.3.0"));
    assert_eq!(tabella["griglia"]["nx"].as_integer(), Some(6));
    assert_eq!(
        tabella["derivazione"]["celle_costruite"].as_integer(),
        Some(4)
    );
    assert_eq!(tabella["provenienza"]["rilievo"].as_integer(), Some(1));
    assert!(tabella.contains_key("riproducibilita"));
    assert!(tabella.contains_key("verifiche_per_rilascio"));

    let ingressi = tabella["ingresso"].as_array().unwrap();
    let ruoli: Vec<&str> = ingressi
        .iter()
        .map(|i| i["ruolo"].as_str().unwrap())
        .collect();
    assert!(ruoli.contains(&"manifesto"));
    assert!(ruoli.contains(&"scenario"));
    assert!(ruoli.contains(&"periodo"));
    assert!(
        ruoli.contains(&"meteo"),
        "i valori meteorologici sono un ingresso, non un risultato"
    );

    let campi = tabella["campo"].as_array().unwrap();
    let nomi: Vec<&str> = campi.iter().map(|c| c["campo"].as_str().unwrap()).collect();
    assert!(nomi.contains(&"frazione illuminata media"));
    assert!(nomi.contains(&"ore di sole"));
    assert!(
        !nomi.contains(&"chiome"),
        "questo Scenario non ha alberi: un campo delle chiome tutto senza dato \
         direbbe che qualcosa è stato misurato e non trovato"
    );
}

#[test]
fn il_giornale_dichiara_ogni_strato_di_chioma_con_la_sua_trasmissivita() {
    let mut progetto = progetto_di_prova(6, vec![periodo("inverno", 1, 15, 2)]);
    // A pine and a plane tree: in January the first is still opaque and the
    // second is bare, so the Corsa carries two layers and the Giornale has to
    // name both.
    progetto.scenari[0].alberi = vec![
        albero_di_prova("020027", (1.5, 1.5)),
        albero_di_prova("020060", (4.5, 4.5)),
    ];
    let (_, rapporto) = scrivi_ed_esegui("giornale-strati", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);

    assert_eq!(
        tabella["derivazione"]["chiome_spogliate"].as_integer(),
        Some(1)
    );
    let campi = tabella["campo"].as_array().unwrap();
    let nomi: Vec<&str> = campi.iter().map(|c| c["campo"].as_str().unwrap()).collect();
    assert!(nomi.contains(&"chiome"), "{nomi:?}");
    assert!(nomi.contains(&"chiome spoglie"), "{nomi:?}");
    for (campo, per_cento) in [("chiome", "3"), ("chiome spoglie", "50")] {
        let nota = campi
            .iter()
            .find(|c| c["campo"].as_str() == Some(campo))
            .unwrap()["nota"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            nota.contains(&format!("{per_cento} per cento")),
            "la nota di «{campo}» non dice quanto lascia passare: {nota}"
        );
    }
}

/// A weather file with `fuso` as its time zone and one record of July per
/// `(giorno, ora, minuto)`, written into the Progetto so the test does not
/// depend on a file that is not in the repository.
fn scrivi_epw_di(dir: &std::path::Path, fuso: f64, record: &[(u32, u32, &str)]) -> PathBuf {
    let mut testo = format!("LOCATION,Prova,-,ITA,X,000000,43.07,12.56,{fuso},213.0");
    for _ in 0..7 {
        testo.push_str("\nRIGA DI INTESTAZIONE,ignorata");
    }
    for (giorno, ora, minuto) in record {
        testo.push_str(&format!(
            "\n2005,7,{giorno},{ora},{minuto},?9?9,24.0,7.2,55,98792,9999,9999,290,120,300,80,\
             999900,999900,999900,99990,45,4.0,1,1,999.0,999,9,999999999,0,0.0000,0,88,\
             999.000,999.0,99.0"
        ));
    }
    testo.push('\n');
    let percorso = dir.join("prova.epw");
    fs::write(&percorso, testo).unwrap();
    percorso
}

/// 15 and 16 July, one record per hour, every minute field as given.
fn due_giorni(minuto: &'static str) -> Vec<(u32, u32, &'static str)> {
    [15, 16]
        .into_iter()
        .flat_map(|giorno| (1..=24).map(move |ora| (giorno, ora, minuto)))
        .collect()
}

fn scrivi_epw(dir: &std::path::Path, fuso: f64) -> PathBuf {
    scrivi_epw_di(dir, fuso, &due_giorni("0"))
}

/// Runs a Corsa of forty-eight Ore at `latitudine` and checks it against the
/// sun of `k + 0:30`, Ora by Ora: the series of a Punto, and the map of the
/// hours in the sun. Returns the series and the elevation of the sun at 23:30
/// of the first day, which is the sun of Ora 23.
fn cammina_col_sole_al_centro(nome: &str, latitudine: f64) -> (Vec<f64>, f64) {
    const ORE: u32 = 48;
    let dir = tempdir_di_prova(nome);
    let epw = scrivi_epw(&dir, 1.0);
    let mut progetto = progetto_di_prova(21, vec![periodo("estate", 7, 15, ORE)]);
    progetto.griglia.origine.1 = latitudine;
    progetto.periodi[0].meteo = epw;
    // West of the tower and a little north: in its shadow for part of the
    // morning, so the series has something to get wrong.
    progetto.punti = vec![punto(1, (6.5, 11.5))];
    ombra::progetto::scrivi(&dir, &progetto).unwrap();
    let rapporto = corsa::esegui_progetto(&dir).unwrap();
    assert_eq!(rapporto.corse[0].errore, None);
    let campi = rapporto.corse[0].campi.as_ref().unwrap();
    let serie = &campi.serie[0];

    // The same sun and the same Motore, asked here hour by hour.
    let raster = ombra::derivazione::deriva(
        &progetto.griglia,
        &progetto.scenari[0],
        &progetto.periodi[0],
    )
    .unwrap();
    let sole = |ora_locale_h: f64| {
        ombra::sole::posizione(
            progetto.periodi[0].inizio,
            ora_locale_h,
            1.0,
            latitudine,
            12.56,
        )
        .unwrap()
    };
    let illuminata = |ora_locale_h: f64| {
        motore::ombre(
            &raster.modello_di_superficie,
            progetto.griglia.passo_m,
            sole(ora_locale_h),
            &raster.strati_di_chioma,
        )
    };

    let mut al_centro = ombra::derivazione::Raster::zeros(campi.ore_di_sole.dim());
    let mut all_inizio = al_centro.clone();
    for ora in 0..ORE {
        let centro = f64::from(ora) + 0.5;
        let attesa = illuminata(centro);
        let del_punto = f64::from(attesa[[serie.riga, serie.colonna]]);
        assert!(
            (serie.frazione_illuminata[ora as usize] - del_punto).abs() < 1e-4,
            "{nome}, Ora {ora}: la serie dice {} e il sole di {centro} dice {del_punto}",
            serie.frazione_illuminata[ora as usize]
        );
        al_centro += &attesa;
        all_inizio += &illuminata(f64::from(ora));
    }

    let scarto_massimo = |altra: &ombra::derivazione::Raster| {
        campi
            .ore_di_sole
            .iter()
            .zip(altra)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max)
    };
    assert!(
        scarto_massimo(&al_centro) < 1e-4,
        "{nome}: le ore di sole non sono la somma delle ombre al centro di ogni \
         Ora, scarto massimo {} h",
        scarto_massimo(&al_centro)
    );
    // What makes the assertion above a test of the convention: on this Scenario
    // the two conventions do not give the same map.
    assert!(
        scarto_massimo(&all_inizio) >= 1.0,
        "{nome}: il sole all'inizio dell'Ora darebbe la stessa mappa, questo \
         Scenario non distingue le due convenzioni"
    );
    (serie.frazione_illuminata.clone(), sole(23.5).altezza_gradi)
}

/// The sun of Ora `k` is the sun of `k + 0:30`, for everything a Corsa computes
/// in that Ora. Forty-eight Ore, so the walk goes past the twenty-fourth.
#[test]
fn il_sole_di_un_ora_e_quello_del_suo_centro() {
    let (serie, altezza_alle_23_30) = cammina_col_sole_al_centro("sole-al-centro", 43.07);
    // At Bastia Ora 23 is night under any convention: the walk says nothing
    // about it here beyond the zero.
    assert!(altezza_alle_23_30 < 0.0);
    assert_eq!(serie[23], 0.0);
}

/// Ora 23 has its sun at 23:30 of the same day. Where that is night, 23:00,
/// 23:30 and midnight all leave the domain dark and no test can tell them
/// apart, so the same walk is taken where the sun does not set in July.
#[test]
fn l_ora_23_ha_il_sole_delle_23_e_30_anche_dove_a_quell_ora_e_giorno() {
    let (serie, altezza_alle_23_30) = cammina_col_sole_al_centro("sole-di-mezzanotte", 75.0);
    assert!(
        altezza_alle_23_30 > 0.0,
        "a 75 gradi nord il sole delle 23:30 di luglio è sopra l'orizzonte"
    );
    assert!(serie[23] > 0.0, "e il Punto, all'Ora 23, lo vede");
}

/// The nights the shadow check counts are counted with the sun of the centre.
/// Six Ore around dawn, where one Ora changes side with the convention: with
/// the clock half an hour ahead the start of the Ora counts one night more,
/// with the clock on the hour the end of the Ora counts one fewer.
#[test]
fn la_verifica_dell_ombra_conta_le_notti_col_sole_del_centro() {
    for (fuso, altra_convenzione_h) in [(1.5, 0.0), (1.0, 1.0)] {
        let dir = tempdir_di_prova(&format!("notti-all-alba-{fuso}"));
        let epw = scrivi_epw(&dir, fuso);
        let mut progetto = progetto_di_prova(6, vec![periodo("alba", 7, 15, 6)]);
        progetto.periodi[0].meteo = epw;
        ombra::progetto::scrivi(&dir, &progetto).unwrap();
        let rapporto = corsa::esegui_progetto(&dir).unwrap();
        let tabella = rileggi(&rapporto.corse[0].giornale);

        let notti = |dentro_l_ora_h: f64| {
            (0..6)
                .filter(|&ora| {
                    ombra::sole::posizione(
                        progetto.periodi[0].inizio,
                        f64::from(ora) + dentro_l_ora_h,
                        fuso,
                        43.07,
                        12.56,
                    )
                    .unwrap()
                    .altezza_gradi
                        <= 0.0
                })
                .count() as i64
        };
        assert_ne!(
            notti(0.5),
            notti(altra_convenzione_h),
            "fuso {fuso}: questo Periodo non distingue le due convenzioni"
        );
        assert_eq!(
            tabella["verifica_ombra"]["ore_notturne"].as_integer(),
            Some(notti(0.5)),
            "fuso {fuso}"
        );
    }
}

#[test]
fn il_giornale_dichiara_l_istante_del_sole_e_il_minuto_del_file_meteo() {
    // Both spellings of an hourly file: the minute does not move the sun, and
    // the Giornale reports which one the file used.
    for (minuto, atteso) in [("0", 0), ("60", 60)] {
        let dir = tempdir_di_prova(&format!("istante-minuto-{minuto}"));
        let epw = scrivi_epw_di(&dir, 1.0, &due_giorni(minuto));
        let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
        progetto.periodi[0].meteo = epw;
        ombra::progetto::scrivi(&dir, &progetto).unwrap();
        let rapporto = corsa::esegui_progetto(&dir).unwrap();
        assert_eq!(rapporto.corse[0].errore, None, "minuto {minuto}");
        let tabella = rileggi(&rapporto.corse[0].giornale);

        assert_eq!(tabella["sole"]["istante"].as_str(), Some("centro dell'Ora"));
        let minuti: Vec<i64> = tabella["sole"]["minuti_del_file_meteo"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m.as_integer().unwrap())
            .collect();
        assert_eq!(minuti, vec![atteso]);
        // The note explains the key it sits next to, and only when there is a
        // file to explain.
        let nota = tabella["sole"]["nota"].as_str().unwrap();
        assert!(nota.contains("minuti_del_file_meteo"), "{nota}");
        assert!(nota.contains("k + 0:30"), "{nota}");
    }
}

#[test]
fn un_file_meteo_non_orario_ferma_la_corsa_prima_del_calcolo() {
    // A file that will not read leaves the Corsa without a time zone and it
    // runs on a guess it declares. A file that reads and is not hourly is
    // another matter: the Corsa would be computed on hours that are not hours.
    let mut mezz_ora = due_giorni("0");
    mezz_ora[30].2 = "30";
    let mut ripetuta = due_giorni("60");
    ripetuta.insert(30, ripetuta[30]);
    // Record 30 is hour 7 of 16 July: without it every Ora after it would get
    // the record of the hour after its own.
    let mut bucata = due_giorni("0");
    bucata.remove(30);
    let mut ora_25 = due_giorni("0");
    ora_25[30].1 = 25;

    for (nome, record, atteso) in [
        ("meteo-a-mezz-ora", mezz_ora, "«30»"),
        ("meteo-ora-ripetuta", ripetuta, "due righe di dati"),
        ("meteo-con-un-buco", bucata, "ci va il 16/7, ora 7"),
        ("meteo-ora-25", ora_25, "l'ora è «25»"),
    ] {
        let dir = tempdir_di_prova(nome);
        let epw = scrivi_epw_di(&dir, 1.0, &record);
        let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
        progetto.periodi[0].meteo = epw;
        ombra::progetto::scrivi(&dir, &progetto).unwrap();
        let rapporto = corsa::esegui_progetto(&dir).unwrap();

        let esito = &rapporto.corse[0];
        let errore = esito
            .errore
            .as_deref()
            .unwrap_or_else(|| panic!("{nome}: la Corsa deve fallire"));
        assert!(errore.contains(atteso), "{nome}: {errore}");
        assert!(errore.contains("prova.epw"), "{nome}: {errore}");
        assert!(esito.campi.is_none(), "{nome}: niente è stato calcolato");

        let tabella = rileggi(&esito.giornale);
        assert_eq!(tabella["conclusione"]["esito"].as_str(), Some("fallita"));
        assert!(!tabella.contains_key("campo"), "{nome}");
        assert!(!tabella.contains_key("verifica_ombra"), "{nome}");
    }
}

#[test]
fn ogni_periodo_risponde_del_suo_file_meteo_e_non_di_quello_di_un_altro() {
    // Two Periodi on an hourly file and one on a file that is not. The one
    // that is not sorts first by name, so a Corsa handed "the first file"
    // instead of its own would fail twice where it has to succeed.
    let dir = tempdir_di_prova("due-file-meteo");
    let mut mezz_ora = due_giorni("0");
    mezz_ora[30].2 = "30";
    let guasto = dir.join("guasto.epw");
    fs::rename(scrivi_epw_di(&dir, 1.0, &mezz_ora), &guasto).unwrap();
    let orario = scrivi_epw(&dir, 1.0);

    let mut progetto = progetto_di_prova(
        6,
        vec![
            periodo("primo", 7, 15, 2),
            periodo("secondo", 7, 16, 2),
            periodo("terzo", 7, 15, 2),
        ],
    );
    progetto.periodi[0].meteo = orario.clone();
    progetto.periodi[1].meteo = orario;
    progetto.periodi[2].meteo = guasto;
    ombra::progetto::scrivi(&dir, &progetto).unwrap();
    let rapporto = corsa::esegui_progetto(&dir).unwrap();

    assert_eq!(rapporto.corse.len(), 3);
    assert_eq!(rapporto.corse[0].errore, None);
    assert_eq!(rapporto.corse[1].errore, None);
    let errore = rapporto.corse[2]
        .errore
        .as_deref()
        .expect("il terzo Periodo ha un file che non è orario");
    assert!(errore.contains("guasto.epw"), "{errore}");
    assert!(errore.contains("«30»"), "{errore}");

    // Each file is an Ingresso once, however many Periodi name it.
    let tabella = rileggi(&rapporto.corse[0].giornale);
    let meteo = tabella["ingresso"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["ruolo"].as_str() == Some("meteo"))
        .count();
    assert_eq!(meteo, 2);
}

#[test]
fn il_fuso_orario_della_corsa_viene_dal_file_meteo() {
    // Twelve degrees and a half of longitude round to a zone of +1, so a file
    // that declares +3 is the only way to tell a read zone from a guessed one.
    // Three hours are forty-five degrees of angolo orario: the sun of a Corsa
    // that guessed would be somewhere else entirely.
    let dir = tempdir_di_prova("fuso-dal-meteo");
    let epw = scrivi_epw(&dir, 3.0);
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    progetto.periodi[0].meteo = epw;
    ombra::progetto::scrivi(&dir, &progetto).unwrap();
    let rapporto = corsa::esegui_progetto(&dir).unwrap();
    let tabella = rileggi(&rapporto.corse[0].giornale);

    assert_eq!(tabella["sole"]["fuso_ore"].as_float(), Some(3.0));
    assert_eq!(tabella["sole"]["fuso_da"].as_str(), Some("file meteo"));
}

#[test]
fn un_fuso_a_mezz_ora_arriva_intero_fino_al_giornale() {
    // Kathmandu is at +5.75 and Adelaide at +9.5. The guess from the longitude
    // could only ever produce whole hours, so the half-hour zones were not
    // approximated: they were unrepresentable.
    let dir = tempdir_di_prova("fuso-a-mezz-ora");
    let epw = scrivi_epw(&dir, 5.75);
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    progetto.periodi[0].meteo = epw;
    ombra::progetto::scrivi(&dir, &progetto).unwrap();
    let rapporto = corsa::esegui_progetto(&dir).unwrap();
    assert_eq!(
        rileggi(&rapporto.corse[0].giornale)["sole"]["fuso_ore"].as_float(),
        Some(5.75)
    );
}

#[test]
fn senza_file_meteo_il_fuso_torna_alla_longitudine_e_il_giornale_lo_dice() {
    // The Corsa still runs — the weather file is an Ingresso and its absence is
    // already recorded — but the number it used is a guess, and a Giornale that
    // showed only the number would not let anyone tell the two apart.
    let dir = tempdir_di_prova("fuso-senza-meteo");
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    progetto.periodi[0].meteo = dir.join("questo-file-non-esiste.epw");
    ombra::progetto::scrivi(&dir, &progetto).unwrap();
    let rapporto = corsa::esegui_progetto(&dir).unwrap();
    let tabella = rileggi(&rapporto.corse[0].giornale);

    assert_eq!(tabella["sole"]["fuso_ore"].as_float(), Some(1.0));
    let fonte = tabella["sole"]["fuso_da"].as_str().unwrap();
    assert!(fonte.contains("longitudine"), "{fonte}");
    assert!(fonte.contains("non si è potuto leggere"), "{fonte}");
    // No file, no minute to report, and a note that does not go on about what
    // the file declares. The instant of the sun is OMBRA's own and is declared
    // all the same.
    assert!(tabella["sole"].get("minuti_del_file_meteo").is_none());
    let nota = tabella["sole"]["nota"].as_str().unwrap();
    assert!(!nota.contains("file"), "{nota}");
    assert!(nota.contains("k + 0:30"), "{nota}");
    assert_eq!(tabella["sole"]["istante"].as_str(), Some("centro dell'Ora"));
}

fn punto(id: u32, posizione_m: (f64, f64)) -> PuntoDiOsservazione {
    PuntoDiOsservazione {
        id,
        posizione_m,
        etichetta: format!("punto {id}"),
    }
}

#[test]
fn ogni_punto_di_osservazione_porta_la_sua_serie_ora_per_ora() {
    // The map answers "where" and the series answers "when". A courtyard that
    // gains two hours of sun in the morning and loses two in the afternoon has
    // the same daily mean as one where nothing changed, and only the series
    // tells them apart.
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 24)]);
    progetto.punti = vec![punto(1, (0.5, 0.5)), punto(2, (5.5, 5.5))];
    let (_, rapporto) = scrivi_ed_esegui("serie-dei-punti", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);

    assert_eq!(tabella["punti"]["dichiarati"].as_integer(), Some(2));
    assert_eq!(tabella["punti"]["dentro_la_griglia"].as_integer(), Some(2));

    let serie = tabella["serie"].as_array().unwrap();
    assert_eq!(serie.len(), 2);
    for (indice, una) in serie.iter().enumerate() {
        assert_eq!(una["punto"].as_integer(), Some(indice as i64 + 1));
        let valori = una["frazione_illuminata"].as_array().unwrap();
        assert_eq!(valori.len(), 24, "una serie per ogni ora del Periodo");
        assert!(
            valori
                .iter()
                .all(|v| (0.0..=1.0).contains(&v.as_float().unwrap())),
            "una frazione illuminata fuori da 0..1"
        );
    }

    // A day has a night in it: some hour of the twenty-four is dark, in every
    // cell. A series of all ones would mean the sun never set on that Punto.
    let primo = serie[0]["frazione_illuminata"].as_array().unwrap();
    assert!(primo.iter().any(|v| v.as_float() == Some(0.0)));
    assert!(primo.iter().any(|v| v.as_float().unwrap() > 0.0));
}

#[test]
fn la_serie_di_un_punto_somma_alle_ore_di_sole_della_sua_cella() {
    // The two outputs are the same numbers seen twice, and this is what says
    // so: if the series were read from another cell — a swapped row and column
    // is the mistake this project is built to fear — the sums would part.
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 12)]);
    // Off the diagonal on purpose: on a cell where row and column are equal, a
    // swap of the two would leave the answer right.
    progetto.punti = vec![punto(1, (1.5, 4.5))];
    let (_, rapporto) = scrivi_ed_esegui("serie-somma", &progetto);

    let campi = rapporto.corse[0].campi.as_ref().unwrap();
    let serie = &campi.serie[0];
    let somma: f64 = serie.frazione_illuminata.iter().sum();
    let dalla_mappa = f64::from(campi.ore_di_sole[[serie.riga, serie.colonna]]);
    assert!(
        (somma - dalla_mappa).abs() < 0.01,
        "la serie somma {somma} e la mappa dice {dalla_mappa}"
    );
}

#[test]
fn un_punto_fuori_dalla_griglia_non_arriva_mai_a_una_corsa() {
    // It is refused where a Progetto is written, so no Corsa ever has to decide
    // what to do with it. The alternative — accept it and serve the nearest
    // cell — would attribute a number to a place nobody asked about.
    let dir = tempdir_di_prova("punto-fuori");
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    progetto.punti = vec![punto(1, (2.5, 2.5)), punto(2, (99.0, 99.0))];
    let errore = ombra::progetto::scrivi(&dir, &progetto)
        .unwrap_err()
        .to_string();
    assert!(errore.contains("punto di osservazione 2"), "{errore}");
}

#[test]
fn un_punto_sul_bordo_lontano_e_fuori_come_ovunque_nel_progetto() {
    // Half-open intervals: minimum included, maximum excluded, which is the
    // coverage rule of the Derivazione. With the maximum included, a Punto at
    // x = 6 on a Griglia of six cells passed the check and then had no cell —
    // accepted by one rule, invisible to the other.
    let dir = tempdir_di_prova("punto-sul-bordo");
    let mut progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    progetto.punti = vec![punto(1, (6.0, 3.0))];
    assert!(ombra::progetto::scrivi(&dir, &progetto).is_err());

    // Just inside, it is accepted and it gets its series.
    progetto.punti = vec![punto(1, (5.999, 3.0))];
    let (_, rapporto) = scrivi_ed_esegui("punto-quasi-sul-bordo", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);
    assert_eq!(tabella["punti"]["dentro_la_griglia"].as_integer(), Some(1));
    assert_eq!(
        tabella["serie"].as_array().unwrap()[0]["colonna"].as_integer(),
        Some(5)
    );
}

#[test]
fn un_progetto_senza_punti_dichiara_zero_e_non_tace() {
    let progetto = progetto_di_prova(6, vec![periodo("estate", 7, 15, 2)]);
    let (_, rapporto) = scrivi_ed_esegui("senza-punti", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);
    assert_eq!(tabella["punti"]["dichiarati"].as_integer(), Some(0));
    assert!(tabella.get("serie").is_none(), "nessuna serie da scrivere");
}

#[test]
fn l_ombra_cade_dalla_parte_opposta_al_sole_calcolato_a_parte() {
    let progetto = progetto_di_prova(21, vec![periodo("estate", 7, 15, 24)]);
    let (_, rapporto) = scrivi_ed_esegui("verifica-ombra", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);

    let verifica = tabella["verifica_ombra"].as_table().unwrap();
    let ore = verifica["ore_verificate"].as_integer().unwrap();
    assert!(
        ore >= 6,
        "il controllo deve avere ore da controllare, non {ore}"
    );
    let scarto = verifica["scarto_massimo_gradi"].as_float().unwrap();
    assert!(
        scarto < 45.0,
        "l'ombra deve cadere dalla parte opposta al sole, scarto {scarto}°"
    );
    assert_eq!(verifica["bandiera"].as_bool(), Some(false));
    assert_eq!(
        verifica["notte_tutta_in_ombra"].as_bool(),
        Some(true),
        "col sole sotto l'orizzonte non c'è nessuna cella illuminata"
    );
}

#[test]
#[ignore = "il cancello dei 60 secondi: legge il caso di riferimento versionato e va misurato in release"]
fn il_caso_di_riferimento_sta_sotto_i_sessanta_secondi() {
    let inizio = std::time::Instant::now();
    let rapporto = corsa::esegui_caso_di_riferimento().unwrap();
    let totale = inizio.elapsed();

    println!("--- cancello del caso di riferimento ---");
    println!("Corse: {}", rapporto.corse.len());
    println!("Derivazioni: {}", rapporto.derivazioni);
    println!("totale ............ {:?}", totale);
    println!("  Derivazione ..... {:?}", rapporto.tempo_derivazione);
    println!("  Motore .......... {:?}", rapporto.tempo_motore);
    println!("  scrittura ....... {:?}", rapporto.tempo_scrittura);
    for esito in &rapporto.corse {
        println!(
            "  {} → {}",
            esito.etichetta,
            esito.errore.as_deref().unwrap_or("riuscita")
        );
    }

    assert_eq!(rapporto.corse.len(), 4, "due Scenari per due Periodi");
    for esito in &rapporto.corse {
        assert_eq!(esito.errore, None, "{}", esito.etichetta);
    }
    assert!(
        totale < std::time::Duration::from_secs(60),
        "il caso di riferimento ha impiegato {totale:?}"
    );
}

/// Una verifica che non ha potuto girare non deve somigliare a una passata.
/// `scarto_massimo` resta a zero quando nessun'ora è verificabile, e zero si
/// legge come accordo perfetto: la bandiera deve alzarsi lo stesso.
#[test]
fn una_verifica_mai_eseguita_alza_la_bandiera() {
    // Uno Scenario senza Edifici: nessun volume di cui confrontare l'ombra,
    // quindi nessun'ora verificabile.
    let mut progetto = progetto_di_prova(6, vec![periodo("luglio", 7, 15, 6)]);
    progetto.scenari[0].edifici.clear();
    let (dir, rapporto) = scrivi_ed_esegui("verifica-mai-eseguita", &progetto);

    let giornale = rileggi(
        &dir.join("corse")
            .join(rapporto.corse[0].impronta.testo())
            .join("giornale.toml"),
    );
    let v = giornale["verifica_ombra"].as_table().unwrap();
    assert_eq!(
        v["ore_verificate"].as_integer(),
        Some(0),
        "senza Edifici non c'è niente da verificare"
    );
    assert_eq!(
        v["bandiera"].as_bool(),
        Some(true),
        "zero ore verificate deve alzare la bandiera, non lasciarla bassa"
    );
}

#[test]
fn il_giornale_riporta_il_cielo_visibile_con_e_senza_le_chiome() {
    // A sky view factor does not depend on the hour, so it is the one field of
    // the radiative chain a Corsa can report today without a Motore that
    // computes radiation. Two fields and not one: masonry is permanent and a
    // tree is a decision, and a study that cannot tell them apart cannot argue
    // for the planting.
    let mut progetto = progetto_di_prova(9, vec![periodo("estate", 7, 15, 4)]);
    progetto.scenari[0].alberi = vec![albero_di_prova("QURO", (2.5, 2.5))];
    let (_, rapporto) = scrivi_ed_esegui("cielo-visibile", &progetto);
    let tabella = rileggi(&rapporto.corse[0].giornale);

    let campi = tabella["campo"].as_array().unwrap();
    let cercato = |nome: &str| {
        campi
            .iter()
            .find(|c| c["campo"].as_str() == Some(nome))
            .unwrap_or_else(|| panic!("il Giornale non riporta il campo «{nome}»"))
    };
    for nome in ["sky view factor", "sky view factor con le chiome"] {
        let campo = cercato(nome);
        let valore = |chiave: &str| campo[chiave].as_float().unwrap();
        assert!(
            (0.0..=1.0).contains(&valore("minimo")) && (0.0..=1.0).contains(&valore("massimo")),
            "«{nome}» esce da [0, 1]: {} … {}",
            valore("minimo"),
            valore("massimo")
        );
        assert_eq!(
            campo["fuori_intervallo"].as_bool(),
            Some(false),
            "«{nome}» alza la bandiera su un Progetto che non ha niente di strano"
        );
    }
    assert!(
        cercato("sky view factor con le chiome")["media"].as_float()
            < cercato("sky view factor")["media"].as_float(),
        "un albero non toglie cielo a nessuno"
    );
}

#[test]
fn ogni_punto_di_osservazione_porta_il_suo_sky_view_factor() {
    // Read from the map at the Punto's own cell, so a swapped row and column
    // parts the two numbers — the same defence the hourly series carries, on
    // the field where a mistake would be hardest to see, because a sky view
    // factor has no night to give it away.
    let mut progetto = progetto_di_prova(9, vec![periodo("estate", 7, 15, 4)]);
    progetto.scenari[0].alberi = vec![albero_di_prova("QURO", (2.5, 2.5))];
    progetto.punti = vec![punto(1, (1.5, 4.5))];
    let (_, rapporto) = scrivi_ed_esegui("cielo-dei-punti", &progetto);

    let campi = rapporto.corse[0].campi.as_ref().unwrap();
    let serie = &campi.serie[0];
    let dalla_mappa = f64::from(campi.sky_view_factor.con_le_chiome[[serie.riga, serie.colonna]]);
    assert!(
        (serie.sky_view_factor - dalla_mappa).abs() < 0.01,
        "il Punto dice {} e la mappa dice {dalla_mappa}",
        serie.sky_view_factor
    );
    assert!(
        (0.0..=1.0).contains(&serie.sky_view_factor),
        "un sky view factor fuori da [0, 1]: {}",
        serie.sky_view_factor
    );
}
