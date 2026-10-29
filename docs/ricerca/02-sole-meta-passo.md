# 02 — Sole a metà passo: convenzione di monte contro `sole.rs`

Ticket: maeurong/CLIMESH#31 (parte di #29, blocca #34). Dossier d'ingresso:
`research/catena-radiativa.md:212` (riga «posizione del sole») e `:229` (riga `timestep_dec`).

**Fonti.** Codice di monte al commit pinnato `UMEP-dev/solweig`
`02246ab71a3a8b127d740dde9640449ee9d558ff`, clone in `/tmp/monte` (`git -C /tmp/monte rev-parse HEAD`);
i `percorso:riga` di monte sono relativi a `/tmp/monte`. Codice di CLIMESH dal branch
`research/passo-fuso`, HEAD `fdd9e13`; i `percorso:riga` di CLIMESH sono relativi alla radice del repo.
Una sola pagina web, catturata in `fonti/`.

Tag: `[V]` letto nella fonte · `[M]` misurato con comando · `[INF]` inferenza · `[NON TROVATO]`.

## Premesse verificate

- `research/catena-radiativa.md:212` cita `weather.py:150-151` per il mezzo passo: giusto, le due
  righe sono `half_timestep = timedelta(minutes=self.timestep_minutes / 2.0)` e
  `sun_time = self.datetime - half_timestep` (`pysrc/solweig/models/weather.py:150-151`). `[V]`
- `:229` cita `timeseries.py:409-412` per `timestep_dec`: giusto (`pysrc/solweig/timeseries.py:407-412`).
  Il ticket però chiede «come si combina» con il mezzo passo: **non si combina**, vedi §1. `[V]`
- Il ticket attribuisce a `src/sole.rs` una convenzione. `sole.rs` non ne ha una sull'intervallo:
  calcola il sole all'istante che riceve. La convenzione sta nel chiamante, `src/corsa.rs`. Correzione
  marginale, non cambia la domanda. `[V]`

---

## 1. Convenzione di monte

**Il sole di un record etichettato H si calcola all'istante H − passo/2.** `[V]`

- `Weather.compute_derived` sottrae mezzo passo all'etichetta e passa l'istante risultante all'SPA
  NREL: `pysrc/solweig/models/weather.py:150-151` (sottrazione), `:154-165` (`time_dict` con
  `"UTC": location.utc_offset`, poi `sp.sun_position`). Il commento a `:146-148` dichiara il motivo:
  «Meteorological data timestamps typically represent the end of an interval, so we compute sun position
  at the center of the interval». Docstring concorde a `:128-130`. `[V]`
- Il passo viene da `timestep_minutes`, predefinito **60** (`weather.py:87`). `[V]`
- Lettura EPW: `io_epw.py:373-380` costruisce `datetime(anno, mese, giorno, ora, minuto)` con l'ora EPW
  così com'è (1-24; l'ora 24 diventa le 00:00 del giorno dopo). `Weather.from_epw` crea l'oggetto a
  `weather.py:450-461` **senza** passare `timestep_minutes`, quindi vale 60. Risultato per l'EPW orario:
  record H → sole alle **H − 0:30 ora solare locale**, cioè al centro dell'intervallo EPW (H−1, H]. `[V]`
- Il lettore del formato met UMEP invece deduce il passo dalle prime due righe (`weather.py:560-574`,
  passato a `:607`): per dati a 10 minuti il sole va a −5 minuti. `[V]`

**`timestep_dec` non tocca il sole.** È la durata del passo in frazione di giorno, calcolata dalle prime due
etichette (`timeseries.py:407-412`, 1/24 se c'è un solo passo), messa nello stato termico
(`timeseries.py:530`), passata al Rust nello `StateBundle` (`computation.py:575`) e consumata dal ritardo
termico `ts_wave_delay_batch_pure` (`rust/src/pipeline.rs:1355-1365`, `rust/src/ground.rs:134-212`, dove
accumula `timeadd`). Lo schema di suolo 2026a la converte in secondi (`computation.py:585-591`). Nessuna di
queste righe sposta l'istante del sole. `[V]`

**Monte stesso mescola due istanti nello stesso passo.** Il sole sta a H − 0:30, ma `dectime`, che pilota il
modello di temperatura del suolo (`rust/src/ground.rs:65-79`), è calcolato sull'etichetta non spostata:
`dectime = (weather.datetime.hour + weather.datetime.minute / 60.0) / 24.0` (`computation.py:309`). Anche
`jday` usa l'etichetta (`computation.py:353`). Un porto che uniformasse i due istanti romperebbe la parità
con monte. `[V]` sul codice, `[INF]` sull'effetto.

**Trappola collaterale.** Il campo minuto EPW ammette 1..60 (fonte in §3). Con minuto 60 la riga di monte
`datetime(..., hour, 60)` solleva `ValueError: minute must be in 0..59, not 60` (`[M]`,
`python3 -I -c "from datetime import datetime; datetime(2021,1,1,1,60)"`), e `io_epw.py:408` la scarta in
silenzio; se tutte le righe hanno minuto 60 il lettore fallisce con «EPW file contains no valid data rows»
(`io_epw.py:411-412`). Il minuto del file di Bastia `[NON TROVATO]`: `ITA_Perugia.161810_IGDG.epw` sta in
`materiale università/`, assente da questa macchina.

## 2. Convenzione di CLIMESH

**`sole::posizione` è istantanea.** Prende `ora_locale_h`, «the hour of the local clock», e `fuso_ore`
(`src/sole.rs:28-39`), e la usa così com'è sia nella frazione d'anno (`:42`) sia nell'angolo orario
(`:54-56`). Nessun mezzo passo. `[V]`

**Il chiamante la interroga alle ore intere dall'inizio del Periodo.** `marcia` cicla `for ora in
0..periodo.ore` e chiama `sole::posizione(periodo.inizio, f64::from(ora), luogo.fuso_ore, ...)`
(`src/corsa.rs:424-431`): passo k → sole alle **k:00** dell'orologio locale. Le ore oltre 24 fanno avanzare
il giorno (`corsa.rs:376-378`). Il fuso viene dall'header EPW (`corsa.rs:306`; `src/meteo.rs:59-60`: ora
solare tutto l'anno). `[V]`

**L'EPW non è ancora agganciato al sole.** `src/meteo.rs:31-33` documenta il record h come media dell'ora
che finisce in h, da h−1 a h, e lascia aperto «which instant inside that hour the sun is computed at».
`Epw::ore_dal(inizio, ore)` (`meteo.rs:290-301`) restituisce i record dal primo del giorno, cioè dall'ora
EPW 1: l'indice k corrisponde all'ora EPW k+1, intervallo (k, k+1]. Nessun file in `src/` chiama `ore_dal`
oggi (`grep -rn ore_dal src tests` lo trova solo in `tests/meteo.rs`). `[V]`

**Accoppiamento naturale e suo errore.** Il giorno in cui la Corsa zippa `ore_dal(...)[k]` con il passo k di
`marcia`, il record EPW k+1 (intervallo k..k+1) avrà il sole alle k:00, cioè **all'inizio** dell'intervallo:
30 minuti prima di monte, 60 minuti prima dell'etichetta EPW. `[INF]` dalla lettura delle due righe.

## 3. Convenzione EPW

**L'ora EPW H copre l'intervallo che finisce in H; le radiazioni sono integrate sui minuti che precedono
l'istante indicato.** `[V]`

- **URL** https://bigladdersoftware.com/epx/docs/24-2/auxiliary-programs/energyplus-weather-file-epw-data-dictionary.html · [V]
  (cattura `fonti/energyplus-24-2-epw-data-dictionary.md`, sidecar accanto)
  **perche' conta qui** è il dizionario dati del formato che CLIMESH legge (`src/meteo.rs`) e che monte
  legge (`io_epw.py`); decide a quale intervallo appartiene ogni record.
  **cosa se ne prende** riga 369 della cattura: «Hour 1 is 00:01 to 01:00», ore 1-24; riga 373: minuto
  «(1..60)»; righe 451, 455, 459: globale, diretta normale e diffusa orizzontale «received ... during the
  number of minutes preceding the time indicated». Quindi il centro dell'intervallo del record H è
  H − 0:30, che è l'istante di monte e non quello di CLIMESH.

Staleness: la cattura è della documentazione EnergyPlus 24.2, l'ultima versione pubblicata su
bigladdersoftware (le URL `25-1`, `25-2` rispondono 404, `[M]` con `curl -s -o /dev/null -w "%{http_code}"`).
EnergyPlus 25.x esiste; che il testo del campo sia invariato è `[INF]`, sostenuto dal fatto che la stessa
frase compare nelle versioni 8.x della stessa documentazione.

## 4. Scarto misurato a Bastia Umbra

**Cosa è stato eseguito.** `[M]`

- Rust: crate di prova nello scratchpad con dipendenza di percorso su CLIMESH (`fdd9e13`), che chiama
  `climesh::sole::posizione(Data{2021, m, g}, h, 1.0, 43.07, 12.56)` per h = 4,0..21,0 a mezz'ora, il 21/6 e
  il 21/12/2021. Comando:
  `cargo run -q --release --manifest-path <scratchpad>/solecli/Cargo.toml > <scratchpad>/climesh_sole.csv`
  (rustc 1.98.0). Coordinate e fuso da `tests/motore.rs:17-24` e `casi/bastia/progetto/progetto.toml`
  (`origine = [12.56, 43.07]`); anno dai Periodi di `casi/bastia/progetto/periodi/`.
- Python: `python3 -I <scratchpad>/confronto.py <scratchpad>/climesh_sole.csv` (Python 3.14.7, numpy 2.5.1),
  che carica `pysrc/solweig/physics/sun_position.py` di monte per percorso e lo chiama con lo stesso
  `time_dict` di `weather.py:154-162`, altitudine 0 (`models/location.py:65`), per il record EPW k+1
  all'istante `etichetta − 30 min`.
- Scratchpad: `/private/tmp/claude-501/-Users-mario-GitHub/89c63284-d20f-45d5-ace4-5f46359bb090/scratchpad`.

Per ogni passo k (sole di CLIMESH alle k:00, record EPW k+1, sole di monte alle k:30) tre differenze, in
gradi, CLIMESH meno monte. **TOT** è lo scarto che vedrebbe il porto. **TEMPO** è
`posizione(k) − posizione(k+0,5)`, solo l'istante. **ALGO** è `posizione(k+0,5) − SPA(k+0,5)`, solo
l'algoritmo (Spencer contro SPA con rifrazione).

**21 giugno 2021**

| k | CLIMESH alt / az | monte alt / az | TOT Δalt / Δaz | TEMPO Δalt / Δaz | ALGO Δalt / Δaz |
|---|---|---|---|---|---|
| 6 | 13,86 / 70,70 | 19,04 / 75,28 | −5,19 / −4,58 | −5,24 / −4,65 | 0,05 / 0,07 |
| 8 | 35,34 / 89,72 | 40,73 / 94,94 | −5,39 / −5,22 | −5,47 / −5,30 | 0,08 / 0,08 |
| 10 | 56,65 / 115,43 | 61,30 / 124,99 | −4,65 / −9,56 | −4,73 / −9,73 | 0,08 / 0,17 |
| 11 | 65,50 / 137,53 | 68,57 / 153,05 | −3,07 / −15,52 | −3,12 / −15,80 | 0,05 / 0,29 |
| 12 | 70,25 / 172,46 | 70,01 / 192,42 | 0,24 / −19,96 | 0,25 / −20,31 | −0,01 / 0,35 |
| 13 | 67,93 / 211,14 | 64,57 / 225,76 | 3,37 / −14,62 | 3,43 / −14,86 | −0,06 / 0,25 |
| 14 | 60,19 / 237,60 | 55,43 / 246,62 | 4,76 / −9,02 | 4,85 / −9,18 | −0,09 / 0,16 |
| 16 | 39,39 / 266,41 | 34,02 / 271,50 | 5,37 / −5,09 | 5,48 / −5,20 | −0,11 / 0,10 |
| 18 | 17,72 / 285,86 | 12,66 / 290,43 | 5,06 / −4,57 | 5,20 / −4,66 | −0,15 / 0,09 |
| 19 | 7,47 / 295,27 | 2,93 / 300,07 | 4,54 / −4,80 | 4,85 / −4,90 | −0,31 / 0,10 |
| 20 | −1,98 / 305,28 | −6,31 / 310,54 | 4,33 / −5,26 | 4,32 / −5,37 | 0,01 / 0,11 |

**21 dicembre 2021**

| k | CLIMESH alt / az | monte alt / az | TOT Δalt / Δaz | TEMPO Δalt / Δaz | ALGO Δalt / Δaz |
|---|---|---|---|---|---|
| 8 | 2,55 / 125,89 | 6,90 / 131,26 | −4,35 / −5,36 | −4,28 / −5,40 | −0,07 / 0,04 |
| 10 | 17,32 / 149,48 | 19,83 / 156,18 | −2,50 / −6,70 | −2,50 / −6,76 | −0,00 / 0,06 |
| 12 | 23,49 / 178,10 | 23,33 / 185,52 | 0,15 / −7,42 | 0,18 / −7,49 | −0,02 / 0,08 |
| 14 | 18,66 / 207,13 | 15,95 / 213,65 | 2,70 / −6,52 | 2,78 / −6,59 | −0,07 / 0,07 |
| 15 | 12,60 / 219,96 | 8,99 / 225,78 | 3,60 / −5,82 | 3,73 / −5,88 | −0,12 / 0,07 |
| 16 | 4,76 / 231,40 | 0,79 / 236,61 | 3,97 / −5,20 | 4,43 / −5,27 | −0,47 / 0,06 |

Tabelle ridotte; l'output completo, ore 6-20 a passo 1 h, si rigenera con i due comandi sopra.

**Lettura.** `[M]`

- Con il sole sopra l'orizzonte in entrambe le convenzioni, lo scarto massimo è **5,39° in altezza e
  19,96° in azimut il 21 giugno** (azimut massimo a mezzogiorno, dove l'azimut corre più veloce), e
  **4,35° in altezza e 7,42° in azimut il 21 dicembre**.
- Lo scarto è quasi tutto **tempo**: la colonna TEMPO ricalca TOT entro 0,5°. L'algoritmo pesa al più
  0,35° in azimut e 0,47° in altezza, il massimo vicino all'orizzonte, dove la rifrazione dell'SPA
  (`physics/sun_position.py:1002-1040`, sempre applicata) conta e Spencer non ce l'ha. È coerente con il
  tetto dichiarato in `src/sole.rs:10-15` (0,10° in altezza e 0,29° in azimut contro Meeus).
- Il segno è sistematico: CLIMESH ha il sole mezz'ora indietro, più basso al mattino, più alto il
  pomeriggio, con l'azimut sempre minore, tra 4,6° e 20°.
- In queste ore e in questi due giorni nessun passo cambia da giorno a notte fra le due convenzioni. Il più
  vicino è il 21/12, k = 16: 4,76° contro 0,79°. `[M]`

## Cosa resta incerto

- **Accoppiamento futuro.** Lo scarto di §4 presuppone che la Corsa abbini `ore_dal(...)[k]` al passo k di
  `marcia`. Se invece abbinasse il passo h all'etichetta EPW h, il sole starebbe alla fine dell'intervallo:
  scarto della stessa ampiezza, segno opposto. Non misurato. `[INF]`
- **Minuto del file di Bastia** e quindi se monte lo legge del tutto: `[NON TROVATO]`, file assente.
- **Effetto su Tmrt.** Mezz'ora di sole sposta ombre e componente diretta; quanto valga in Tmrt sul caso di
  riferimento non è misurato qui. `[NON TROVATO]`

## Raccomandazione (non decisione)

Per la parità con monte il porto dovrebbe chiamare `sole::posizione` al centro dell'intervallo del record,
cioè `ora_EPW − 0,5` (con l'indice di `ore_dal`, `k + 0,5`), e lasciare `dectime` e `jday`
sull'etichetta come fa `computation.py:309,353`. È anche la convenzione che il dizionario EPW rende
naturale (§3). Da decidere nel thread, eventualmente con un ADR: tocca anche le ombre della Giornale
già calcolate, che oggi stanno alle ore intere.
