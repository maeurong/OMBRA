# Piano B — Lo strato portato

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Le parti dello strato Python di `solweig` che preparano gli ingressi del passo esistono in Rust, ciascuna verificata contro un record generato una volta dalla wheel di monte al pin.

**Architecture:** Ogni parte è una funzione pura con la riga di monte citata. Il sole SPA sostituisce l'algoritmo NOAA in `src/sole.rs`; gli scalari dell'Ora stanno in `src/scalari.rs`; il suolo in `src/suolo.rs`, come le specie in `src/specie.rs`; i Muri nel Motore, accanto allo Sky view factor; le due Trasmissività e `svfalfa` in `src/derivazione.rs`; lo Stato termico in `src/stato_termico.rs`. Un generatore Python fuori dalla CI scrive i record in `tests/record/`; un test confronta il pin di ogni record con quello di `PROVENIENZA.toml`.

**Tech Stack:** Rust 2021, `ndarray` 0.16. Python 3.12 (la wheel chiede `>=3.11,<3.14`) con `solweig==0.1.0b101` e le sue dipendenze a versione fissata, solo nel generatore, lanciato da Mario.

**Spec:** [`docs/superpowers/specs/2026-10-29-catena-radiativa.md`](../specs/2026-10-29-catena-radiativa.md), sezioni «Lo strato Python portato», «Il Motore, in OMBRA» (Muri e Trasmissività), «Decisioni sui test § 3». **Vincoli globali e ordine dei piani:** [indice](2026-11-22-catena-radiativa.md#global-constraints). **Chi esegue, in che ordine, con quali revisori:** [annotazione di dispatch](2026-11-22-catena-radiativa-b-strato-dispatch.md); la sua sezione «Difetti» è integrata in questa versione.

**Dipende dal piano A.** Solo i task 1 e 2 partono prima che A sia unito: il task 3 consuma `motore::{WeatherScalars, ProprietaDelSuolo}` (A, task 4) e l'assenza di `psi` (A, task 3); il task 4 `compute_wall_aspect_pure` (A, task 2) e `motore::Muri`; il task 5 `motore::PassataDelCielo`; il task 6 `motore::StatoTermico`.

**Fonti da aprire:** `docs/ricerca/04-strato-python-monte.md` (ticket [#54](https://github.com/maeurong/OMBRA/issues/54), la tabella «parte per parte»), `docs/ricerca/02-sole-meta-passo.md:27`, `docs/ricerca/04-stato-termico.md:47`, `:102`, `:257`, `docs/ricerca/01-passo-fuso.md:155`, `docs/ricerca/03-chiome-riunite.md:104`. Lato monte, al pin: `pysrc/solweig/physics/sun_position.py:8`, `clearnessindex_2013b.py:11`, `diffusefraction.py:6`, `wallalgorithms.py:10`, `models/weather.py:165-251` e `:491-495`, `computation.py:337-385` e `:705-727`, `components/svf_resolution.py:96-100`, `models/precomputed.py:61-70`, `models/surface.py:1163` e `:1959-1966`, `models/location.py:65` e `:221`, `io_epw.py:370-378`, `timeseries.py:138-143` e `:463-469`.

**Assunzioni dichiarate dal piano**, senza scelta di prodotto in gioco:
- l'anno del sole è quello di `Periodo.inizio`, come la scrittura delle Ore (#63); il generatore passa alla wheel lo stesso anno, così la parità regge;
- la pressione dell'EPW serve al clearness index (`weather.py:243-251`): un valore mancante ferma la Corsa come gli altri valori che servono al passo (spec, riga 269);
- l'altitudine del sole è la quota dell'EPW (`Epw::quota_m`, `src/meteo.rs:109`), come fa monte (`location.py:221`).

## Review Focus

1. **Il Muro e il suo conteggio.** `findwalls` mette il Muro sulla cella bassa, fuori dall'impronta (`wallalgorithms.py:44-50`), contro la sua docstring (`:13-15`). Contare «celle-Muro fuori da un Edificio» darebbe sempre tutte le celle-Muro: il task 4 conta le celle-Muro senza un Edificio fra i quattro vicini.
2. **L'Ora 24 dell'EPW** è la mezzanotte del giorno dopo (`io_epw.py:376-378`): `jday`, `dectime`, `snup` e `altmax` vengono dal giorno dopo, e il 31/12 ora 24 dà `jday` 1. Test nel task 3.
3. **Il clearness index infinito**, con il sole sotto 0,01° o `I0 == 0` (`clearnessindex_2013b.py:43-45`, `:104-111`), portato nella notte (`timeseries.py:139-143`). Il record e il confronto devono reggere `inf` (task 1 e task 3).
4. **L'azimut intorno a 0/360**, a Tromsø a mezzanotte: si confronta modulo 360, non in tolleranza relativa (task 2).
5. **Una classe di Superficie tradotta nella classe di monte sbagliata.** Monte riceve già la classe sbagliata e il confronto non se ne accorge: il task 3 ha un test di corrispondenza suo.

---

### Task 1: Il generatore dei record e il test del pin

Due parti in parallelo, poi un solo passaggio di Mario.

**Files:**
- Create (1a, Python): `strumenti/record/genera.py`, `strumenti/record/requisiti.txt`, `strumenti/record/LEGGIMI.md`
- Create (1b, Rust): `tests/record.rs`, `tests/comune/mod.rs`
- Create: `tests/record/` (i file generati)

**Interfaces:**
- Produces, in `tests/comune/mod.rs` (usato con `mod comune;`):
  - `pub fn leggi_casi(nome: &str) -> toml::Table` per le funzioni scalari: `tests/record/<nome>.toml` con `[intestazione]` (`pin`, `wheel`, `comando`) e un array `[[caso]]`; `inf` e `-inf` sono valori TOML validi e si confrontano con `==`;
  - `pub fn leggi_campo(nome: &str) -> (toml::Table, ndarray::ArrayD<f32>)` per i raster: `tests/record/<nome>.toml` con `pin`, `wheel`, `comando`, `forma` (due o tre dimensioni), `sha256`, e accanto `tests/record/<nome>.f32`, `f32` little-endian in ordine di riga, riga 0 a nord.
  - Lo `sha256` si controlla con `ombra::giornale::somma_di_controllo` (`src/giornale.rs:421`): nessun crate nuovo.

- [ ] **Step 1: Scrivi il test del pin (1b), che fallisce**

`ogni_record_porta_il_pin_della_copia_vendorata`: per ogni `tests/record/*.toml`, `intestazione.pin == motore::versione().commit`, e per ogni `.f32` lo `sha256` del file è quello dichiarato. Con la cartella vuota il test fallisce con «nessun record».

- [ ] **Step 2: Verifica che fallisca**

Run: `cargo test -p ombra --test record` → FAIL, «nessun record».

- [ ] **Step 3: Scrivi il generatore completo (1a)**

`genera.py <parte>` e `genera.py tutto`, con le parti di ogni task di questo piano (`sole`, `scalari`, `suolo`, `muri`, `svfalfa`, `stato`) e del task 7 del piano C (`meteo-48-ore`, `bastia`), quest'ultima scritta dal piano C. Ogni parte scrive il `comando` esatto. `requisiti.txt` fissa la wheel e le sue dipendenze (`numpy` compreso) a versione esatta. `LEGGIMI.md` dice come creare l'ambiente con Python 3.12 (`python3.12 -m venv` o `uv venv -p 3.12`) e che il generatore non entra in CI. I casi di ogni parte sono quelli scritti nei task 2-6.

- [ ] **Step 4: Mario lancia `genera.py tutto`** (HITL)

Un solo passaggio per le sei parti di questo piano. Il test del pin passa.

- [ ] **Step 5: Commit**

```bash
git add strumenti/record tests/record.rs tests/comune/mod.rs tests/record
git commit -m "test(record): generate upstream records and pin them to the vendored copy"
```

---

### Task 2: Il sole è l'SPA di monte

**Files:**
- Modify: `src/sole.rs` (tutto `posizione` `:33`, il commento di modulo `:1-15`)
- Modify: `src/corsa.rs` (`Luogo` `:337-343` prende la quota; la chiamata in `marcia`)
- Modify: `Cargo.toml` (`version = "0.4.0"`)
- Test: `tests/sole.rs` (nuovo), `tests/motore.rs` (sei chiamate a `posizione`: `:49`, `:89`, `:110`, `:118`, `:130`, `:138`), `tests/corsa.rs`

**Interfaces:**
- Produces: `pub fn posizione(data: Data, ora_locale_h: f64, fuso_ore: f64, latitudine_gradi: f64, longitudine_gradi: f64, altitudine_m: f64) -> Option<PosizioneSolare>`. `PosizioneSolare` resta com'è. Un'`ora_locale_h` oltre 24 si riporta al giorno dopo prima del giorno giuliano.
- Produces: `pub fn altezza_massima_del_giorno(data: Data, fuso_ore: f64, latitudine_gradi: f64, longitudine_gradi: f64, altitudine_m: f64) -> f64`, l'`altmax` di `models/weather.py:197-232`: 96 passi da 15 minuti dalla mezzanotte, il massimo, mai sotto zero.

- [ ] **Step 1: I casi della parte `sole`**

Bastia (43,07 N 12,55 E, fuso +1, quota dell'EPW) il 21 giugno, il 21 dicembre e il 20 marzo 2026, ogni mezz'ora; lo stesso punto a 1830 m il 21 giugno a mezzogiorno; Tromsø (69,65 N 18,96 E, fuso +1) il 21 giugno a mezzanotte; Sydney (−33,87, 151,21, fuso +10) il 21 giugno a mezzogiorno. Uscite: zenit, azimut, `altmax` del giorno.

- [ ] **Step 2: Scrivi i test che falliscono**

- `il_sole_coincide_con_monte`: altezza `90 − zenit` entro `1e-9` relativo; azimut con `|(a − b + 180).rem_euclid(360) − 180| < 1e-9`; `altezza_massima_del_giorno` entro `1e-9`. Se `1e-9` non regge per l'ordine delle operazioni sul giorno giuliano (un ulp del giorno giuliano vale circa 1,7e-7°, `sun_position.py:195-224`), la tolleranza si allarga alla misura, scritta accanto;
- `la_quota_sposta_il_sole`: a 1830 m e a 0 m l'altezza differisce, e ciascuna coincide con il suo record;
- `il_sole_di_riferimento_nrel`: il caso d'esempio del rapporto NREL dell'SPA (Reda e Andreas, NREL/TP-560-34302), con i valori presi dal rapporto e citati: è l'unico controllo indipendente da monte che resta.

- [ ] **Step 3: Verifica che falliscano**

Run: `cargo test -p ombra --test sole` → FAIL: il NOAA dista fino a 0,47° (`docs/ricerca/02-sole-meta-passo.md` §4).

- [ ] **Step 4: Porta `sun_position.py`**

Funzione per funzione, da `julian_calculation` (`:164`) a `sun_topocentric_zenith_angle_calculate` (`:1002`), in `f64`, ogni tabella di coefficienti con la riga citata e nello stesso ordine di operazioni. Il commento di modulo di `src/sole.rs` si riscrive: il sole viene da monte, il «Known ceiling» del NOAA (`:10-15`) cade. La verifica dell'ombra del Giornale (`src/corsa.rs:393-410`) resta quello che era, il confronto fra l'ombra del Motore e gli angoli che il Motore riceve; la posizione non è più indipendente da monte, e il commento lo dice.

- [ ] **Step 5: Aggiorna i test e la versione**

I test di `tests/motore.rs` e `tests/corsa.rs` che chiamano `posizione` o fissano angoli prendono l'argomento nuovo e i valori nuovi; un test che cambia valore cita nel commento la misura che lo giustifica. `Cargo.toml` passa a `0.4.0`: il sole cambia ombre e Giornale (decisione di Mario, 2026-11-23).

- [ ] **Step 6: Verifica e commit**

Run: `cargo test -p ombra --test sole --test motore --test corsa` → PASS. Poi il cancello di CI.

```bash
git add src/sole.rs src/corsa.rs Cargo.toml Cargo.lock tests/sole.rs tests/motore.rs tests/corsa.rs
git commit -m "feat(sole)!: port upstream SPA as the single sun of the Ora"
```

---

### Task 3: Gli scalari dell'Ora e il suolo

**Files:**
- Create: `src/scalari.rs`, `src/suolo.rs`
- Modify: `src/meteo.rs` (`ValoriOrari` `:29` prende la pressione), `src/lib.rs`
- Test: `tests/scalari.rs`, `tests/suolo.rs`, `tests/meteo.rs`

**Interfaces:**
- Consumes: `motore::{WeatherScalars, ProprietaDelSuolo}` (piano A, task 4), `meteo::{Epw, ValoriOrari}`, `sole::{posizione, altezza_massima_del_giorno}`.
- Produces, in `src/meteo.rs`: `pub pressione_hpa: Option<f64>` in `ValoriOrari`, letta dal campo della pressione atmosferica dell'EPW (in Pa nel file, convertita come fa `weather.py:491-493`).
- Produces, in `src/scalari.rs`:

```rust
/// The clearness index carried into the night, and where it came from.
pub struct ClearnessPortato { pub valore: f64, pub da: Option<MeseGiornoOra> }  // da: None is the 1.0 fallback

pub fn clearness_della_prima_notte(epw: &Epw, inizio: Data, luogo: &Luogo) -> ClearnessPortato;

/// The WeatherScalars of one Ora, and the clearness index the next Ora carries.
pub fn scalari_dell_ora(
    record: &ValoriOrari, inizio: Data, ora: u32, luogo: &Luogo,
    sole: PosizioneSolare, altmax_gradi: f64, portato: &ClearnessPortato,
) -> (WeatherScalars, ClearnessPortato);
```

`Luogo` (`src/corsa.rs:337-343`, con la quota aggiunta al task 2) si sposta in `src/sole.rs` e diventa `pub`: lo usano la Corsa e gli scalari. `patch_option` vale 2, come `PATCH_DEL_CIELO` (`src/motore.rs`).

- Produces, in `src/suolo.rs`:

```rust
pub struct ClasseDiMonte {
    pub codice: u8, pub albedo: f32, pub emissivita: f32,
    pub tgk: f32, pub tstart: f32, pub tmaxlst: f32,
    pub riga: &'static str,   // the upstream line the five values come from
}
pub const SOTTO_EDIFICIO: ClasseDiMonte;   // upstream class 2
pub fn classe_di_monte(classe: u8) -> ClasseDiMonte;   // OMBRA's CLASSE_NESSUNA..=CLASSE_TERRENO_NUDO
pub struct ConteggioDelSuolo { pub per_classe: BTreeMap<String, usize> }   // String keys: TOML tables need them
pub fn proprieta_del_suolo(classi: &Array2<u8>, costruite: &Raster) -> (ProprietaDelSuolo, ConteggioDelSuolo);
```

- [ ] **Step 1: I casi delle parti `scalari` e `suolo`**

`scalari`: 48 Ore estive a Bastia con una sera coperta (clearness index sotto 0,95 portato oltre la mezzanotte), un'Ora d'alba con il sole sotto l'orizzonte al centro e la globale sopra zero, e il 31/12 ora 24. Il record parte da `ClearnessPortato { valore: 1.0, da: None }`, come monte (`timeseries.py:138`). Uscite: ogni campo di `WeatherScalars` tranne `psi`. `suolo`: i cinque valori delle classi 0, 1, 2, 5, 6, 7.

- [ ] **Step 2: Scrivi i test che falliscono**

In `tests/scalari.rs`:
- `gli_scalari_coincidono_con_monte`: ogni campo `f32` uguale al record convertito da `f64`, o entro la tolleranza misurata e scritta accanto; `jday` e `is_daytime` uguali;
- `l_alba_tiene_la_radiazione_misurata`: nell'Ora d'alba `direct_rad` e `diffuse_rad` sono la DNI e la DHI del file, come monte con radiazione misurata (`weather.py:236-239`, `:494-495`; decisione di Mario, 2026-11-23);
- `dopo_mezzanotte_resta_l_ultimo_clearness_del_giorno`: nell'Ora 0–1 dopo la sera coperta `clearness_index` è quello dell'ultima Ora diurna, non 1,0;
- `l_ora_ventiquattro_e_il_giorno_dopo`: per il record del 31/12 ora 24, `jday == 1` e `dectime` e `altmax` sono quelli del 1° gennaio;
- `il_clearness_infinito_passa_nella_notte`: con il sole sotto 0,01° all'ultima Ora diurna, il valore portato è quello che dà monte, `inf` compreso;
- `senza_una_sera_il_clearness_e_uno`: un `Epw` di 24 Ore che comincia a mezzanotte → `valore == 1.0`, `da == None`;
- `la_sera_si_cerca_sul_cerchio_solo_per_un_anno_intero`: un anno intero dal 1/1 ora 1 al 31/12 ora 24 che comincia di notte trova la sera del 31 dicembre; lo stesso file troncato al 30 dicembre no.

In `tests/meteo.rs`: `la_pressione_si_legge_dall_epw`, con il valore del campo e `None` per il valore mancante dichiarato del formato.

In `tests/suolo.rs`:
- `le_classi_di_ombra_vanno_nelle_classi_di_monte`: `CLASSE_NESSUNA → 0`, `CLASSE_PAVIMENTATO → 1`, `CLASSE_ERBA → 5`, `CLASSE_TERRENO_NUDO → 6`, `CLASSE_ACQUA → 7`, `SOTTO_EDIFICIO.codice == 2`;
- `i_valori_del_suolo_coincidono_con_monte`: i cinque valori di ogni classe uguali al record, esatti;
- `sotto_un_edificio_vale_la_classe_due`: una cella con `costruite > 0` e classe `CLASSE_ERBA` riceve i valori di `SOTTO_EDIFICIO` e si conta sotto `"2"`.

- [ ] **Step 3: Verifica che falliscano**

Run: `cargo test -p ombra --test scalari --test suolo --test meteo` → errori di compilazione.

- [ ] **Step 4: Implementa**

`scalari_dell_ora` porta il ramo con radiazione misurata di `weather.py` (sole al centro, clearness index con la pressione, `:165-251`, `:491-495`) e la parte di `computation.py:337-385` (`dectime`, `jday`, `snup` e `daylen`, `rad_g0` con quota 0 e pressione −999 poi `diffusefraction` con `Kt = 1`, `zen_deg`, `is_daytime`). Trappola dell'umidità: frazione a `computation.py:355`, per cento a `:361` (`diffusefraction.py:29`). `dectime` e `jday` vengono dall'etichetta del record, come a monte. Il valore `None` non arriva qui: lo ferma la Corsa (piano C, task 3). L'acqua resta alla classe 7 senza la sua temperatura, e il commento cita [UMEP-dev/umep-solweig#4](https://github.com/UMEP-dev/umep-solweig/issues/4).

- [ ] **Step 5: Verifica e commit**

Run: `cargo test -p ombra --test scalari --test suolo --test meteo --test record` → PASS. Poi il cancello di CI.

```bash
git add src/scalari.rs src/suolo.rs src/meteo.rs src/sole.rs src/corsa.rs src/lib.rs tests/scalari.rs tests/suolo.rs tests/meteo.rs
git commit -m "feat(scalari): port the hourly weather scalars and ground properties"
```

---

### Task 4: I Muri

**Files:**
- Modify: `src/motore.rs` (una funzione accanto a `sky_view_factor`)
- Test: `tests/motore.rs`

**Interfaces:**
- Consumes: `motore::Muri` (piano A, task 4), `compute_wall_aspect_pure` (piano A, task 2), chiamata con avanzamento e annullamento a `None`.
- Produces:

```rust
pub struct ConteggioDeiMuri {
    pub lato_del_filtro_celle: usize,
    pub celle_muro: usize,
    pub altezza_massima_m: f64,
    /// Wall cells with no Edificio among their four neighbours: a terrain step.
    pub celle_muro_senza_edificio: usize,
    pub edifici_sul_bordo: usize,
}
pub fn muri(modello_di_superficie: &Raster, costruite: &Raster, passo_m: f64, soglia_m: f64) -> (Muri, ConteggioDeiMuri);
```

- [ ] **Step 1: I casi della parte `muri`**

Sette DSM piccoli, ciascuno con il raster di `findwalls` e l'orientamento di monte: un gradino di 3 m fra due metà della Griglia; un gradino alto esattamente quanto la soglia; un Edificio largo una cella; un Edificio sul bordo della Griglia; un gradino del terreno senza Edificio; un Edificio la cui faccia sta nella fascia di bordo del filtro; uno Scenario senza Edifici.

- [ ] **Step 2: Scrivi per primo il test del gradino**

`il_muro_cade_dal_lato_del_record`: sul gradino, le celle con `altezza > 0` sono quelle del record, esatte, e stanno sulla cella bassa (`wallalgorithms.py:44-50`).

- [ ] **Step 3: Scrivi gli altri test**

- `i_muri_coincidono_con_monte`: per i sette casi, `altezza` e `orientamento_gradi` uguali esatti, compresa la fascia di bordo, dove l'orientamento viene dal gradiente del DSM (`wall_aspect.rs:300-310`);
- `un_gradino_alto_quanto_la_soglia_e_un_muro`: il caso alla soglia dà `celle_muro > 0`;
- `senza_edifici_i_muri_sono_zero`: `celle_muro == 0`, `altezza` tutta a zero;
- `il_filtro_ha_un_lato_dispari_di_almeno_tre`: `lato_del_filtro_celle` vale 9, 5, 3, 3 a 1, 2, 4, 10 m (`wall_aspect.rs:159-165`);
- `una_soglia_piu_alta_dell_edificio_non_fa_muri`: soglia 50 m su un Edificio di 6 m → `celle_muro == 0`;
- `il_gradino_del_terreno_si_conta`: nel caso senza Edificio `celle_muro_senza_edificio == celle_muro`; nel caso dell'Edificio largo una cella vale 0;
- `l_edificio_sul_bordo_si_conta`: `edifici_sul_bordo == 1` nel caso sul bordo, 0 negli altri.

- [ ] **Step 4: Verifica che falliscano, implementa, verifica che passino**

Run: `cargo test -p ombra --test motore` → FAIL, poi PASS. `findwalls` si porta da `wallalgorithms.py:10` in `f32`, come a monte, con la riga citata. `edifici_sul_bordo` conta gli Edifici contigui (`costruite > 0`) che toccano la prima o l'ultima riga o colonna.

- [ ] **Step 5: Commit**

```bash
git add src/motore.rs tests/motore.rs
git commit -m "feat(motore): derive walls from the surface model"
```

---

### Task 5: Le due Trasmissività e `svfalfa`

**Files:**
- Modify: `src/motore.rs` (`ombre` `:59`), `src/derivazione.rs`
- Test: `tests/derivazione.rs`, `tests/motore.rs`

**Interfaces:**
- Consumes: `motore::PassataDelCielo` (piano A, task 4).
- Produces:

```rust
// src/motore.rs: `ombre` keeps its signature and result; the per-layer shading
// is exposed, and the attenuation formula (src/motore.rs:338-340) stays in one place.
pub struct OmbreDellOra { pub illuminata: Raster, pub ombra_per_strato: Vec<Raster> }  // 1 = the layer shades the cell
pub fn ombre_per_strato(modello_di_superficie: &Raster, passo_m: f64, sole: PosizioneSolare, strati: &[StratoDiChioma]) -> OmbreDellOra;

// src/derivazione.rs
pub fn trasmissivita_lungo_il_raggio(strati: &[StratoDiChioma], ombra_per_strato: &[Raster]) -> Raster;
/// 1 − detrazione / volta_coperta, clamped to [0, 1]; 1 where volta_coperta is 0 (ADR 0006:34-36).
pub fn trasmissivita_verso_il_cielo(cielo: &PassataDelCielo) -> Raster;
pub fn svfalfa(cielo: &PassataDelCielo) -> Raster;
```

- [ ] **Step 1: I casi della parte `svfalfa`**

`svf` e `svf_veg` di un caso con un Albero; uscita `svfalfa` (`components/svf_resolution.py:96-100`), calcolata a monte in `f32` (`models/precomputed.py:61-70`).

- [ ] **Step 2: Scrivi i test che falliscono**

- `senza_chiome_le_trasmissivita_valgono_uno`: nessuno Strato → i due raster valgono 1,0 in ogni cella;
- `il_raggio_attraverso_due_strati_e_il_prodotto`: una cella ombreggiata da entrambi gli Strati d'inverno vale `0.03 * 0.5`, una dal solo spoglio `0.5`, una da nessuno `1.0`;
- `con_uno_strato_il_cielo_e_la_sua_trasmissivita`: uno Strato sempreverde → ogni cella con `volta_coperta > 0` vale 0,03 entro la misura dello step 3;
- `il_cielo_con_due_strati_sta_fra_le_due_trasmissivita`: d'inverno, ogni cella coperta sta fra `0.03 * 0.5` e `0.5`;
- `svfalfa_coincide_con_monte`: uguale al record entro 2 ulp di `f32`, o entro la misura scritta accanto;
- `ombre_per_strato_ricompone_l_illuminata`: `illuminata` è il prodotto, cella per cella, di `1 − ombra_per_strato[i] · (1 − trasmissività_i)` sugli Strati, moltiplicato per l'ombra degli edifici.

- [ ] **Step 3: Verifica, implementa, misura, commit**

Run: `cargo test -p ombra --test derivazione --test motore` → FAIL, poi PASS. Scrivi nel commento di `trasmissivita_verso_il_cielo` lo scarto massimo fra `svfbuveg` ricostruito con il raster nella formula di monte in `f32` e il `con_le_chiome` della 0.3.0 sul caso di riferimento (spec, «Note»).

```bash
git add src/motore.rs src/derivazione.rs tests/derivazione.rs tests/motore.rs
git commit -m "feat(derivazione): derive beam and sky transmissivity rasters"
```

---

### Task 6: Lo Stato termico

**Files:**
- Create: `src/stato_termico.rs`
- Modify: `src/lib.rs`
- Test: `tests/stato_termico.rs`

**Interfaces:**
- Consumes: `motore::StatoTermico` (piano A, task 4).
- Produces:

```rust
/// Upstream's initial state for an hourly Periodo: timestep_dec is 1/24 (timeseries.py:463-469).
pub fn iniziale(forma: (usize, usize)) -> StatoTermico;
/// After the step (computation.py:705-727): by day firstdaytime goes to 0; at night,
/// sun at the centre of the Ora at or below 0, firstdaytime back to 1 and timeadd to 0.
pub fn dopo_il_passo(stato: StatoTermico, altezza_al_centro_gradi: f64) -> StatoTermico;
```

- [ ] **Step 1: Il caso della parte `stato`**

Lo stato iniziale di monte per un Periodo orario. Lo stato dopo il passo non ha record: servirebbe far girare il passo nella wheel, e la regola è di tre righe. La verificano i test qui sotto e, nel piano C, le Corse registrate.

- [ ] **Step 2: Scrivi i test che falliscono**

- `lo_stato_iniziale_coincide_con_monte`: tutti i campi uguali al record (`docs/ricerca/04-strato-python-monte.md`: `solweig` dà `timestep_dec` 1/24, `umep-solweig` 0; si porta `solweig`);
- `di_notte_firstdaytime_torna_a_uno`: `dopo_il_passo(s, -5.0)` → `firstdaytime == 1`, `timeadd == 0.0`, i raster invariati;
- `di_giorno_firstdaytime_va_a_zero`: `dopo_il_passo(s, 30.0)` → `firstdaytime == 0`, `timeadd` e i raster invariati;
- `sole_di_mezzanotte_non_azzera_mai`: 24 Ore con il sole sempre sopra zero → `firstdaytime` resta 0 dopo la prima Ora (`docs/ricerca/04-stato-termico.md` §7).

- [ ] **Step 3: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test stato_termico` → FAIL, poi PASS. Porta `computation.py:705-727` riga per riga, con le righe citate.

```bash
git add src/stato_termico.rs src/lib.rs tests/stato_termico.rs
git commit -m "feat(stato_termico): port the initial and post-step thermal state"
```
