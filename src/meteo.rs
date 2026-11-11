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
    /// ticket #35 of `maeurong/OMBRA`. The reader below refuses a file that is
    /// not hourly, as [`MeteoError::non_orario`] lists: in such a file that
    /// centre would mean nothing.
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

/// The month, day and hour of a record, as an error names them. Fields with
/// names, because a month and a day swapped would still be a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeseGiornoOra {
    pub mese: u32,
    pub giorno: u32,
    pub ora: u32,
}

impl fmt::Display for MeseGiornoOra {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "il {}/{}, ora {}", self.giorno, self.mese, self.ora)
    }
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
    /// A month, day or hour field that is not a whole number from 1 to
    /// `massimo`, a number or not: month 13, 31 April, hour 0, hour 13.5.
    FuoriCalendario {
        percorso: PathBuf,
        riga: usize,
        campo: &'static str,
        testo: String,
        massimo: u32,
    },
    /// A record that is not the hour after the record before it: hours are
    /// missing, or out of order.
    OraFuoriPosto {
        percorso: PathBuf,
        riga: usize,
        attesa: MeseGiornoOra,
        trovata: MeseGiornoOra,
    },
    /// A Periodo whose first date the file has no record for. A typical year
    /// carries no 29 February, so a Periodo starting there ends up here.
    GiornoAssente {
        percorso: PathBuf,
        giorno: String,
    },
    /// A Periodo that reaches a day the file carries only part of, at its
    /// first record or where the Periodo wraps: the file is hourly and was
    /// read, and the hour the Periodo counts is not the one the file has
    /// there.
    GiornoMonco {
        percorso: PathBuf,
        attesa: MeseGiornoOra,
        trovata: MeseGiornoOra,
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
                "{}: non porta nessuna ora del {giorno}. Un anno tipo non ha il 29 \
                 febbraio, e un file ritagliato porta solo i suoi giorni: il Periodo deve \
                 cominciare in un giorno che il file ha",
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
            Self::FuoriCalendario {
                percorso,
                riga,
                campo,
                testo,
                massimo,
            } => write!(
                f,
                "{}, riga {riga}: {campo} è «{testo}», e lì ci va un numero intero da 1 a \
                 {massimo}",
                percorso.display()
            ),
            Self::OraFuoriPosto {
                percorso,
                riga,
                attesa,
                trovata,
            } => write!(
                f,
                "{}, riga {riga}: qui ci va {attesa}{}, e c'è {trovata}. Un file orario \
                 porta tutte le ore, una dopo l'altra: se ne mancano serve un file che le \
                 abbia, se sono fuori ordine vanno rimesse in fila",
                percorso.display(),
                o_il_primo_marzo(*attesa)
            ),
            Self::GiornoMonco {
                percorso,
                attesa,
                trovata,
            } => write!(
                f,
                "{}: il Periodo chiede {attesa}{}, e il file a quel punto porta {trovata}. \
                 Il file comincia o finisce a metà di un giorno, e un Periodo che arriva in \
                 fondo al file ricomincia dal suo primo record: serve un file con giorni \
                 interi, o un Periodo che stia nei giorni che il file porta interi",
                percorso.display(),
                o_il_primo_marzo(*attesa)
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
    /// them. Not hourly is a minute that is not 0 or 60, a month, day or hour
    /// outside the calendar, an hour written twice, an hour missing or out of
    /// order. `GiornoAssente` and `GiornoMonco` answer no: they come from
    /// `ore_dal`, on a file that was read, and say a Periodo does not fit in
    /// it.
    pub fn non_orario(&self) -> bool {
        // No wildcard arm: a new variant has to say which of the two it is.
        match self {
            Self::MinutoNonOrario { .. }
            | Self::OraRipetuta { .. }
            | Self::FuoriCalendario { .. }
            | Self::OraFuoriPosto { .. } => true,
            Self::NonLeggibile { .. }
            | Self::SenzaLuogo { .. }
            | Self::NonNumerico { .. }
            | Self::RigaCorta { .. }
            | Self::SenzaOre { .. }
            | Self::GiornoAssente { .. }
            | Self::GiornoMonco { .. } => false,
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

/// A field as an error message quotes it: printable ASCII only, and cut
/// short. The message goes to a terminal and into every Giornale of the
/// Progetto, and the field comes from a file somebody else may have written.
/// Anything else is shown as U+FFFD and not dropped: a control character
/// would rewrite the line, a Unicode one can reorder or hide what follows,
/// and a field with one taken out could read as a number it is not.
fn citato(testo: &str) -> String {
    const CARATTERI: usize = 32;
    let mut caratteri = testo.trim().chars().map(|c| {
        if c.is_ascii_graphic() || c == ' ' {
            c
        } else {
            char::REPLACEMENT_CHARACTER
        }
    });
    let mut citato: String = caratteri.by_ref().take(CARATTERI).collect();
    if caratteri.next().is_some() {
        citato.push('…');
    }
    citato
}

/// The last day of each month. February has 29: a leap-year file carries it,
/// and a typical year takes each month from a different year, so the reader
/// does not ask whether the year is leap.
const GIORNI_DEL_MESE: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// A month, day or hour field: a whole number from 1 to `massimo`, however it
/// is spelled. Compared as read, so 13.5 is not 13 and NaN is not 0.
fn intero_fino_a(testo: &str, massimo: u32) -> Option<u32> {
    let valore = testo.trim().parse::<f64>().ok()?;
    (valore.fract() == 0.0 && (1.0..=f64::from(massimo)).contains(&valore)).then_some(valore as u32)
}

/// The month, day and hour after these, which have to be in the calendar.
/// After the last hour of 28 February this is 29 February.
fn ora_dopo(mese: u32, giorno: u32, ora: u32) -> MeseGiornoOra {
    let (mese, giorno, ora) = if ora < 24 {
        (mese, giorno, ora + 1)
    } else if giorno < GIORNI_DEL_MESE[mese as usize - 1] {
        (mese, giorno + 1, 1)
    } else {
        (mese % 12 + 1, 1, 1)
    };
    MeseGiornoOra { mese, giorno, ora }
}

/// Whether `trovata` is where `attesa` says, or is the one other place the
/// calendar allows: 29 February is there in a leap year and in no other, so
/// after the 28th both it and 1 March are the hour after.
fn al_suo_posto(attesa: MeseGiornoOra, trovata: MeseGiornoOra) -> bool {
    trovata == attesa || (attesa == VENTINOVE_FEBBRAIO && trovata == PRIMO_MARZO)
}

/// What a message adds after naming `attesa`, so it never tells the owner of
/// a typical year that a 29 February is missing.
fn o_il_primo_marzo(attesa: MeseGiornoOra) -> &'static str {
    if attesa == VENTINOVE_FEBBRAIO {
        " (o il 1/3, ora 1)"
    } else {
        ""
    }
}

const VENTINOVE_FEBBRAIO: MeseGiornoOra = MeseGiornoOra {
    mese: 2,
    giorno: 29,
    ora: 1,
};
const PRIMO_MARZO: MeseGiornoOra = MeseGiornoOra {
    mese: 3,
    giorno: 1,
    ora: 1,
};

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

        let mut ore: Vec<ValoriOrari> = Vec::new();
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
            // As for the minute below: a field that is not a number ends up
            // here rather than in `NonNumerico`, which the Corsa survives.
            let del_calendario = |colonna: usize, campo: &'static str, massimo: u32| {
                intero_fino_a(campi[colonna - 1], massimo).ok_or_else(|| {
                    MeteoError::FuoriCalendario {
                        percorso: percorso.to_path_buf(),
                        riga: numero_di_riga,
                        campo,
                        testo: citato(campi[colonna - 1]),
                        massimo,
                    }
                })
            };
            let mese = del_calendario(2, "il mese", 12)?;
            let giorno = del_calendario(3, "il giorno", GIORNI_DEL_MESE[mese as usize - 1])?;
            let ora = del_calendario(4, "l'ora", 24)?;
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
            // Against the whole file and not only the record before, and ahead
            // of the sequence check below: a record written twice is then
            // reported as a repeat and not as an hour out of place. The
            // sequence check also leaves the year out, so a whole year pasted
            // twice passes it as two years, and this is what stops it. The
            // year is compared as text: two copies that spell it differently,
            // `2005` and `2005.0`, are two years here.
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
            // Every record is the hour after the one before it, so `ore_dal`
            // can count records and call them hours. The year is left out: a
            // file of several years goes from 31 December to 1 January, and so
            // does a typical year whose months come from different years.
            if let Some(prima) = ore.last() {
                let attesa = ora_dopo(prima.mese, prima.giorno, prima.ora);
                let trovata = MeseGiornoOra { mese, giorno, ora };
                if !al_suo_posto(attesa, trovata) {
                    return Err(MeteoError::OraFuoriPosto {
                        percorso: percorso.to_path_buf(),
                        riga: numero_di_riga,
                        attesa,
                        trovata,
                    });
                }
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
    ///
    /// **Record `k` of the answer is hour `k % 24 + 1`.** The reader has
    /// checked that the records follow one another, not that the file begins
    /// at hour 1 or ends at hour 24: a file cut out of a year is read. So the
    /// hours are checked here. The check can fail only where the count begins
    /// and where it wraps, and there the file has to end at hour 24 and begin
    /// at hour 1: a Periodo that reaches a day the file has only part of is
    /// `GiornoMonco`.
    pub fn ore_dal(&self, inizio: Data, ore: u32) -> Result<Vec<&ValoriOrari>, MeteoError> {
        let primo = self
            .ore
            .iter()
            .position(|v| v.mese == inizio.mese && v.giorno == inizio.giorno)
            .ok_or_else(|| MeteoError::GiornoAssente {
                percorso: self.percorso.clone(),
                giorno: format!("{}/{}", inizio.giorno, inizio.mese),
            })?;
        let mut attesa = MeseGiornoOra {
            mese: inizio.mese,
            giorno: inizio.giorno,
            ora: 1,
        };
        let mut serviti = Vec::new();
        for passo in 0..ore as usize {
            let indice = (primo + passo) % self.ore.len();
            let valori = &self.ore[indice];
            // The hour alone: across the wrap the day is another one by
            // design, and inside the file the reader has checked it. Back at
            // the first record the hour has to be 1 as well: two half days,
            // one from each end of the file, have their hours in a row and
            // are not a day.
            let a_capo = passo > 0 && indice == 0;
            if valori.ora != attesa.ora || (a_capo && valori.ora != 1) {
                return Err(MeteoError::GiornoMonco {
                    percorso: self.percorso.clone(),
                    attesa,
                    trovata: MeseGiornoOra {
                        mese: valori.mese,
                        giorno: valori.giorno,
                        ora: valori.ora,
                    },
                });
            }
            attesa = ora_dopo(valori.mese, valori.giorno, valori.ora);
            serviti.push(valori);
        }
        Ok(serviti)
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
