# Piano A — Il passo vendorato

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** La copia vendorata contiene il passo fuso di monte con le due Trasmissività al posto di `psi`, e OMBRA lo chiama da `motore::PassoDelloScenario`, verificato bit per bit contro un clone di monte al pin.

**Architecture:** `estrai.py` estende la copia alla chiusura delle dipendenze di `compute_timestep_rust` con regole a conteggio, poi aggiunge due campi raster a `TimestepInputs` e sostituisce le undici righe di ADR 0004. `src/motore.rs` costruisce `TimestepInputs` con un letterale che nomina ogni campo, una volta per Corsa la geometria del suolo e gli scalari di configurazione, e per ogni Ora restituisce Tmrt, ombra del passo, quattro Flussi radiativi e lo Stato termico. Un crate `banco/` con un suo `[workspace]` confronta quel passo con `compute_timestep_rust` del clone.

**Tech Stack:** Rust 2021, `ndarray` 0.16, `rayon`; Python 3 standard library per `estrai.py`; il crate di monte `rustalgos` in `rlib`, solo nel banco.

**Spec:** [`docs/superpowers/specs/2026-10-29-catena-radiativa.md`](../specs/2026-10-29-catena-radiativa.md), sezioni «Il passo, da monte», «Il Motore, in OMBRA», «Decisioni sui test § 2». **Vincoli globali e ordine dei piani:** [indice](2026-11-22-catena-radiativa.md#global-constraints), da leggere prima del primo task. **Chi esegue, in che ordine, con quali revisori:** [annotazione di dispatch](2026-11-22-catena-radiativa-a-passo-dispatch.md); la sua sezione «Difetti» è già integrata in questa versione del piano.

**Fonti da aprire:** `research/catena-radiativa.md` §1-§4 (chiusura, trasformazioni T1-T9, trappola T5, regola dei residui), [ADR 0004](../../adr/0004-psi-per-cella.md) (le undici righe, «Si sostituiscono le legature, non le chiamate»), `docs/ricerca/01-passo-fuso.md:34`, `docs/ricerca/03-chiome-riunite.md:18`, `docs/ricerca/04-stato-termico.md:47`, `docs/ricerca/05-crate-rlib.md` (dopo l'unione di #62). Le righe di monte che queste ricerche citano possono essere scalate al pin (`docs/ricerca/index.md:13`): si rilegge al pin, non si copia il numero.

**Clone di monte al pin**, per ogni task: `git clone --filter=blob:none https://github.com/UMEP-dev/solweig <dir> && git -C <dir> checkout 41840b74cd7c003f88dbde0c30fff736cb51bd5d`, fuori dal repository.

## Review Focus

1. **La sostituzione di `psi` dentro un ciclo per cella.** Uno scalare diventato raster va letto alla cella corrente, non alla prima. Con raster costanti l'errore non si vede: il terzo caso del banco (task 5) divide la Griglia in due metà con trasmissività diverse.
2. **Un conteggio dichiarato che torna per il motivo sbagliato**, per esempio una regola che toglie fino alla `}` sbagliata e un'altra che ne compensa il numero. Il task 2 lo prova su un sorgente costruito apposta.
3. **La maschera degli edifici al contrario.** Monte vuole 1 sul suolo e 0 sotto un Edificio (`rust/src/pipeline_inputs.rs:107`). Il banco non lo vede, perché i due lati ricevono lo stesso ingresso: lo prova un test del task 4.
4. **Raster di forma diversa dal modello di superficie, e `anisotropo` senza una delle tre matrici.** Monte controlla solo le matrici d'ombra e ripiega in silenzio sull'isotropo. Qui sono errori prima del passo (task 4).
5. **Il confronto che misura il pool di thread invece del passo.** OMBRA gira in un pool a un thread (`src/motore.rs:354`), e le riduzioni in `f32` di monte (`rust/src/gvf.rs:243-293`) raggruppano le somme secondo il pool. Il banco chiama anche monte in un pool a un thread (task 5).

---

### Task 1: La regola dei residui guarda gli identificatori

**Files:**
- Modify: `vendor/solweig/estrai.py` (il controllo dei residui, `main()` a `:226`)
- Modify: `.github/workflows/vendor-check.yml` (lancia anche `--prova`)

**Interfaces:**
- Produces: `residui(testo: str) -> list[int]` in `estrai.py`, i numeri di riga (da 1) con codice Python fuori dai commenti `//`. `main()` rifiuta il file se la lista non è vuota. `estrai.py --prova` esegue `prova() -> int`, 0 se ogni caso torna.

- [ ] **Step 1: Scrivi la prova che fallisce**

```python
assert residui("// a numpy view\nlet x = 1;\n") == []
assert residui('let s = "contiguous numpy views";\n') == []
assert residui("use numpy::PyArray2;\n") == [1]
assert residui("fn f(a: PyReadonlyArray2<f32>) {}\n") == [1]
assert residui("let p: Python = py;\n") == [1]
assert residui("fn f(x: Bound<'_, PyAny>) {}\n") == [1]
assert residui("#[pyfunction]\nfn f() {}\n") == [1]
assert residui("use pyo3::prelude::*;\n") == [1]
```

- [ ] **Step 2: Verifica che fallisca**

Run: `python3 -I vendor/solweig/estrai.py --prova` → errore, `residui` non definita.

- [ ] **Step 3: Implementa `residui`**

Codice fuori da `//`, poi `\b(pyo3|numpy)::`, `\bPy[A-Z]\w*`, `\bPython\b`, `\bBound<`, `#\[py` (`research/catena-radiativa.md` §2, «Il controllo dei residui»). Sostituisce il controllo per sottostringhe. `vendor-check.yml` lancia `--prova` prima dell'estrazione, così la prova gira in CI.

- [ ] **Step 4: Verifica la prova e la copia di oggi**

Run: `python3 -I vendor/solweig/estrai.py --prova` → `0`.
Run: `python3 -I vendor/solweig/estrai.py <clone> vendor/solweig/src && git diff --exit-code vendor/solweig/src` → nessuna differenza.

- [ ] **Step 5: Commit**

```bash
git add vendor/solweig/estrai.py .github/workflows/vendor-check.yml
git commit -m "chore(vendor): detect Python residue by identifier outside comments"
```

---

### Task 2: La chiusura del passo fuso

**Files:**
- Modify: `vendor/solweig/estrai.py` (`FILE` a `:49`, `DA_TENERE`, `DA_TOGLIERE`, `SOSTITUZIONI`, regole nuove)
- Modify: `vendor/solweig/PROVENIENZA.toml` (`[cosa_e_stato_preso]`, `[cosa_e_stato_tolto]`, `[modifiche_al_codice_rimasto]`, un conteggio per regola)
- Modify: `.github/workflows/vendor-check.yml` se elenca i file
- Generate: `vendor/solweig/src/*.rs`
- `vendor/solweig/Cargo.toml` non cambia: se la chiusura chiede una dipendenza oltre a `ndarray` e `rayon`, **fermati e riporta**.

**Interfaces:**
- Produces: `solweig_vendorato::api::{compute_timestep_rust, TimestepInputs, TimestepResultRaw, SurfaceViews, SvfViews, PropertyViews, ThermalStateViews, GroundSchemeViews, WeatherScalars, HumanScalars, ConfigScalars, GvfGeometryCache, precompute_gvf_geometry_cpu, OUT_ALL}`, con le firme di monte al pin, e `solweig_vendorato::wall_aspect::compute_wall_aspect_pure` raggiungibile da fuori: oggi è `pub(crate)` (`rust/src/wall_aspect.rs:146`) e il modulo è privato (`rust/src/lib.rs:28`). Il piano B e il banco la consumano.

- [ ] **Step 1: L'elenco dei file**

I moduli di `rust/src/lib.rs:3-28` al pin meno `gpu`, `morphology`, `pet`, `pipeline_bundles`: ventuno file, `api.rs` e `pipeline_inputs.rs` compresi, più i tre già vendorati. `pipeline_bundles` compare solo a `pipeline.rs:508`. Lo script oggi va in `KeyError` su un file senza voce nelle liste (`estrai.py:239-242`): ogni file nuovo ha la sua voce in ogni lista, anche vuota.

- [ ] **Step 2: Scrivi le prove delle regole nuove**

In `prova()`, un caso per regola, su un sorgente costruito apposta:
- **T1** `use numpy::{\n    PyArray2,\n};\nfn f() {}\n` → resta `fn f() {}\n`;
- **T2** `#[pyo3(get, set)]\n    pub ta: f32,\n` → resta `    pub ta: f32,\n`;
- **T5** `pub use crate::pipeline_bundles::{\n    A,\n};\nfn dopo() {\n}\n` → resta `fn dopo() {\n}\n`. Con `DA_TOGLIERE` la regola cancellerebbe anche `dopo`;
- **esporre `pub(crate)`** `pub(crate) fn compute_wall_aspect_pure(` → `pub fn compute_wall_aspect_pure(`; la regola esistente `esponi()` (`estrai.py:208-222`) scriverebbe `pub pub(crate)`;
- **conteggio** una regola con conteggio dichiarato 2 su un sorgente che ne contiene 1 → uscita diversa da 0, con il nome della regola.

- [ ] **Step 3: Verifica che falliscano**

Run: `python3 -I vendor/solweig/estrai.py --prova` → errore sul primo caso nuovo.

- [ ] **Step 4: Implementa le regole e dichiara i conteggi**

Le classi del dossier §2. T1; T2 sugli attributi `#[pyo3(get, set)]` delle tre struct degli scalari. T3 è `DA_TENERE` per `WeatherScalars`, `HumanScalars` e `ConfigScalars`, che perdono `#[pyclass]` e i blocchi `#[pymethods]`. T4 è `DA_TOGLIERE` per `impl Default for WallAspectRunner` (`wall_aspect.rs:405`) e per `allow_threads_unchecked` (`pipeline.rs:287`). T5 (`pipeline.rs:508`); la regola sua per `compute_wall_aspect_pure`. T7 non serve: i campi di `TimestepResultRaw` sono già `pub` al pin. `lib.rs` generato dichiara `pub mod wall_aspect;`. Il commento di modulo di monte che parla di `#[new]` resta, dichiarato in `PROVENIENZA.toml`. Ogni conteggio si legge dal primo giro dello script e si scrive in `PROVENIENZA.toml`.

- [ ] **Step 5: Verifica**

Run: `python3 -I vendor/solweig/estrai.py --prova` → `0`.
Run: `python3 -I vendor/solweig/estrai.py <clone> vendor/solweig/src` → zero residui, ogni conteggio torna.
Run: `cargo build --manifest-path vendor/solweig/Cargo.toml` → zero errori.
Run: il cancello di CI di `ombra` → verde, nessun test cambia esito.

- [ ] **Step 6: Commit**

```bash
git add vendor/solweig/estrai.py vendor/solweig/PROVENIENZA.toml vendor/solweig/src .github/workflows/vendor-check.yml
git commit -m "chore(vendor): extract the fused timestep closure from upstream"
```

---

### Task 3: Le due Trasmissività al posto di `psi`

**Files:**
- Modify: `vendor/solweig/estrai.py`, `vendor/solweig/PROVENIENZA.toml` (sezione per la trasformazione additiva dichiarata di ADR 0004, con il conteggio delle righe toccate per file)
- Generate: `vendor/solweig/src/pipeline_inputs.rs`, `pipeline.rs`, `pipeline_radiation.rs`, `vegetation.rs`, `pipeline_scalars.rs`

**Interfaces:**
- Produces: in `TimestepInputs<'a>` due campi obbligatori, `pub trasmissivita_raggio: ArrayView2<'a, f32>` e `pub trasmissivita_cielo: ArrayView2<'a, f32>`; in `timestep_physics` le legature `let trasmissivita_raggio_v = inputs.trasmissivita_raggio;` e `let trasmissivita_cielo_v = inputs.trasmissivita_cielo;`, accanto a `let vbshmat_v = inputs.vbshmat;`. `WeatherScalars` senza `psi`, `ConfigScalars` senza `conifer`.

- [ ] **Step 1: Scrivi la prova**

In `prova()`: la regola dei campi `psi`/`conifer`, sulla forma reale di `pipeline_scalars.rs` al pin (attributo già tolto da T2, commento `///` sopra il campo), toglie il campo e il suo commento, e si ferma se ne trova zero.

- [ ] **Step 2: Verifica che fallisca**

Run: `python3 -I vendor/solweig/estrai.py --prova` → errore sul caso nuovo.

- [ ] **Step 3: Implementa la sostituzione**

Le undici righe della tabella di ADR 0004 («Il conteggio era sbagliato»), rilette al pin. L'espressione lungo il raggio (`pipeline.rs:679`) usa `trasmissivita_raggio_v` elemento per elemento. Le due `let psi = weather.psi;` (`:761`, `:1318`) diventano legature a `trasmissivita_cielo_v`. Le firme di `compute_ani_lum_from_packed` e `kside_veg_isotropic_pure` prendono `ArrayView2<f32>` al posto di `f32`. **Dove il corpo legge `psi` dentro un ciclo per cella, legge l'elemento della cella corrente.** Le chiamate non si toccano: seguono le legature. Una regola a conteggio toglie i campi `psi` e `conifer` dagli scalari.

- [ ] **Step 4: Verifica**

Run: `python3 -I vendor/solweig/estrai.py --prova` → `0`; lo script sul clone → ogni conteggio torna.
Run: `cargo build --manifest-path vendor/solweig/Cargo.toml` → zero errori.
Run: `grep -n "weather\.psi\|\.conifer\|\bpsi: f32" vendor/solweig/src/*.rs` → vuoto.

La prova di comportamento è il banco del task 5: qui si verifica che compili e che i conteggi tornino.

- [ ] **Step 5: Commit**

```bash
git add vendor/solweig/estrai.py vendor/solweig/PROVENIENZA.toml vendor/solweig/src
git commit -m "feat(vendor): pass beam and sky transmissivity as rasters (ADR 0004)"
```

---

### Task 4: Il passo dell'Ora nel Motore

**Files:**
- Modify: `src/motore.rs` (commento di modulo `:1-5`; accanto a `ombre` `:59` e `sky_view_factor` `:198`)
- Test: `tests/motore.rs`

**Interfaces:**
- Consumes: l'API del task 2 e i due campi del task 3.
- Produces, in `src/motore.rs`:

```rust
pub use solweig_vendorato::api::{HumanScalars, WeatherScalars};

/// What the sky pass yields beyond the two published Sky view factor fields.
pub struct PassataDelCielo {
    pub sky_view_factor: SkyViewFactor,
    /// The kernel's directional fields, named as in upstream `SvfViews`
    /// (svf_n/e/s/w, svf_veg + n/e/s/w, svf_aveg + n/e/s/w). `svfbuveg` is
    /// `sky_view_factor.con_le_chiome` and is not repeated.
    pub direzionali: Direzionali,
    /// The three bit-packed shadow matrices, (rows, cols, 20); `None` unless asked for.
    pub matrici: Option<MatriciDOmbra>,
    /// Per cell: the opaque part the canopies take off the sky (the deduction of
    /// src/motore.rs:320-329), and the vault the Chiome riunite cover, 1 − svf_veg.
    pub detrazione: Raster,
    pub volta_coperta: Raster,
}
pub struct Direzionali { /* one Raster per upstream SvfViews field above */ }
pub struct MatriciDOmbra { pub edifici: Array3<u8>, pub chiome: Array3<u8>, pub chiome_davanti_agli_edifici: Array3<u8> }
pub fn cielo(modello_di_superficie: &Raster, passo_m: f64, strati: &[StratoDiChioma], conserva_matrici: bool) -> Result<PassataDelCielo, String>;

pub struct Muri { pub altezza: Raster, pub orientamento_gradi: Raster }
pub struct ProprietaDelSuolo { pub albedo: Raster, pub emissivita: Raster, pub tgk: Raster, pub tstart: Raster, pub tmaxlst: Raster }
pub struct StatoTermico {
    pub firstdaytime: i32, pub timeadd: f32, pub timestep_dec: f32,
    pub tgmap1: Raster, pub tgmap1_e: Raster, pub tgmap1_s: Raster,
    pub tgmap1_w: Raster, pub tgmap1_n: Raster, pub tgout1: Raster,
}
pub struct UscitaDellOra {
    pub temperatura_media_radiante: Raster,
    /// The step's own lit fraction, 1 in the sun.
    pub illuminata: Raster,
    pub k_giu: Raster, pub k_su: Raster, pub l_giu: Raster, pub l_su: Raster,
    /// `timeadd` and the six rasters from the step; `firstdaytime` and
    /// `timestep_dec` carried from the input, since the step does not return
    /// them (upstream pipeline.rs:249-267). Plan B's `stato_termico::dopo_il_passo`
    /// updates them.
    pub stato: StatoTermico,
}
pub struct IngressiDelloScenario<'a> {
    pub modello_di_superficie: &'a Raster,
    pub passo_m: f64,
    pub strati: &'a [StratoDiChioma],
    /// Building heights above the terrain, > 0 under an Edificio: the `costruite`
    /// of src/corsa.rs:762. The Motore turns it into upstream's mask, 1 on ground.
    pub costruite: &'a Raster,
    pub muri: &'a Muri,
    pub cielo: &'a PassataDelCielo,
    pub svfalfa: &'a Raster,
    pub suolo: &'a ProprietaDelSuolo,
    pub trasmissivita_verso_il_cielo: &'a Raster,
    pub persona: HumanScalars,
    pub anisotropo: bool,
}
pub struct PassoDelloScenario<'a> { /* the inputs, the ConfigScalars, the mask, the GvfGeometryCache */ }
impl<'a> PassoDelloScenario<'a> {
    pub fn nuovo(ingressi: IngressiDelloScenario<'a>) -> Result<Self, String>;
    pub fn ora(&self, meteo: &WeatherScalars, trasmissivita_lungo_il_raggio: &Raster, stato: &StatoTermico) -> Result<UscitaDellOra, String>;
}
```

`ConfigScalars` lo costruisce `nuovo`, non il chiamante: `pixel_size = passo_m`; `max_height` = il rilievo con le chiome, come in `ombre` (`src/motore.rs:74-90`); `use_veg = !strati.is_empty()`; `has_walls = true`; `use_anisotropic = anisotropo`; `max_shadow_distance_m` = il predefinito dello strato di monte (`pysrc/solweig/_orchestration.py:21-82`, da rileggere e citare); i cinque valori dei Muri di `solweig`, costanti con la riga citata (albedo 0,20, emissività 0,90, TgK 0,37, Tstart −3,41, TmaxLST 15,0). `valid_mask` vale 1 dove il modello di superficie è finito. `sky_view_factor` resta con la sua firma e diventa `cielo(…, false).map(|c| c.sky_view_factor)`: i test di oggi non cambiano.

- [ ] **Step 1: Scrivi i test che falliscono, in `tests/motore.rs`**

- `il_passo_rifiuta_un_raster_di_forma_diversa`: `svfalfa` 4 × 5 su un modello di superficie 5 × 5 → `nuovo` è `Err`, e il messaggio nomina `svfalfa`;
- `l_ora_rifiuta_un_raggio_di_forma_diversa`: `trasmissivita_lungo_il_raggio` 5 × 4 → `ora` è `Err`;
- `anisotropo_senza_matrici_si_ferma`: `anisotropo = true`, `cielo.matrici = None` → `nuovo` è `Err`;
- `la_maschera_degli_edifici_ha_il_verso_di_monte`, test di modulo (`#[cfg(test)]` in `src/motore.rs`) sulla funzione privata `maschera_degli_edifici(costruite: &Raster) -> Raster`: una cella con `costruite = 6.0` dà 0, una con `0.0` dà 1, una con `NaN` dà 1;
- `il_passo_su_un_piano_aperto_e_finito`: 8 × 8 celle piane, nessun Edificio, nessuna chioma, Muri a zero, mezzogiorno estivo (`sun_altitude = 60`, `ta = 30`, `global_rad = 800`, `direct_rad = 700`, `diffuse_rad = 150`) → ogni cella di `temperatura_media_radiante` è finita e fra `ta − 30` e `ta + 70`, `illuminata` vale 1, `k_giu` > 0 ovunque;
- `cielo_conserva_le_matrici_solo_se_chiesto`: `cielo(…, false).matrici.is_none()`; `cielo(…, true)` dà tre matrici di forma `(righe, colonne, 20)` (`skyview.rs:267`);
- `senza_chiome_i_campi_della_vegetazione_valgono_uno`: nessuno Strato → `direzionali.svf_veg` e i suoi quattro direzionali valgono 1 ovunque, come `precomputed.py:143-152` al pin e ADR 0006:108-110.

- [ ] **Step 2: Verifica che falliscano**

Run: `cargo test -p ombra --test motore` → errori di compilazione sui tipi nuovi.

- [ ] **Step 3: Implementa**

`cielo` estende la passata di `sky_view_factor`: con due Strati di chioma le matrici della vegetazione vengono dalla passata sulle Chiome riunite ([ADR 0006](../../adr/0006-chiome-riunite.md)). `TimestepInputs` si costruisce con un **letterale che nomina ogni campo**, senza `..Default::default()`: un campo aggiunto o tolto a monte rompe la compilazione. `cdsm` e `tdsm` dalle Chiome riunite, `bush` a zeri, `ground_scheme: None`, `lc_grid: None`, `output_mask: OUT_ALL`, `gvf_cache: Some(&self.gvf)`. La cache si costruisce in `nuovo` con `precompute_gvf_geometry_cpu`; `first_ht` e `second_ht` vengono dall'altezza del baricentro della persona con la formula di `pipeline.rs:415-416` (`round().max(1)`, `* 20`), che sta nella pyfunction tolta dall'estrazione: va portata e citata. Il passo gira in `pool_a_un_thread()`, come `sky_view_factor`. Un `Err` di monte passa invariato. Il commento di modulo `:1-5` si riscrive com'è vero oggi: anche `src/comfort.rs:37` nomina la copia.

- [ ] **Step 4: Verifica**

Run: `cargo test -p ombra --test motore` → PASS, compresi i test di oggi. Poi il cancello di CI.

- [ ] **Step 5: Commit**

```bash
git add src/motore.rs tests/motore.rs
git commit -m "feat(motore): run one hour of the fused timestep"
```

---

### Task 5: Il banco di parità

Due parti: 5a misura la via e può partire subito; 5b scrive il confronto dopo il task 4.

**Files:**
- Create: `banco/Cargo.toml` (con un suo `[workspace]`; `ombra = { path = ".." }`, `rustalgos = { path = "monte/rust", default-features = false }`), `banco/Cargo.lock`, `banco/prepara.sh`, `banco/tests/parita.rs`, `banco/README.md`
- Modify: `.gitignore` (`banco/monte/`, `banco/target/`)
- Modify: `.github/workflows/ci.yml` (un lavoro `parita`, su `ubuntu-latest`, accanto al lavoro `verifica`)

**Interfaces:**
- Consumes: `motore::{cielo, PassoDelloScenario, StatoTermico}` (task 4); `solweig_vendorato::wall_aspect::compute_wall_aspect_pure` (task 2); `rustalgos::api::compute_timestep_rust` dal clone.

- [ ] **Step 1 (5a): Misura la via su `ubuntu-latest`**

`docs/ricerca/05-crate-rlib.md` raccomanda A2, `extension-module` tolto dal manifesto del clone dopo il checkout, con `PYO3_CONFIG_FILE` (B2) come riserva, e dice che Linux non è misurato. `prepara.sh` clona al commit di `PROVENIENZA.toml`, toglie `extension-module` da `monte/rust/Cargo.toml` e stampa la versione di Python trovata. Lancia il lavoro `parita` con un test vuoto su un branch (serve il sì di Mario al push) e scrivi in `banco/README.md` l'esito: collega o no, con quale Python, in quale profilo. Se A2 non collega, prova B2. Se nessuna delle due collega, **fermati e riporta**.

- [ ] **Step 2 (5b): Scrivi i test di parità**

Un ingresso costruito nel test, 16 × 16 celle a 1 m: un Edificio di 4 × 4 celle alto 6 m, un Albero con chioma a 5-8 m e zona tronco fino a 2 m, Muri ricavati con `compute_wall_aspect_pure` della copia vendorata, suolo uniforme ai valori della classe 1 di monte (`pysrc/solweig/models/surface.py:1959-1966`), `anisotropo`, persona in piedi. Sei Ore consecutive del 21 giugno, dalle 14 alle 20, a 43,07 N 12,55 E, fuso +1: le prime cinque di giorno, l'ultima a cavallo del tramonto. Lo Stato termico passa da un'Ora all'altra copiando l'uscita nell'ingresso, uguale sui due lati; `firstdaytime` e `timestep_dec` restano quelli iniziali. **Monte gira in un `rayon::ThreadPool` a un thread**, come OMBRA.

- `con_trasmissivita_costanti_il_passo_e_monte_bit_per_bit`: raggio e cielo costanti a 0,03, monte con `psi = 0.03`. Per ogni Ora `tmrt`, `shadow`, `kdown`, `kup`, `ldown`, `lup` hanno gli stessi bit (`to_bits()` cella per cella); dopo la sesta Ora gli stessi bit hanno `timeadd` e i sei raster dello Stato termico;
- `raggio_e_cielo_non_si_scambiano`: raggio a 0,5, cielo a 0,03. In un'Ora di giorno `illuminata` è uguale bit per bit a `shadow` di monte con `psi = 0.5`, e `k_giu` sotto la chioma è diverso da quello di monte con `psi = 0.5`: il cielo è stato letto;
- `la_trasmissivita_si_legge_cella_per_cella`: raggio a 0,03 sulla metà ovest e 0,5 sulla metà est, cielo costante a 0,03. In un'Ora di giorno `illuminata` è uguale bit per bit, sulla metà ovest, a `shadow` di monte con `psi = 0.03`, e sulla metà est a quello con `psi = 0.5`. L'ombra è elemento per elemento (`pipeline.rs:679`), quindi l'oracolo è esatto.

- [ ] **Step 3: Verifica che falliscano, poi che passino**

Run: `bash banco/prepara.sh && cargo test --manifest-path banco/Cargo.toml --release` → prima del codice di confronto FAIL, dopo PASS.

- [ ] **Step 4: Il lavoro di CI**

Il lavoro `parita` lancia `prepara.sh`, che esce con codice diverso da 0 se il clone non c'è: un banco che si salta da solo darebbe verde senza confrontare niente. Il lavoro `verifica` di `ci.yml` resta senza Python. Il trigger di `parita` (ogni PR o solo con percorsi sotto `vendor/`, `src/motore.rs`, `banco/`) è una decisione di Mario: chiedila prima del commit.

- [ ] **Step 5: Commit**

```bash
git add banco/Cargo.toml banco/Cargo.lock banco/prepara.sh banco/tests/parita.rs banco/README.md .gitignore .github/workflows/ci.yml
git commit -m "test(banco): compare the vendored timestep with upstream bit for bit"
```
