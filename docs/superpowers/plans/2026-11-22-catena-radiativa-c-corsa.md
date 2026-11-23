# Piano C — La Corsa fino alla UTCI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Una Corsa calcola per ogni Ora e per ogni cella la Tmrt e la UTCI, scrive ai Punti le serie di Tmrt, UTCI e dei quattro Flussi radiativi, e il Giornale dichiara le scelte di modellazione e tre verifiche nuove.

**Architecture:** Il Progetto prende tre scelte con un predefinito, che entrano nell'Impronta. `corsa::marcia` costruisce una volta per Corsa ciò che non cambia con l'Ora (passata del cielo, Muri, suolo, Trasmissività verso il cielo, `PassoDelloScenario`), poi per ogni Ora abbina il record, chiama il passo, porta lo Stato termico all'Ora dopo e calcola la UTCI sul campo. Il Giornale guadagna sei sezioni, la sezione `[ore]` in ISO 8601 e gli inviluppi con l'Ora e la cella degli estremi. Due meteo sintetici di 48 Ore rendono il caso di riferimento eseguibile in CI, e le Corse registrate su Bastia fissano il risultato contro monte.

**Tech Stack:** Rust 2021, `ndarray` 0.16, `serde`, `toml`.

**Spec:** [`docs/superpowers/specs/2026-10-29-catena-radiativa.md`](../specs/2026-10-29-catena-radiativa.md), sezioni «La Corsa», «Il Progetto», «Il Giornale», «Vocabolario e lingua», «Decisioni sui test § 1». **Vincoli globali e ordine dei piani:** [indice](2026-11-22-catena-radiativa.md#global-constraints). **Chi esegue, in che ordine, con quali revisori:** [annotazione di dispatch](2026-11-22-catena-radiativa-c-corsa-dispatch.md); la sua sezione «Difetti» è integrata in questa versione, e l'ordine dei task qui sotto è il suo.

**Dipende dai piani A e B**, uniti in `main`. Il task 1 aspetta anche la PR [#53](https://github.com/maeurong/OMBRA/pull/53) (la voce Postura).

**Fonti da aprire:** i commenti di risoluzione dei ticket [#25](https://github.com/maeurong/OMBRA/issues/25), [#36](https://github.com/maeurong/OMBRA/issues/36), [#21](https://github.com/maeurong/OMBRA/issues/21), [#20](https://github.com/maeurong/OMBRA/issues/20), [#63](https://github.com/maeurong/OMBRA/issues/63), [#28](https://github.com/maeurong/OMBRA/issues/28); [ADR 0005](../../adr/0005-il-sole-al-centro-dell-ora.md); `docs/ricerca/04-stato-termico.md:102`, `:151`, `:257`; `docs/ricerca/01-passo-fuso.md:186`; `docs/ricerca/02-sole-meta-passo.md:105`.

**Decisioni di Mario del 2026-11-23** che questo piano applica: una Corsa senza file meteo si ferma; «Bastia com'è» si registra d'estate e d'inverno; la versione sale a 0.5.0 con la catena (la 0.4.0 l'ha portata il sole, piano B).

## Review Focus

1. **Il caso di riferimento senza meteo in CI.** I Periodi di Bastia puntano a `materiale università/` (`casi/bastia/progetto/periodi/estate.toml:2`), assente in CI e su questa macchina. I test della Corsa girano sui meteo sintetici del task 7a; un test che legge il file vero si salta da solo e lo dice nel nome.
2. **Il vento mancante in un'Ora sola.** La Tmrt c'è, la UTCI di quell'Ora è NaN, conta nella frazione senza dato e non alza la bandiera (task 3 e task 5c).
3. **Due Corse con scelte di Progetto diverse e oggetti identici.** Hanno Impronte diverse, provato con `corsa::esegui` sugli stessi ingressi (task 1).
4. **Un fuso non intero**, +05:30 e +05:45, e il 29 febbraio, nell'ISO 8601 delle Ore (task 4).
5. **Un Progetto scritto dalla 0.3.0**, senza le tre chiavi nuove: si legge con i predefiniti (task 1).

---

### Task 1: Le tre scelte del Progetto e la persona

**Files:**
- Modify: `src/dominio.rs` (accanto a `Progetto` `:238`), `src/progetto.rs` (`Manifesto` `:100-106`, `valida`, `ProgettoError` `:21`, `Display` `:54-94`), `src/corsa.rs` (`Parametri`), `src/motore.rs` (la persona), `src/da_inx.rs:313` e `tests/cli.rs:104` (costruiscono `Progetto` per letterale)
- Test: `tests/progetto.rs`, `tests/corsa.rs`, `tests/motore.rs`

**Interfaces:**
- Produces, in `src/dominio.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelloDiCielo { #[default] Anisotropo, Isotropo }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Postura { #[default] #[serde(rename = "in piedi")] InPiedi, #[serde(rename = "seduta")] Seduta }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScelteDiProgetto {
    pub modello_di_cielo: ModelloDiCielo,
    pub postura: Postura,
    pub soglia_del_muro_m: f64,
}
// `impl Default for ScelteDiProgetto` by hand, with soglia_del_muro_m = 1.0:
// a derived Default would give 0.0, and `valida` would refuse every 0.3.0 Progetto.
// In Progetto: `pub scelte: ScelteDiProgetto`; in Manifesto: `#[serde(default)] scelte`.
```

- Produces: `ProgettoError::SogliaDelMuro(f64)`, il cui messaggio, nel `Display` come gli altri errori del modulo, cita `solweig@41840b7 rust/src/wall_aspect.rs:156` e dice che sotto 0,5 m l'orientamento resta a nord.
- Produces, in `src/motore.rs`: `pub fn persona(postura: Postura) -> HumanScalars`, con `height` 1,1 m in piedi e 0,75 m seduta (`pipeline_scalars.rs:103-105`), `abs_k = 0.70`, `abs_l = 0.97`, `is_standing`. Il commento di `abs_l` dichiara lo scarto da UMEP-QGIS (0,95, circa 0,4 K a 40 °C); quello dell'altezza seduta dichiara che monte non cita una fonte.

- [ ] **Step 1: Scrivi i test che falliscono**

- `un_manifesto_senza_scelte_prende_i_predefiniti`: un manifesto della 0.3.0 si legge con `Anisotropo`, `InPiedi`, `1.0`;
- `le_scelte_fanno_andata_e_ritorno`: `scrivi` poi `leggi` con `Isotropo`, `Seduta`, `2.5`, e nel file la riga `postura = "seduta"`;
- `una_soglia_sotto_mezzo_metro_si_rifiuta`: `0.49`, `-1.0`, `f64::NAN`, `f64::INFINITY` → `ProgettoError::SogliaDelMuro` sia a `scrivi` sia a `leggi` di un manifesto scritto a mano; `0.5` passa;
- `ogni_scelta_cambia_l_impronta`: per ciascuna delle tre scelte, due chiamate a `corsa::esegui` con gli stessi `ingressi` e Progetti che differiscono solo in quella → Impronte diverse. Con `esegui_progetto` il test passerebbe senza codice, perché il manifesto è già un ingresso (`src/corsa.rs:842`);
- `la_persona_in_piedi_e_seduta`: `persona(InPiedi).height == 1.1`, `persona(Seduta).height == 0.75`, `abs_k == 0.70` e `abs_l == 0.97` in entrambe.

- [ ] **Step 2: Verifica, implementa, verifica**

Run: `cargo test -p ombra --test progetto --test corsa --test motore` → FAIL, poi PASS. Le scelte entrano in `Parametri` (`src/corsa.rs`), che è già la stringa `parametri` dell'Impronta.

- [ ] **Step 3: Commit**

```bash
git add src/dominio.rs src/progetto.rs src/corsa.rs src/motore.rs src/da_inx.rs tests/progetto.rs tests/corsa.rs tests/motore.rs tests/cli.rs
git commit -m "feat(progetto): add sky model, posture and wall threshold choices"
```

---

### Task 2: Categorie di stress e nomi nelle due lingue

**Files:**
- Modify: `src/comfort.rs` (`fuori_intervallo` `:64`, `banda` `:87`), `src/lingua.rs` (`Messaggi` `:77`, `ITALIANO` `:116`, `INGLESE` `:166`)
- Test: `tests/comfort.rs`, `tests/lingua.rs`

**Interfaces:**
- Produces:

```rust
// src/comfort.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoriaDiStress { CaldoEstremo, CaldoMoltoForte, CaldoForte, CaldoModerato, Nessuno,
                             FreddoLeggero, FreddoModerato, FreddoForte, FreddoMoltoForte, FreddoEstremo }
pub fn categoria_di_stress(utci_c: f64) -> CategoriaDiStress;        // the thresholds of `banda` today
pub const SOGLIE_DELLE_CATEGORIE: [(CategoriaDiStress, f64); 9];    // the lower bounds, for the Giornale
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimiteSuperato { TemperaturaDellAria, ScartoRadiante, Vento }
pub fn estrapolata(temperatura_c: f64, temperatura_media_radiante_c: f64, vento_ms: f64) -> Option<LimiteSuperato>;
// src/lingua.rs, in Messaggi
pub categorie_di_stress: [&'static str; 10],   // in the order of CategoriaDiStress
pub mesi_brevi: [&'static str; 12],            // "gen".."dic" / "Jan".."Dec"
```

- [ ] **Step 1: Scrivi i test che falliscono**

- `le_soglie_delle_categorie_restano_quelle_di_oggi`: i confini di `banda` (`46, 38, 32, 26, 9, 0, -13, -27, -40`) danno la categoria di sotto, il valore appena sopra quella di sopra;
- `estrapolata_dice_quale_limite`: `(51, 51, 1)` → `TemperaturaDellAria`; `(20, 95, 1)` → `ScartoRadiante`; `(20, 20, 0.4)` → `Vento`; `(20, 20, 1)` → `None`;
- `ogni_categoria_ha_un_nome_in_entrambe_le_lingue`: le dieci stringhe non vuote e diverse fra loro, in `ITALIANO` e `INGLESE`;
- `i_mesi_brevi`: `ITALIANO.mesi_brevi[5] == "giu"`, `INGLESE.mesi_brevi[5] == "Jun"`.

- [ ] **Step 2: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test comfort --test lingua` → FAIL, poi PASS. «Fuori intervallo» resta solo al valore non plausibile dell'inviluppo (`giornale::Inviluppo`): `fuori_intervallo` e `banda` spariscono e i chiamanti passano ai nomi nuovi.

```bash
git add src/comfort.rs src/lingua.rs tests/comfort.rs tests/lingua.rs
git commit -m "refactor(comfort): name stress categories and extrapolation limits"
```

---

### Task 7a: I due meteo sintetici di 48 Ore

Anticipato: i test dei task 3-6 ne hanno bisogno.

**Files:**
- Modify: `strumenti/record/genera.py` (parte `meteo-48-ore`)
- Create: `casi/bastia/meteo-48-ore-estate.epw`, `casi/bastia/meteo-48-ore-inverno.epw`
- Modify: `tests/comune/mod.rs` (nato nel piano B, task 1) → `pub fn caso_con_meteo_sintetico() -> Progetto`, il caso di riferimento letto da `casi/bastia/progetto` con il `meteo` dei due Periodi sostituito dai due file sintetici

**Interfaces:**
- Produces: due EPW validi per `meteo::Epw::leggi`, con l'intestazione di Bastia (lat, lon, fuso, quota del file vero, da `casi/bastia/valori-di-riferimento.toml` o dall'intestazione EPW che Mario riporta), che coprono le date dei due Periodi del caso. Estate: un giorno sereno, una sera coperta con il clearness index sotto 0,95 portato oltre la mezzanotte, un'Ora d'alba con il sole sotto l'orizzonte al centro e la globale sopra zero, globale zero nelle Ore di notte piena. Inverno: due giorni, uno sereno e uno coperto.

- [ ] **Step 1: Scrivi la parte del generatore e il test di lettura**

`i_meteo_sintetici_si_leggono`: i due file si leggono, hanno 48 Ore dall'ora 1 del primo giorno, e nessun valore mancante.

- [ ] **Step 2: Mario genera i due file** (HITL), poi il test passa

Run: `cargo test -p ombra --test meteo` → PASS.

- [ ] **Step 3: Commit**

```bash
git add strumenti/record/genera.py casi/bastia/meteo-48-ore-estate.epw casi/bastia/meteo-48-ore-inverno.epw tests/comune/mod.rs tests/meteo.rs
git commit -m "test(meteo): add two synthetic 48-hour Bastia weather files"
```

---

### Task 4: Come si scrive un'Ora

**Files:**
- Create: `src/ore.rs`
- Modify: `src/lib.rs`
- Test: `tests/ore.rs`

**Interfaces:**
- Consumes: `Messaggi::mesi_brevi` (task 2).
- Produces:

```rust
/// `2026-06-21T12:00+01:00/PT1H`: the start with the offset, then the duration.
pub fn intervallo_iso(inizio: Data, ora: u32, fuso_ore: f64) -> String;
/// The day column and the hour column of a view: ("21 giu", "12–13").
pub fn vista(inizio: Data, ora: u32, m: &Messaggi) -> (String, String);
```

- [ ] **Step 1: Scrivi i test che falliscono**

- `la_prima_ora_del_periodo`: `intervallo_iso(21/6/2026, 0, 1.0) == "2026-06-21T00:00+01:00/PT1H"`;
- `l_ora_dodici`: Ora 12 → `"2026-06-21T12:00+01:00/PT1H"`;
- `l_ora_dopo_mezzanotte_cambia_giorno`: Ora 24 → `"2026-06-22T00:00+01:00/PT1H"`;
- `l_ultima_ora_dell_anno_passa_all_anno_dopo`: 31/12/2026 Ora 23 → `"2026-12-31T23:00+01:00/PT1H"`, Ora 24 → `"2027-01-01T00:00+01:00/PT1H"`;
- `il_ventinove_febbraio`: 28/2/2028 Ora 24 → `"2028-02-29T00:00+01:00/PT1H"`; 28/2/2026 Ora 24 → `"2026-03-01T00:00+01:00/PT1H"`;
- `un_fuso_non_intero`: 5,5 → `+05:30`; 5,75 → `+05:45`; −3,5 → `-03:30`; 0 → `+00:00`;
- `la_vista`: Ora 12 del 21/6 → `("21 giu", "12–13")` in italiano e `("21 Jun", "12–13")` in inglese; Ora 23 → `"23–24"`, nello stesso giorno.

- [ ] **Step 2: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test ore` → FAIL, poi PASS. Il trattino della vista è U+2013. L'anno è quello di `Periodo.inizio`.

```bash
git add src/ore.rs src/lib.rs tests/ore.rs
git commit -m "feat(ore): write an Ora as an ISO 8601 interval and as a view"
```

---

### Task 3: Il record dell'Ora, il valore mancante, la Corsa senza meteo

**Files:**
- Modify: `src/corsa.rs` (`CorsaError` `:49`, `coordinate` `:304`, il ripiego sul fuso dalla longitudine `:320-326`), `src/meteo.rs` (il commento di `:611-613` sull'avvolgimento, se diventa falso)
- Test: `tests/corsa.rs` (il test di `:1094` e i quindici che usano `periodo()` di `:560-571` passano a `caso_con_meteo_sintetico` o a un EPW scritto nel test)

**Interfaces:**
- Consumes: `Epw::ore_dal(inizio, ore) -> Result<Vec<&ValoriOrari>, MeteoError>` (`src/meteo.rs:623`): l'Ora `k` è l'elemento `k` del vettore, che `ore_dal` conta dall'ora 1 del primo giorno; `ore::intervallo_iso` (task 4).
- Produces:
  - `CorsaError::SenzaMeteo(PathBuf)`: il file meteo del Periodo non c'è o non si legge. La Corsa si ferma prima del calcolo; il ripiego sul fuso dalla longitudine sparisce;
  - `CorsaError::MeteoNonAbbinato(String)` per l'errore di `ore_dal`;
  - `CorsaError::MeteoMancante { ora: String, campo: &'static str }`, con `ora` in ISO e `campo` uno di `temperatura`, `umidità`, `pressione`, `globale`, `diretta`, `diffusa`.

- [ ] **Step 1: Leggi il minuto e la quota dell'EPW di Bastia** (HITL)

Il file sta fuori dal repository. Mario lancia `ombra esegui casi/bastia/progetto` e riporta la sezione `[sole]` del Giornale (`minuti_del_file_meteo`) e la riga `LOCATION` del file. Se il minuto non è 0 o 60, **fermati e riporta**.

- [ ] **Step 2: Scrivi i test che falliscono**

- `senza_file_meteo_la_corsa_si_ferma`: un Periodo con `meteo` che non esiste → `CorsaError::SenzaMeteo`, nessun campo calcolato, il Giornale con l'errore;
- `l_ora_k_prende_il_record_k`: `corsa::record_delle_ore(&epw, &periodo)` su un Periodo di 3 Ore e un EPW con temperature 10, 11, 12, 13 alle ore 1-4 → le tre temperature sono 10, 11, 12. `record_delle_ore(epw: &Epw, periodo: &Periodo) -> Result<Vec<&ValoriOrari>, CorsaError>` è la funzione pubblica che fa abbinamento e controllo dei valori, e `esegui` la chiama;
- `un_file_che_comincia_a_meta_giornata_ferma_la_corsa`: `CorsaError::MeteoNonAbbinato`;
- `un_valore_mancante_ferma_la_corsa_e_dice_quale`: temperatura `None` all'Ora 2 → `MeteoMancante { campo: "temperatura", ora }` con l'Ora 2 in ISO, prima di ogni passo; lo stesso per umidità, pressione e ciascuna delle tre radiazioni.

- [ ] **Step 3: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test corsa` → FAIL, poi PASS. L'abbinamento e il controllo dei valori stanno prima di ogni chiamata al Motore. Niente si riempie.

```bash
git add src/corsa.rs src/meteo.rs tests/corsa.rs
git commit -m "feat(corsa)!: pair each Ora with its record and stop without weather"
```

---

### Task 5a: La Corsa per Ora

**Files:**
- Modify: `src/corsa.rs` (`Campi` `:110`, `SerieDiPunto` `:132`, `marcia` `:423`, `esegui` `:628`)
- Test: `tests/corsa.rs`

**Interfaces:**
- Consumes: tutto il piano A e il piano B; i task 1-4.
- Produces:

```rust
// in Campi
pub temperatura_media_radiante: Vec<Raster>,   // one per Ora
pub utci: Vec<Raster>,                         // NaN where the Ora has no wind
// in SerieDiPunto: NaN is "no value", the convention of src/giornale.rs:219-220;
// an Option would not serialise to TOML.
pub temperatura_media_radiante: Vec<f64>,
pub utci: Vec<f64>,
pub k_giu: Vec<f64>, pub k_su: Vec<f64>, pub l_giu: Vec<f64>, pub l_su: Vec<f64>,
pub utci_estrapolata: Vec<(usize, LimiteSuperato)>,
```

- [ ] **Step 1: Scrivi i test che falliscono**

Su `caso_con_meteo_sintetico`:
- `ogni_ora_ha_tmrt_e_utci`: `temperatura_media_radiante.len() == periodo.ore`, ogni raster della forma della Griglia, ogni cella finita;
- `la_utci_e_il_polinomio_sulla_tmrt`: a un Punto, `utci[k] == comfort::utci(ta_k, ur_k, tmrt[k], vento_k)` per ogni Ora;
- `il_vento_mancante_lascia_senza_utci_solo_quell_ora`: vento `None` all'Ora 2 → la Corsa riesce, `tmrt[2]` c'è, `utci[2]` è NaN in ogni cella, le altre Ore hanno la UTCI;
- `le_serie_hanno_i_quattro_flussi`: ai Punti, `k_giu`, `k_su`, `l_giu`, `l_su` hanno `periodo.ore` valori finiti;
- `lo_stato_termico_passa_all_ora_dopo`: le due metà di un Periodo di 48 Ore, corse separatamente con lo stato iniziale, danno all'Ora 24 una Tmrt diversa da quella della Corsa intera (la memoria c'è); la misura di `docs/ricerca/04-stato-termico.md:151` dice di quanto.

- [ ] **Step 2: Verifica che falliscano**

Run: `cargo test -p ombra --test corsa` → FAIL.

- [ ] **Step 3: Implementa `marcia`**

Una volta per Corsa: `motore::cielo` (con `conserva_matrici = modello_di_cielo == Anisotropo`), `motore::muri`, `suolo::proprieta_del_suolo`, `derivazione::{trasmissivita_verso_il_cielo, svfalfa}`, `motore::persona`, `PassoDelloScenario::nuovo` (con `anisotropo` dalla scelta e `costruite` di `src/corsa.rs:762`), `scalari::clearness_della_prima_notte`, `stato_termico::iniziale`. Per ogni Ora: `sole::posizione` al centro, `sole::altezza_massima_del_giorno` (una volta per giorno), `motore::ombre_per_strato`, `derivazione::trasmissivita_lungo_il_raggio`, `scalari::scalari_dell_ora`, `PassoDelloScenario::ora`, `stato_termico::dopo_il_passo`, `comfort::utci_sul_campo` se c'è il vento, le serie ai Punti. L'ombra del passo resta in memoria per l'Ora, per la verifica del task 6. Lo Sky view factor della 0.3.0 resta, preso da `cielo.sky_view_factor`. Il passo gira nel pool a un thread: fuori dal pool il costo per Ora sale di un ordine di grandezza (#24).

- [ ] **Step 4: Verifica e commit**

Run: `cargo test -p ombra --test corsa` → PASS; il cancello di CI; `cargo test -p ombra --release --test corsa -- --ignored il_caso_di_riferimento_sta_sotto_i_sessanta_secondi` sul caso con meteo sintetico, e il tempo nel messaggio del commit.

```bash
git add src/corsa.rs tests/corsa.rs
git commit -m "feat(corsa): compute Tmrt and UTCI for every Ora"
```

---

### Task 5b: Le sei sezioni nuove e la sezione delle Ore

**Files:**
- Modify: `src/corsa.rs` (`esegui` `:628`, le struct delle sezioni accanto a `SoleCitato`)
- Test: `tests/corsa.rs`

**Interfaces:**
- Produces, nel Giornale, ciascuna con le righe di monte citate dove un valore viene da monte:
  - `[ore] intervalli`: un array di `periodo.ore` stringhe `intervallo_iso`; `[periodo] ore` resta il conteggio. Un array al primo livello scritto dopo una sezione finisce dentro quella sezione (`src/giornale.rs:356-364`): per questo è una sezione sua;
  - `[cielo]`: `modello`;
  - `[persona]`: `postura`, `altezza_del_baricentro_m`, `abs_k`, `abs_l`, ciascuno con la sua `riga`;
  - `[suolo]`: per ogni tipo di Superficie presente, la classe di monte, i cinque valori, la riga, il numero di celle; la nota sull'acqua;
  - `[muri]`: i campi di `ConteggioDeiMuri`, la soglia, i cinque valori dei Muri;
  - `[stato_termico]`: lo stato iniziale, il criterio di notte, il clearness index della prima notte con il record da cui viene o `ripiego = 1.0`, `ore_dallo_stato_iniziale` con la definizione della decisione 7 di #21 (riportata nell'annotazione di dispatch, task 5b);
  - `[utci]`: la fonte (Bröde et al. 2012, copia vendorata), la regola del vento («a 10 m, dal file, mai ritoccato»), `SOGLIE_DELLE_CATEGORIE`, le celle-Ora estrapolate per limite.

- [ ] **Step 1: Scrivi i test che falliscono, uno per sezione**

- `il_giornale_scrive_le_ore`: `[ore] intervalli` ha `periodo.ore` elementi, il primo è `intervallo_iso(inizio, 0, fuso)`, ogni serie ai Punti ha la stessa lunghezza;
- `la_sezione_cielo`: `modello = "anisotropo"` col predefinito, `"isotropo"` con la scelta;
- `la_sezione_persona`: le quattro chiavi, ciascuna con una `riga` che contiene `solweig@41840b7`;
- `la_sezione_suolo`: le classi presenti nel caso, i cinque valori, il numero di celle; una nota che contiene `umep-solweig/issues/4`;
- `la_sezione_muri`: soglia 1,0, `lato_del_filtro_celle == 9` a 1 m, `celle_muro > 0`, i cinque valori dei Muri (0,20, 0,90, 0,37, −3,41, 15,0);
- `la_sezione_stato_termico`: `ore_dallo_stato_iniziale` vale 0 sul caso di Bastia e `periodo.ore` su un Periodo di sole di mezzanotte a Tromsø (gli oracoli di #21, decisione 7);
- `la_sezione_utci`: la fonte, la regola del vento, nove soglie.

- [ ] **Step 2: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test corsa` → FAIL, poi PASS.

```bash
git add src/corsa.rs tests/corsa.rs
git commit -m "feat(giornale): record sky, person, ground, walls, thermal state and UTCI"
```

---

### Task 5c: Inviluppi e bandiere

**Files:**
- Modify: `src/giornale.rs` (`Inviluppo` `:192`, `inviluppo` `:224`), `src/corsa.rs` (`inviluppi` `:549`)
- Test: `tests/corsa.rs`, `tests/giornale.rs` se esiste, altrimenti `tests/corsa.rs`

**Interfaces:**
- Produces:

```rust
// in Inviluppo, all `skip_serializing_if = "Option::is_none"`
pub ora_del_minimo: Option<String>, pub cella_del_minimo: Option<(usize, usize)>,
pub ora_del_massimo: Option<String>, pub cella_del_massimo: Option<(usize, usize)>,

/// The envelope over every cell and every Ora, with the range decided Ora by Ora
/// (Tmrt − Ta of that Ora) and a NaN raising the flag where `nan_e_difetto(ora, cella)`.
pub fn inviluppo_per_ora(
    campo: &str, unita: &str, ore: &[String], valori: &[Raster],
    intervallo_dell_ora: impl Fn(usize) -> (f64, f64),
    nan_e_difetto: impl Fn(usize) -> bool, nota: &str,
) -> Inviluppo;
```

- [ ] **Step 1: Scrivi i test che falliscono**

- `gli_inviluppi_di_tmrt_e_utci_hanno_ora_e_cella`: `ora_del_minimo` è una delle stringhe di `[ore] intervalli`, `cella_del_minimo` sta nella Griglia;
- `la_bandiera_della_tmrt_si_alza_fuori_intervallo`: `inviluppo_per_ora` con una cella a `Ta + 71` nell'Ora 1 → `fuori_intervallo == true`; con un NaN e `nan_e_difetto = |_| true` → `true`;
- `la_utci_senza_vento_non_alza_la_bandiera`: con il vento mancante a un'Ora, l'inviluppo della UTCI ha `frazione_senza_dato > 0` e `fuori_intervallo == false`;
- `i_flussi_sulle_serie_ai_punti`: gli inviluppi di K↓, K↑ (0-1410 W/m²) e L↓, L↑ (100-900 W/m²) esistono, senza Ora né cella.

- [ ] **Step 2: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test corsa` → FAIL, poi PASS.

```bash
git add src/giornale.rs src/corsa.rs tests/corsa.rs
git commit -m "feat(giornale): envelope Tmrt and UTCI with the Ora and cell of each extreme"
```

---

### Task 6: Le tre verifiche nuove

**Files:**
- Modify: `src/corsa.rs` (accanto a `VerificaOmbra`), `src/bin/ombra.rs` (`riassunto` `:260-265` conta tutte le verifiche, non solo `verifica_ombra`)
- Test: `tests/corsa.rs`, `tests/cli.rs`

**Interfaces:**
- Produces, ciascuna nella sua sezione del Giornale con `bandiera: bool` e `nota`:

```rust
struct VerificaDueOmbre { ore_verificate: usize, ore_sole_basso: usize, celle_ore_diverse: usize, frazione: f64, soglia: f64, bandiera: bool, nota: &'static str }
struct VerificaNotteBuia { ore_verificate: usize, celle_non_buie: usize, bandiera: bool, nota: &'static str }
struct VerificaCieloAperto { ore_verificate: usize, scarto_massimo_wm2: f64, ora: Option<String>, cella: Option<(usize, usize)>, soglia_wm2: f64, bandiera: bool, nota: &'static str }
```

- [ ] **Step 1: Scrivi i test che falliscono**

- `le_due_ombre_si_contano_sopra_tre_gradi`: le Ore con il sole sopra 3° contano le celle-Ora con `|illuminata_del_passo − illuminata| > 0.5`; le Ore fra 0° e 3° vanno in `ore_sole_basso`. Il passo usa 3° (`pipeline.rs:671`), `ombre` 2° (`src/motore.rs:41`): la nota lo dice;
- `la_notte_piena_e_buia`: sul meteo sintetico, che ha globale zero di notte, `celle_non_buie == 0`. Gli EPW scritti nei test di oggi hanno globale 120 a mezzanotte (`tests/corsa.rs:753-772`, `tests/cli.rs:50-68`): i test che passano per questa verifica li correggono;
- `il_cielo_aperto_torna_col_file`: nella cella illuminata con lo Sky view factor più alto, K↓ contro `DNI · sin h + DHI`; il Giornale ha lo scarto massimo, la sua Ora e la sua cella;
- `una_verifica_mai_eseguita_alza_la_bandiera`: un Periodo di sole Ore notturne (Bastia, dicembre, 20-24) → le due verifiche diurne hanno `ore_verificate == 0` e `bandiera == true`; un Periodo di sole Ore diurne (Tromsø, 21 giugno) → la notte buia ha `bandiera == true`;
- `interroga_conta_tutte_le_verifiche`: in `tests/cli.rs`, il riassunto conta le quattro bandiere.

- [ ] **Step 2: Misura le soglie su Bastia**

Corri il caso con meteo sintetico con le verifiche senza soglia e scrivi la frazione delle due ombre e lo scarto del cielo aperto. La soglia si fissa poco sopra, con la misura nel commento della costante e nella `nota`. Quando Mario ha il file vero, rilancia e scrive accanto anche quella misura.

- [ ] **Step 3: Verifica, implementa, verifica, commit**

Run: `cargo test -p ombra --test corsa --test cli` → FAIL, poi PASS.

```bash
git add src/corsa.rs src/bin/ombra.rs tests/corsa.rs tests/cli.rs
git commit -m "feat(corsa): check the two shadows, the dark night and the open sky"
```

---

### Task 7b: Gli ingressi per monte

**Files:**
- Create: `tests/bastia_registrata.rs`, con un test `#[ignore]` `esporta_gli_ingressi_per_monte`
- Modify: `tests/comune/mod.rs` (il lettore di campi a tre dimensioni, se il piano B non lo ha già)

**Interfaces:**
- Produces: in `tests/record/ingressi/`, per lo Scenario `com'è` del caso di Bastia e per la sua variante uniforme, i raster che la wheel deve ricevere identici: modello di superficie, chiome e zona tronco (Chiome riunite), classi di monte, `costruite`, nel formato `.toml` + `.f32` del piano B, task 1. La wheel ricava da sé Muri, Sky view factor e il resto, così il confronto copre anche quelle parti.

- [ ] **Step 1: Scrivi il test che esporta, lancialo, verifica i file**

Run: `cargo test -p ombra --test bastia_registrata -- --ignored esporta_gli_ingressi_per_monte` → i file esistono, lo `sha256` torna.

- [ ] **Step 2: Commit**

```bash
git add tests/bastia_registrata.rs tests/comune/mod.rs tests/record/ingressi
git commit -m "test(record): export Bastia inputs for the upstream records"
```

---

### Task 7c: Le parti del generatore per Bastia

**Files:**
- Modify: `strumenti/record/genera.py` (parte `bastia`)

- [ ] **Step 1: Scrivi la parte**

Legge gli ingressi del task 7b e i due meteo del task 7a, e fa girare la wheel per tre Corse: *uniforme* d'estate (uno Strato di chioma, trasmissività costante uguale allo scalare di `solweig` per la data), in piedi e seduta; *com'è* d'estate e *com'è* d'inverno, con il `psi` che `solweig` calcola per la data, in piedi. Record: il campo Tmrt intero per Ora (tre dimensioni) e la UTCI ai Punti.

- [ ] **Step 2: Mario lancia `genera.py bastia`** (HITL)

- [ ] **Step 3: Commit**

```bash
git add strumenti/record/genera.py tests/record
git commit -m "test(record): generate the Bastia runs with upstream"
```

---

### Task 7d: Le Corse registrate

**Files:**
- Modify: `tests/bastia_registrata.rs`

- [ ] **Step 1: Scrivi i test**

- `bastia_uniforme_riproduce_monte_entro_un_millesimo`: in piedi e seduta, ogni cella e ogni Ora entro 1e-3 °C dal record;
- `bastia_com_e_d_estate_sta_entro_la_soglia_misurata` e `bastia_com_e_d_inverno_sta_entro_la_soglia_misurata`: scarto massimo e 95° percentile entro le costanti dello step 2. L'inverno, con due Strati, è il caso in cui la Trasmissività per cella di ADR 0004 si scosta dallo scalare di monte: lo scarto misura quella scelta, non un difetto.

- [ ] **Step 2: Misura e fissa le soglie**

Alla prima esecuzione scrivi scarto massimo e 95° percentile per stagione; le costanti stanno poco sopra, con la misura nel commento. Nella stessa misura: quanto la classe 2 sotto i tetti sposta la Tmrt al suolo (Corsa con e senza la classe 2, differenza al suolo fuori dagli Edifici), e quanto pesa l'unica Trasmissività verso il cielo (lo scarto d'inverno del caso com'è contro uniforme). Le misure vanno nel commento del test e nella nota del Giornale che le riguarda.

- [ ] **Step 3: Verifica e commit**

Run: `cargo test -p ombra --test bastia_registrata --test record` → PASS.

```bash
git add tests/bastia_registrata.rs
git commit -m "test(corsa): register the Bastia runs against upstream"
```

---

### Task 8: Le viste, il tempo e la versione

**Files:**
- Modify: `src/bin/ombra.rs` (`interroga` `:219`, `riassunto` `:230`), `src/lingua.rs`, `Cargo.toml` (`version`), `tests/cli.rs`, `tests/corsa.rs` (il cancello dei 60 secondi `:1254`)

**Interfaces:**
- Consumes: `ore::vista`, `Messaggi::{categorie_di_stress, mesi_brevi}`.

- [ ] **Step 1: Scrivi i test che falliscono, in `tests/cli.rs`**

Sul Progetto sintetico che `tests/cli.rs` già costruisce:
- `interroga_mostra_le_serie_in_tabella`: per un Punto, una riga per Ora con le colonne giorno, Ora, Tmrt, UTCI, Categoria di stress; la prima riga ha il giorno di inizio e `0–1`; l'intestazione dichiara «ora solare, UTC+1»;
- `interroga_ha_una_vista_tabellare_per_ogni_campo`: per Tmrt e UTCI, minimo, massimo e media con l'Ora e la cella degli estremi scritte come vista;
- `interroga_in_inglese`: con `--lingua en` le Categorie e i mesi sono in inglese, i nomi dei comandi restano.

- [ ] **Step 2: Verifica, implementa, verifica**

Run: `cargo test -p ombra --test cli` → FAIL, poi PASS.

- [ ] **Step 3: La versione sale**

`Cargo.toml` passa a `0.5.0`: cambia il contenuto del Giornale (decisione di Mario, 2026-11-23).

- [ ] **Step 4: Il tempo**

Run: `cargo test -p ombra --release --test corsa -- --ignored il_caso_di_riferimento_sta_sotto_i_sessanta_secondi` → PASS sul caso con meteo sintetico. Misura anche il caso al tetto di progetto (200 × 200 m a 2 m, 24 Ore, `anisotropo`) con un Progetto sintetico in un test `#[ignore]`, e scrivi i due tempi nel messaggio del commit. Quando Mario ha il file vero, rilancia il cancello anche su quello. Se il tetto supera 10 minuti, **fermati e riporta**.

- [ ] **Step 5: Commit**

```bash
git add src/bin/ombra.rs src/lingua.rs Cargo.toml Cargo.lock tests/cli.rs tests/corsa.rs
git commit -m "feat(cli)!: show Tmrt and UTCI tables and release 0.5.0"
```
