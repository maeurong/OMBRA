//! The weather file: EnergyPlus Weather, read without a dependency.
//!
//! An EPW is a text file with eight header lines and one record per hour of a
//! typical year. OMBRA reads two things from it: the place — latitude,
//! longitude and **time zone** — which the header carries, and the hourly
//! values a Periodo asks for.
//!
//! **The time zone is the reason this module exists before the radiation does.**
//! Until now the Corsa guessed the zone from the longitude, and a guess of one
//! hour moves every shadow by fifteen degrees of hour angle. The EPW states it,
//! and the file is already an Ingresso of the Corsa with its own checksum.
//!
//! **Missing is not zero.** An EPW writes a missing value as a sentinel — 99.9
//! for the air temperature, 999 for the humidity and the wind, 9999 for the
//! three radiations — and those numbers are inside the range a reader would
//! accept. Every field that can be missing comes back as `None`, so a sentinel
//! cannot be summed, averaged or plotted as if it had been measured.

use crate::dominio::Data;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The hourly record an EPW carries, cut down to what OMBRA consumes.
///
/// `None` is a value the file declares missing, never a value OMBRA failed to
/// parse: a field that is present and not a number fails the whole read.
#[derive(Debug, Clone, PartialEq)]
pub struct ValoriOrari {
    pub mese: u32,
    pub giorno: u32,
    /// The EPW hour, 1 to 24. Record `h` is the average over the hour that ends
    /// at `h`, so it covers local standard time from `h - 1` to `h`. The Corsa
    /// computes the sun of that hour at its centre, `h - 0:30`: the decision is
    /// ticket #35 of `maeurong/OMBRA`. The reader below refuses a file whose
    /// minute field is not 0 or 60, or that carries two records for one hour:
    /// in a file that is not hourly that centre would mean nothing.
    pub ora: u32,
    /// Dry bulb air temperature, degrees Celsius.
    pub temperatura_c: Option<f64>,
    /// Relative humidity, per cent.
    pub umidita_relativa: Option<f64>,
    /// Global horizontal radiation, W/m2.
    pub globale_orizzontale_wm2: Option<f64>,
    /// Direct normal radiation, W/m2.
    pub diretta_normale_wm2: Option<f64>,
    /// Diffuse horizontal radiation, W/m2.
    pub diffusa_orizzontale_wm2: Option<f64>,
    /// Wind speed, m/s, at the ten metres the file is written for — which is
    /// also the height UTCI asks for, so nothing extrapolates it.
    pub vento_ms: Option<f64>,
    /// Wind direction, degrees from north clockwise.
    pub direzione_vento_gradi: Option<f64>,
}

/// A weather file, header and hours.
#[derive(Debug, Clone)]
pub struct Epw {
    pub percorso: PathBuf,
    pub stazione: String,
    pub latitudine_gradi: f64,
    pub longitudine_gradi: f64,
    /// Hours ahead of UTC, as the file states it. Standard time all year: an EPW
    /// never shifts its hours for daylight saving.
    pub fuso_ore: f64,
    pub quota_m: f64,
    /// The values the minute field takes across the records, each once, in
    /// ascending order: `[0]`, `[60]` or both. The hour comes from the hour
    /// field in both cases, so this moves nothing. It is kept for the
    /// Giornale, which reports what the file said.
    pub minuti: Vec<u32>,
    ore: Vec<ValoriOrari>,
}

#[derive(Debug)]
pub enum MeteoError {
    NonLeggibile {
        percorso: PathBuf,
        causa: String,
    },
    /// The `LOCATION` line is missing, is not the first line, or does not carry
    /// the nine fields that hold the place.
    SenzaLuogo {
        percorso: PathBuf,
    },
    /// A number the header or a record must have, and does not.
    NonNumerico {
        percorso: PathBuf,
        riga: usize,
        campo: &'static str,
        testo: String,
    },
    /// A record with fewer fields than the columns OMBRA reads.
    RigaCorta {
        percorso: PathBuf,
        riga: usize,
        campi: usize,
    },
    SenzaOre {
        percorso: PathBuf,
    },
    /// A record whose minute field is anything but 0 or 60, a number or not: a
    /// sub-hourly file, or one that samples instants. OMBRA reads hourly means.
    MinutoNonOrario {
        percorso: PathBuf,
        riga: usize,
        testo: String,
    },
    /// A second record for a year, month, day and hour the file already
    /// carries at line `prima`.
    OraRipetuta {
        percorso: PathBuf,
        riga: usize,
        prima: usize,
        mese: u32,
        giorno: u32,
        ora: u32,
    },
    /// A Periodo whose first date the file has no record for. An EPW carries no
    /// 29 February, so a Periodo starting there ends up here.
    GiornoAssente {
        percorso: PathBuf,
        giorno: String,
    },
}

impl fmt::Display for MeteoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonLeggibile { percorso, causa } => {
                write!(f, "{}: non si legge: {causa}", percorso.display())
            }
            Self::SenzaLuogo { percorso } => write!(
                f,
                "{}: la prima riga non è un LOCATION con latitudine, longitudine e fuso: \
                 senza fuso orario le ore del file non sono collocabili nel giorno",
                percorso.display()
            ),
            Self::NonNumerico {
                percorso,
                riga,
                campo,
                testo,
            } => write!(
                f,
                "{}, riga {riga}: «{testo}» non è un numero, e lì ci va {campo}",
                percorso.display()
            ),
            Self::RigaCorta {
                percorso,
                riga,
                campi,
            } => write!(
                f,
                "{}, riga {riga}: {campi} campi invece dei {} che servono",
                percorso.display(),
                CAMPI_MINIMI
            ),
            Self::SenzaOre { percorso } => write!(
                f,
                "{}: nessuna riga di dati dopo le otto di intestazione",
                percorso.display()
            ),
            Self::GiornoAssente { percorso, giorno } => write!(
                f,
                "{}: non porta nessuna ora del {giorno}. Un file EPW è un anno tipo di \
                 8760 ore e non ha il 29 febbraio",
                percorso.display()
            ),
            Self::MinutoNonOrario {
                percorso,
                riga,
                testo,
            } => write!(
                f,
                "{}, riga {riga}: il campo minuto è «{testo}», e un file orario porta 0 o \
                 60. OMBRA legge solo file orari: serve un EPW con una riga di dati per \
                 ora",
                percorso.display()
            ),
            Self::OraRipetuta {
                percorso,
                riga,
                prima,
                mese,
                giorno,
                ora,
            } => write!(
                f,
                "{}, riga {riga}: due righe di dati per il {giorno}/{mese}, ora {ora}: \
                 la prima è alla riga {prima}. Un file orario ne porta una per ora: se è \
                 un doppione va tolto, se il file è sub-orario serve la sua versione \
                 oraria",
                percorso.display()
            ),
        }
    }
}

impl std::error::Error for MeteoError {}

impl MeteoError {
    /// Whether the file was read and found not to be hourly, as opposed to not
    /// read at all. The Corsa treats the two differently: without a file it
    /// still runs, on a time zone it guesses and declares, while a file that is
    /// not hourly would pair every Ora with the wrong record once a Corsa reads
    /// them.
    pub fn non_orario(&self) -> bool {
        // No wildcard arm: a new variant has to say which of the two it is.
        match self {
            Self::MinutoNonOrario { .. } | Self::OraRipetuta { .. } => true,
            Self::NonLeggibile { .. }
            | Self::SenzaLuogo { .. }
            | Self::NonNumerico { .. }
            | Self::RigaCorta { .. }
            | Self::SenzaOre { .. }
            | Self::GiornoAssente { .. } => false,
        }
    }
}

/// The eight header lines of an EPW, of which OMBRA reads only the first.
const RIGHE_DI_INTESTAZIONE: usize = 8;

/// Fields up to and including the wind speed, which is the last column OMBRA
/// reads. A record is allowed to be longer, never shorter.
const CAMPI_MINIMI: usize = 22;

/// The value an EPW writes when a field was not measured. Each column has its
/// own, and they all sit inside the range a reader would otherwise accept.
const ASSENTE_TEMPERATURA: f64 = 99.9;
const ASSENTE_PERCENTO: f64 = 999.0;
const ASSENTE_RADIAZIONE: f64 = 9999.0;
const ASSENTE_VENTO: f64 = 999.0;
const ASSENTE_DIREZIONE: f64 = 999.0;

fn numero(
    testo: &str,
    percorso: &Path,
    riga: usize,
    campo: &'static str,
) -> Result<f64, MeteoError> {
    testo
        .trim()
        .parse::<f64>()
        .map_err(|_| MeteoError::NonNumerico {
            percorso: percorso.to_path_buf(),
            riga,
            campo,
            testo: testo.trim().to_owned(),
        })
}

/// A field as an error message quotes it: without control characters, and cut
/// short. The message goes to a terminal and into every Giornale of the
/// Progetto, and the field comes from a file somebody else may have written.
fn citato(testo: &str) -> String {
    const CARATTERI: usize = 32;
    let mut caratteri = testo.trim().chars().filter(|c| !c.is_control());
    let mut citato: String = caratteri.by_ref().take(CARATTERI).collect();
    if caratteri.next().is_some() {
        citato.push('…');
    }
    citato
}

/// A value, unless the file says it is the sentinel that means "not measured".
fn misurato(valore: f64, assente: f64) -> Option<f64> {
    (valore != assente).then_some(valore)
}

impl Epw {
    pub fn leggi(percorso: &Path) -> Result<Self, MeteoError> {
        let testo = std::fs::read_to_string(percorso).map_err(|e| MeteoError::NonLeggibile {
            percorso: percorso.to_path_buf(),
            causa: e.to_string(),
        })?;
        Self::da_testo(percorso, &testo)
    }

    /// The body of [`Epw::leggi`], separated so the tests can hand over a file
    /// they wrote in the test itself: the reference EPW is not in the
    /// repository and never will be.
    pub fn da_testo(percorso: &Path, testo: &str) -> Result<Self, MeteoError> {
        let mut righe = testo.lines();
        let prima = righe.next().unwrap_or_default();
        let luogo: Vec<&str> = prima.split(',').collect();
        if !luogo
            .first()
            .is_some_and(|c| c.trim().eq_ignore_ascii_case("LOCATION"))
            || luogo.len() < 10
        {
            return Err(MeteoError::SenzaLuogo {
                percorso: percorso.to_path_buf(),
            });
        }
        let latitudine_gradi = numero(luogo[6], percorso, 1, "la latitudine")?;
        let longitudine_gradi = numero(luogo[7], percorso, 1, "la longitudine")?;
        let fuso_ore = numero(luogo[8], percorso, 1, "il fuso orario")?;
        let quota_m = numero(luogo[9], percorso, 1, "la quota della stazione")?;

        let mut ore = Vec::new();
        let mut minuti = Vec::new();
        // The line each hour was first seen at. The year is in the key as the
        // file writes it, so a file of several years still has one record per
        // hour, and so does a typical year stitched from months of different
        // years.
        let mut viste: HashMap<(&str, u32, u32, u32), usize> = HashMap::new();
        for (indice, riga) in testo.lines().enumerate().skip(RIGHE_DI_INTESTAZIONE) {
            let riga = riga.trim();
            if riga.is_empty() {
                continue;
            }
            let numero_di_riga = indice + 1;
            let campi: Vec<&str> = riga.split(',').collect();
            if campi.len() < CAMPI_MINIMI {
                return Err(MeteoError::RigaCorta {
                    percorso: percorso.to_path_buf(),
                    riga: numero_di_riga,
                    campi: campi.len(),
                });
            }
            let leggi = |colonna: usize, campo: &'static str| {
                numero(campi[colonna - 1], percorso, numero_di_riga, campo)
            };
            let (mese, giorno, ora) = (
                leggi(2, "il mese")? as u32,
                leggi(3, "il giorno")? as u32,
                leggi(4, "l'ora")? as u32,
            );
            // The minute is compared as read, not truncated: 0.5 is not 0. A field
            // that is not a number is not 0 or 60 either, and ends up here
            // rather than in `NonNumerico`, which the Corsa survives.
            let minuto = match campi[4].trim().parse::<f64>() {
                Ok(minuto) if minuto == 0.0 || minuto == 60.0 => minuto as u32,
                _ => {
                    return Err(MeteoError::MinutoNonOrario {
                        percorso: percorso.to_path_buf(),
                        riga: numero_di_riga,
                        testo: citato(campi[4]),
                    })
                }
            };
            // Against the whole file and not only the record before: a block
            // pasted twice repeats nothing next to itself, and `ore_dal` counts
            // records in the order they come.
            if let Some(prima) = viste.insert((campi[0].trim(), mese, giorno, ora), numero_di_riga)
            {
                return Err(MeteoError::OraRipetuta {
                    percorso: percorso.to_path_buf(),
                    riga: numero_di_riga,
                    prima,
                    mese,
                    giorno,
                    ora,
                });
            }
            if let Err(posto) = minuti.binary_search(&minuto) {
                minuti.insert(posto, minuto);
            }
            ore.push(ValoriOrari {
                mese,
                giorno,
                ora,
                temperatura_c: misurato(leggi(7, "la temperatura dell'aria")?, ASSENTE_TEMPERATURA),
                umidita_relativa: misurato(leggi(9, "l'umidità relativa")?, ASSENTE_PERCENTO),
                globale_orizzontale_wm2: misurato(
                    leggi(14, "la radiazione globale orizzontale")?,
                    ASSENTE_RADIAZIONE,
                ),
                diretta_normale_wm2: misurato(
                    leggi(15, "la radiazione diretta normale")?,
                    ASSENTE_RADIAZIONE,
                ),
                diffusa_orizzontale_wm2: misurato(
                    leggi(16, "la radiazione diffusa orizzontale")?,
                    ASSENTE_RADIAZIONE,
                ),
                vento_ms: misurato(leggi(22, "la velocità del vento")?, ASSENTE_VENTO),
                direzione_vento_gradi: misurato(
                    leggi(21, "la direzione del vento")?,
                    ASSENTE_DIREZIONE,
                ),
            });
        }
        if ore.is_empty() {
            return Err(MeteoError::SenzaOre {
                percorso: percorso.to_path_buf(),
            });
        }

        Ok(Self {
            percorso: percorso.to_path_buf(),
            stazione: luogo[1].trim().to_owned(),
            latitudine_gradi,
            longitudine_gradi,
            fuso_ore,
            quota_m,
            minuti,
            ore,
        })
    }

    pub fn ore_totali(&self) -> usize {
        self.ore.len()
    }

    /// The `ore` records a Periodo asks for, beginning at the first hour of
    /// `inizio`.
    ///
    /// **The year runs in a circle.** A Periodo of 48 hours starting on 31
    /// December needs the first of January, and an EPW is a *typical* year, not
    /// a calendar one: the file has no next year to read, and the hour after
    /// its last is its first. A Corsa that wraps is computing December weather
    /// on a January record, which is what every tool that reads a typical year
    /// does, and the Giornale is where that has to be said.
    pub fn ore_dal(&self, inizio: Data, ore: u32) -> Result<Vec<&ValoriOrari>, MeteoError> {
        let primo = self
            .ore
            .iter()
            .position(|v| v.mese == inizio.mese && v.giorno == inizio.giorno)
            .ok_or_else(|| MeteoError::GiornoAssente {
                percorso: self.percorso.clone(),
                giorno: format!("{}/{}", inizio.giorno, inizio.mese),
            })?;
        Ok((0..ore as usize)
            .map(|passo| &self.ore[(primo + passo) % self.ore.len()])
            .collect())
    }

    /// Whether a Periodo of `ore` hours from `inizio` runs off the end of the
    /// file and comes back at its first hour.
    pub fn si_avvolge(&self, inizio: Data, ore: u32) -> bool {
        self.ore
            .iter()
            .position(|v| v.mese == inizio.mese && v.giorno == inizio.giorno)
            .is_some_and(|primo| primo + ore as usize > self.ore.len())
    }
}
