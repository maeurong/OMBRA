# La catena radiativa fino alla Tmrt — chiusura, trasformazioni, cosa resta in Python

**Scopo**: Fase 1 del lavoro sulla catena radiativa (branch `feat/catena-radiativa-tmrt`).
Ricognizione in sola lettura, prima di estendere `vendor/solweig/estrai.py` al passo fuso
`compute_timestep`, come prevede il ripiego di [ADR 0003](../docs/adr/0003-catena-radiativa-e-cucitura-a-monte.md).

**Data ricerca**: 2026-10-29.
**Repo CLIMESH**: HEAD `1f1852f` (2026-09-08), 244 test di cui 3 ignorati, vendor-check identico.
**Motore letto**: `UMEP-dev/solweig` al commit pinnato `02246ab71a3a8b127d740dde9640449ee9d558ff`.
I numeri di riga di `rust/src/` e `pysrc/solweig/` si riferiscono a quel commit.

**Convenzione di marcatura** (estende quella di `research/motore-api.md`)
- `[P]` = letto nel sorgente o nell'API di GitHub.
- `[C]` = verificato **compilando**: un crate di prova fuori dal repository, con le regole di
  `estrai.py` applicate ai file di monte e le trasformazioni proposte qui simulate da uno script.
- `[I]` = inferenza, non verificata né compilando né eseguendo.

---

## 0. Risposta in breve

1. **Il pin è ancora l'ultimo commit su `rust/`.** L'ultimo commit che tocca il percorso è
   `9447c11` del 2026-07-09; dopo il pin c'è solo `813876f` del 2026-10-01, che corregge
   l'orologio dei dati di validazione di Göteborg e non tocca `rust/`. La richiesta
   [UMEP-dev/solweig#14](https://github.com/UMEP-dev/solweig/issues/14) è aperta, senza commenti
   e senza PR collegate; `crate-type` è ancora `["cdylib"]`. `[P]`
2. **La chiusura del passo fuso è di 17 moduli**, muri compresi, e **compila senza Python**
   con le regole attuali più sei trasformazioni nuove, tutte meccaniche. `[C]`
3. **Nessuna firma `PyResult` sopravvive nei moduli foglia**: stavano tutte negli involucri. Le
   uniche costruzioni d'errore sono le tre validazioni di `compute_timestep`, e restano vive
   con la regola già esistente `new_err → String::from`. `[C]`
4. **La trasformazione di `compute_timestep` funziona** nella forma proposta: firma sostituita
   da una struct di ingresso più una destrutturazione, legatura numpy cancellata fra marcatori,
   chiusura invocata sul posto, riconversione sostituita da `Ok(raw)`. La chiusura — 1245 righe
   — resta identica. `[C]`
5. **Il controllo dei residui di `estrai.py` va rifatto prima di estendere la lista dei file.**
   Così com'è scatta su commenti e su un letterale stringa che nominano numpy, e non vede
   `PyReadonlyArray2` né `Python`. `[C]`
6. **Una parte non piccola dell'orchestrazione è Python**, non Rust: la preparazione degli
   scalari meteo, le costanti delle pareti, le proprietà del suolo, la maschera degli edifici,
   l'altezza dei muri, e l'aggiornamento di due campi dello stato termico dopo ogni passo. Non
   si vendora: si porta, citando la riga di monte (decisione di Mario, 2026-10-29). `[P]`
7. **ADR 0004 non è ancora applicato** e rientra in questo lavoro: `psi` è letto da
   `weather.psi` in tre punti e passato in altri cinque. `[P]`

---

## 1. Chiusura delle dipendenze

Determinata in due passi: un grep transitivo su `crate::` a partire da `pipeline.rs` e
`wall_aspect.rs`, poi la compilazione, che ha trovato il modulo che il grep non vedeva
(`patch_radiation`, importato da `sky.rs` in un `use crate::{…}` con le graffe).

| Modulo | Righe a monte → dopo | Chi lo chiede | Note |
|---|---|---|---|
| `pipeline.rs` | 2021 → 1594 | — | il passo fuso; trasformazione §3 |
| `pipeline_scalars.rs` | 189 → 56 | `pipeline` | `WeatherScalars`, `HumanScalars`, `ConfigScalars` |
| `pipeline_radiation.rs` | 430 → 430 | `pipeline` | porta nativa, nessun involucro |
| `perez.rs` | 445 → 420 | `pipeline_radiation` | |
| `gvf.rs` | 772 → 555 | `pipeline` | |
| `gvf_geometry.rs` | 487 → 487 | `gvf`, `sun`, `pipeline` | porta nativa |
| `sun.rs` | 497 → 497 | `gvf` | porta nativa |
| `ground.rs` | 499 → 232 | `pipeline` | |
| `ground_surface.rs` | 791 → 617 | `pipeline` | schema di suolo 2026a, opzionale |
| `sky.rs` | 857 → 574 | `pipeline` | `use numpy::{…}` su più righe, §2 |
| `patch_radiation.rs` | 249 → 249 | `sky` | |
| `emissivity_models.rs` | 28 → 28 | `sky` | |
| `sunlit_shaded_patches.rs` | 32 → 32 | `sky`, `vegetation` | |
| `tmrt.rs` | 243 → 142 | `pipeline` | |
| `vegetation.rs` | 1238 → 515 | `pipeline` | |
| `wall_aspect.rs` | 458 → 378 | i muri | `impl Default for WallAspectRunner`, §2 |
| `shadowing.rs` | già vendorato | `pipeline` | |

`[C]` per l'insieme: il crate di prova con questi 17 moduli compila, zero errori. I conteggi
delle righe sono quelli dello script di prova e cambieranno di poco con le regole definitive.
`skyview.rs` e `utci.rs` restano come sono; `pipeline` non li importa.

**Fuori dalla chiusura**: `pipeline_bundles.rs`, che dopo l'estrazione lascerebbe solo due
costanti di versione di un protocollo Python (`STATE_BUNDLE_VERSION`,
`GROUND_SCHEME_BUNDLE_VERSION`), e il modulo `gpu`, che compare solo dietro
`#[cfg(feature = "gpu")]` e si compila fuori da sé come già oggi. `morphology.rs` e `pet.rs` non
sono chiesti da nessuno. `[C]`

**Dimensione**: la copia vendorata passerebbe da circa 1.850 a circa 8.650 righe. `[I]`, somma
dei conteggi sopra più `skyview.rs` e `utci.rs`.

---

## 2. Trasformazioni, modulo per modulo

Le classi esistenti (togliere `use numpy::`/`use pyo3::`, gli elementi con `#[pyfunction]`,
`#[pyclass]`, `#[pymethods]`, `PyResult` e `new_err`, `DA_TENERE`, `DA_TOGLIERE`,
`DA_ESPORRE`) bastano per la maggior parte dei moduli. Ne servono di nuove, tutte con un
conteggio atteso su cui lo script si ferma.

| # | Trasformazione | Dove | Conteggio trovato | Classe |
|---|---|---|---|---|
| T1 | togliere un `use numpy::{` o `use pyo3::{` che va a capo, fino al `};` | `sky.rs:3-5` | 1 | **nuova** |
| T2 | togliere gli attributi di campo `#[pyo3(get, set)]` | `pipeline_scalars.rs` | 34 | **nuova** |
| T3 | tenere senza attributo le tre struct degli scalari | `pipeline_scalars.rs` | 3 | `DA_TENERE`, esistente |
| T4 | togliere per nome `impl Default for WallAspectRunner` e `unsafe fn allow_threads_unchecked` | `wall_aspect.rs:405`, `pipeline.rs:263` | 2 | `DA_TOGLIERE`, esistente |
| T5 | togliere `pub use crate::pipeline_bundles::{…};` | `pipeline.rs:482-485` | 1 | **nuova**: vedi la trappola sotto |
| T6 | il passo fuso: firma, legatura, chiusura, riconversione | `pipeline.rs:500-2021` | 5 marcatori | **nuova**, §3 |
| T7 | `TimestepResultRaw` e i suoi 17 campi diventano `pub` | `pipeline.rs:230-248` | 18 | **nuova** (esporre campi) |
| T8 | esporre `GvfGeometryCache`, `precompute_gvf_geometry_cpu`, `compute_wall_aspect_pure` | `gvf_geometry.rs:37,255`, `wall_aspect.rs:146` | 3 | `DA_ESPORRE`, esistente |
| T9 | `psi` per cella (ADR 0004) | `pipeline.rs:744,826,1383` + 5 usi | da contare | **nuova**, §4 |

**Firme e costruzioni d'errore** dopo le rimozioni: zero firme `PyResult` in tutti i moduli,
tre `new_err` in `pipeline.rs` (righe 661, 682, 691 a monte). `[C]`

**La trappola di T5.** `estrai()` toglie un elemento scorrendo fino alla prima riga che è
esattamente `}` in colonna zero. Il `pub use` chiude con `};`, quindi se fosse aggiunto a
`DA_TOGLIERE` lo script non si fermerebbe lì e **cancellerebbe in silenzio** il codice
seguente, fino alla graffa di chiusura della prima funzione. Serve una regola sua, che si fermi
su `};`. `[P]` sulla regola di `estrai()`, `[I]` sull'effetto: non provato apposta.

**Il controllo dei residui.** Oggi `estrai.py` rifiuta un file se una riga qualunque contiene
`pyo3`, `numpy`, `PyArray`, `Py<` o `PyResult`. Sui 17 moduli scatta su righe innocue:

- commenti che descrivono il porting: `ground_surface.rs:295`, sei righe di `wall_aspect.rs`;
- un **letterale stringa** in codice vivo: `vegetation.rs:13`, «contiguous numpy views».

E non vede ciò che dovrebbe: `PyReadonlyArray2` non contiene `PyArray`, e `Python` nessuna
delle cinque sottostringhe. La regola provata guarda solo il codice fuori da `//` e cerca
identificatori: `\b(pyo3|numpy)::`, `\bPy[A-Z]\w*`, `\bPython\b`, `\bBound<`, `#[py`. Dà zero
residui sul crate di prova e sulla copia vendorata di oggi, e 77 righe sul `pipeline.rs` di
monte. `[C]`

Ha un limite noto: taglia anche un `//` dentro un letterale stringa. Nei 17 file non succede,
e se succedesse il residuo comparirebbe invece di sparire, quindi l'errore sarebbe rumoroso.

---

## 3. `compute_timestep`: la forma provata

La funzione di monte (`pipeline.rs:500-2021`) è fatta di quattro regioni.

| Righe | Cosa fa | Cosa diventa |
|---|---|---|
| 500-533 | firma: tre `&…Scalars`, cinque `&…Bundle` `#[pyclass]`, sei `PyReadonlyArray`, `output_mask` | **sostituita**: `pub fn compute_timestep(inputs: TimestepInputs<'_>) -> Result<TimestepResultRaw, String>` più la destrutturazione di `inputs` |
| 534-652 | legatura: `.bind(py).readonly()` e `.as_array()`, ~70 viste | **cancellata**: i nomi `*_v` arrivano come campi di `TimestepInputs` |
| 653-700 | `want_*` da `output_mask`, poi tre validazioni | **tenuta**, con le tre `new_err` sostituite dalla regola esistente |
| 701-709 | `gvf_cache.map(\|c\| &c.inner)`, commento `SAFETY`, `unsafe { allow_threads_unchecked(py, \|\| {` | `let raw = { (\|\| {` |
| 709-1953 | la chiusura: fisica | **identica** |
| 1953-1954 | `})` e `}; // end allow_threads_unchecked` | `})()` e la stessa riga |
| 1955-2021 | riconversione in `PyArray` | `Ok(raw)` |

La chiusura deve restare una chiusura: alla riga 1149 contiene un `return TimestepResultRaw {…}`
per il ramo notturno, che con un blocco semplice uscirebbe dalla funzione invece che dalla
chiusura. Invocarla sul posto, `(|| { … })()`, la lascia intatta. `[C]`

**Perché una struct e non una firma piatta.** I campi di `TimestepInputs` portano i nomi esatti
delle viste catturate (`dsm_v`, `svf_n_v`, `sch_a1_v`, …), e la destrutturazione li rimette in
scope senza toccare la chiusura. Le alternative:

- *firma piatta da 62 parametri*: stessa sicurezza, ma ogni chiamata in `motore.rs` diventa una
  lista posizionale illeggibile, dove due raster scambiati compilano;
- *ricostruire i bundle come struct Rust*: tiene la forma di monte, ma ogni bundle diventa un
  tipo nostro dentro `vendor/` con il suo costruttore, cioè più testo nostro, non meno.

Con la struct, ogni discrepanza con la chiusura è un errore di compilazione: una vista che la
chiusura usa e la struct non ha dà «cannot find value»; un campo dichiarato e non usato dà un
avviso nella copia vendorata. `[C]` per il primo caso, che ha trovato `sch_a1_v`, `sch_a2_v` e
`sch_a3_v`, sfuggiti al grep.

**Il testo nostro dentro `vendor/`.** T6 è il primo punto in cui `estrai.py` inserisce codice
invece di togliere righe o sostituirne in modo meccanico: la struct (62 campi) e la
destrutturazione. Vive nello script, non in un file a mano, quindi il vendor-check continua a
confrontare byte per byte.

**I marcatori** su cui lo script si ferma se non li trova esattamente una volta:

1. `pub fn compute_timestep(\n` (inizio della firma);
2. `    let vbshmat_v = vbshmat.as_ref().map(|a| a.as_array());\n` (fine della legatura);
3. `    // Extract GVF cache reference (pure Rust data) before releasing the GIL\n` fino a
   `allow_threads_unchecked(py, || {\n`;
4. `        })\n    }; // end allow_threads_unchecked\n`;
5. `    // ── Convert final outputs to PyArrays (needs GIL)` fino alla prima `\n}\n`.

**Lo schema di suolo 2026a** (`ground_scheme`, i campi `sch_*`) resta nella struct come `None`.
Toglierlo vorrebbe dire riscrivere i rami della chiusura che lo leggono, ed è proprio la
modifica che questa forma evita. CLIMESH lo lascia spento: il ramo classico è quello che a monte
chiamano «byte-identical baseline».

---

## 4. `psi` per cella (ADR 0004)

Letture di `weather.psi` nella chiusura, al commit pinnato: `[P]`

- `pipeline.rs:744`: `bldg_sh - &((1.0 - veg_sh) * (1.0 - weather.psi))`, cioè l'ombra del
  raggio diretto;
- `pipeline.rs:826` e `pipeline.rs:1383`: `let psi = weather.psi;`, poi `psi` passato alle
  righe 862, 1090, 1571, 1699 (`compute_ani_lum_from_packed`) e 1803.

ADR 0004 conta «uno per il raggio, sette per il cielo». Il conteggio delle righe coincide (1 +
2 letture + 5 passaggi), ma **non ho verificato** che i cinque passaggi siano tutti verso il
cielo, né che le funzioni chiamate accettino un raster al posto di uno scalare senza
cambiarne la firma. `[I]` È il primo lavoro della Fase 2d, e il test che ADR 0004 chiede è che
un raster costante riproduca a monte byte per byte.

---

## 5. Lo strato Python da portare

`compute_timestep` riceve scalari e raster già preparati. Li prepara `calculate_core_fused` in
`pysrc/solweig/computation.py:67-705`, più le funzioni che chiama. Elenco di ciò che **calcola**
(non di ciò che sposta in memoria), con quello che CLIMESH ha già.

| Grandezza | Dove a monte | CLIMESH oggi | Nota |
|---|---|---|---|
| posizione del sole, `sun_zenith` | `models/weather.py:121-…`, SPA in `physics/sun_position.py` | `src/sole.rs` | a monte il sole è calcolato a **metà passo** (`datetime - timestep/2`, `weather.py:150-151`); da confrontare con la nostra convenzione |
| `altmax` | `weather.py`, massimo su passi da 15 min | — | da portare |
| `clearness_index`, ripartizione diretta/diffusa | `weather.py:216-219` e seguenti | EPW: diretta normale, diffusa orizzontale | da verificare se a monte `direct_rad` è normale o orizzontale `[I]` |
| `snup` | `physics/daylen.py:6` (35 righe) | — | da portare |
| `dectime`, `zen_deg`, `jday` | `computation.py:309-310,353` | — | aritmetica |
| `rad_g0` | `physics/clearnessindex_2013b.py:11` (120 righe) + `physics/diffusefraction.py:6` (50 righe), `computation.py:319-332` | — | da portare |
| `psi` | `components/shadows.py:13` (`compute_transmissivity`) | per Strato, `StratoDiChioma::trasmissivita` | **sostituito** dal nostro, ADR 0004 |
| `svfbuveg` | `components/svf_resolution.py:117` (`adjust_svfbuveg_with_psi`) | `SkyViewFactor::con_le_chiome` | stessa formula, già nostra per Strato |
| `svfalfa` | `components/svf_resolution.py:95-100` | — | una riga: `arcsin(sqrt(1 - (svf + svf_veg - 1)))`, con il clamp di monte |
| SVF direzionali, `svf_aveg_*`, matrici d'ombra | `calculate_svf_inner` (già vendorato) | se ne usano solo `svf` e `svf_veg` | `svf_veg_blocks_bldg_sh*` è ciò che il bundle chiama `svf_aveg*` `[I]`, da confermare in `resolve_svf` |
| costanti delle pareti | `computation.py:291-295`: `tgk 0.37`, `tstart -3.41`, `tmaxlst 15.0`, `albedo 0.20`, `emis 0.90` | — | **le costanti assenti dai paper** di cui parla ADR 0002: si portano con la riga citata |
| proprietà del suolo per cella | `models/surface.py` (`get_land_cover_properties`) + `parametersforsolweig.json` | Superfici a elenco chiuso | mappa classe → albedo, emissività, `tgk`, `tstart`, `tmaxlst` |
| maschera degli edifici | `components/gvf.py:39` (`detect_building_mask`) | — | da portare |
| altezza dei muri | `physics/wallalgorithms.py:10` (`findwalls`) | — | da portare; l'aspetto è Rust (`compute_wall_aspect_pure`) |
| parametri della persona | `models/config.py:286-293`: in piedi, `abs_k 0.7`, `abs_l 0.97`, altezza 1,75 m | — | predefiniti, dichiarati nel Giornale |
| `max_shadow_distance_m` | `computation.py:376`: 1000 se assente | — | |
| stato iniziale | `models/state.py:62-82`: raster a zero, `firstdaytime 1`, `timeadd 0`, `timestep_dec 0` | — | |
| `timestep_dec` | `timeseries.py:409-412`: passo in frazione di giorno, 1/24 se ignoto | — | |
| **stato dopo il passo** | `computation.py:637-657` | — | `timeadd` e `tgmap*` tornano dal Rust, ma **`firstdaytime` lo decide Python**: 0 di giorno; di notte 1, con `timeadd` azzerato |

L'ultima riga è la più delicata: è logica di stato, non preparazione, e se il porto la sbaglia
la temperatura del suolo deriva lungo il Periodo senza che nessun test di un passo singolo se ne
accorga. Va coperta da un test su più passi. `[P]` sul codice, `[I]` sull'effetto.

**Cosa il banco di parità non copre.** Il banco deciso è il clone di monte allo stesso pin
patchato in `rlib`, con `compute_timestep_rust` estratta. Chiama il passo Rust direttamente e
quindi salta tutto ciò che sta in questa tabella. Le funzioni portate hanno bisogno di
riferimenti propri: i test di monte quando esistono, altrimenti valori calcolati a mano dalle
formule dei paper citati nei file.

---

## 6. Il cielo anisotropo con più Strati di chioma

`compute_timestep` prende **una** `vegshmat`, **una** `vbshmat` e **un** insieme di `svf_veg*`
direzionali. CLIMESH fa una passata di `calculate_svf_inner` per Strato di chioma (`src/motore.rs:184-259`),
e d'inverno gli Strati sono due. Serve una regola per comporli in un ingresso solo. È la parte
concreta dell'approssimazione che ADR 0004 dichiara («una cella che vede un sempreverde in una
patch e una chioma spoglia in un'altra riceve una sola trasmissività verso il cielo»).

Due strade, da decidere in Fase 3:

- **una passata SVF con tutte le chiome insieme** per la geometria, e la trasmissività verso il
  cielo per cella a pesare: la geometria è esatta, l'approssimazione sta tutta nella
  trasmissività ed è quella che ADR 0004 già dichiara;
- **comporre le matrici bit a bit** (bloccata se un qualunque Strato blocca): nessuna passata in
  più, ma i `svf_veg*` direzionali vanno composti a parte, e la composizione di due frazioni di
  cielo non è un'operazione bit a bit.

La prima mi sembra la più onesta. `[I]`

---

## 7. Rischi

**Manutenibilità del ripiego.** T6 è testo nostro dentro `vendor/`, legato a cinque marcatori e
a 62 nomi di variabili di monte. Un aggiornamento del pin che rinomina una vista o riordina la
legatura ferma lo script o la compilazione: il guasto si vede, ma ogni aggiornamento chiederà di
rileggere la struct a mano. È il motivo per cui ADR 0003 lo chiama ripiego, e il motivo per cui
le PR 1 e 2 a monte restano la via d'uscita.

**Perdita di dati.** Nessuna nel repository: `vendor/solweig/src` è un prodotto rigenerabile. Le
tre validazioni di `compute_timestep` non vanno perse, perché restano nella copia (§3).

**Performance.** Non misurata. Il caso di riferimento sono 192 passi orari su 2.500 celle, con
GVF, un cielo a 153 patch e il pool rayon a un thread. La `GvfGeometryCache` si calcola una volta
per Scenario, come a monte. Oggi i 0,47 s contro 60 lasciano margine, ma il passo fuso è la parte
pesante e va misurato in release prima di dichiarare la Fase 4.

**Costanti portate.** Le costanti di §5 sono numeri senza fonte bibliografica nel file di monte.
Il porto le cita per riga e per commit, così chi aggiorna il pin vede quando cambiano.

---

## 8. Non verificato

- Che i cinque passaggi di `psi` (§4) siano tutti verso il cielo, e che le firme a valle reggano
  un raster.
- Il nome `svf_aveg` ↔ `svf_veg_blocks_bldg_sh` (§5).
- Se il `direct_rad` di monte sia normale o orizzontale (§5): decide la conversione dall'EPW.
- Il comportamento notturno della chiusura (`pipeline.rs:1149`) quando CLIMESH chiama anche le
  ore senza sole: a monte `calculate_core_fused` è chiamata per ogni passo, e la chiusura ha un
  ramo che restituisce prima.
- L'effetto della trappola di T5: dedotto dalla regola, non provato.

**Come si rifà la prova.** Gli script stanno fuori dal repository, perché sono una ricognizione
e non un'estrazione: applicano le funzioni di `estrai.py` a un clone di monte al pin e simulano
T1-T7. La Fase 2 li sostituisce con regole dichiarate in `estrai.py` e `PROVENIENZA.toml`, una
per commit.
