# La catena radiativa fino a Tmrt e UTCI — indice dei piani

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement each plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [`docs/superpowers/specs/2026-10-29-catena-radiativa.md`](../specs/2026-10-29-catena-radiativa.md). Ogni piano ragiona dalla spec: chi esegue legge la spec, questo indice e il proprio piano.

La spec copre tre strati, che sono anche i suoi tre seam di test. Ognuno è un piano a sé, con un branch e una PR, e si verifica senza il successivo:

| Piano | Seam della spec | Cosa consegna | Branch |
|---|---|---|---|
| [A — Il passo vendorato](2026-11-22-catena-radiativa-a-passo.md) | 2. Il passo del Motore | la copia vendorata estesa al passo fuso con le due Trasmissività, `motore::PassoDelloScenario`, il banco di parità | `feat/passo-vendorato` |
| [B — Lo strato portato](2026-11-22-catena-radiativa-b-strato.md) | 3. Lo strato portato | sole SPA, scalari dell'Ora, suolo, Muri, Trasmissività, Stato termico, con i record di monte | `feat/strato-portato` |
| [C — La Corsa fino alla UTCI](2026-11-22-catena-radiativa-c-corsa.md) | 1. La Corsa | le tre scelte del Progetto, la Corsa per Ora, il Giornale nuovo, le verifiche, le Corse registrate su Bastia, le viste, la 0.4.0 | `feat/corsa-tmrt-utci` |

**Ordine.** A, poi B, poi C. B usa i tipi che A espone (`motore::{PassataDelCielo, Muri, ProprietaDelSuolo, StatoTermico, WeatherScalars}`); C usa tutto. Il percorso critico e il parallelismo vero li dà la [revisione trasversale](2026-11-22-catena-radiativa-revisione.md), §4; l'ordine dentro ogni piano lo dà la sua annotazione di dispatch. In sintesi:

- **subito, in parallelo ad A1**: la misura del banco su `ubuntu-latest` (A, task 5 step 1) e il generatore dei record (B, task 1), in worktree separati;
- **in parallelo ad A2-A3**: il sole SPA (B, task 2), chiuso prima di A4, perché tutti e due toccano `tests/motore.rs`;
- **dopo A4**: B3-B6; poi C, dove i task 2 e 4 possono girare insieme a B perché toccano solo `src/comfort.rs`, `src/lingua.rs` e `src/ore.rs`;
- **passaggi di Mario**: `genera.py tutto` dopo B1; i due meteo sintetici (C7a) e le Corse di Bastia (C7c); la lettura dell'EPW vero (C3); il sì al push del branch di misura del banco.

**Le decisioni di Mario del 2026-11-23**, già applicate ai piani e alla spec: all'alba si tengono le misure del file, come monte; una Corsa senza file meteo si ferma; «Bastia com'è» si registra d'estate e d'inverno; la versione sale a 0.4.0 con il sole e a 0.5.0 con la catena.

**Prima di cominciare** (fuori da questi piani, da Mario):

- la PR [#53](https://github.com/maeurong/OMBRA/pull/53), con la voce Postura di `CONTEXT.md`, è unita: il piano C nomina `Postura` nel codice;
- la PR [#62](https://github.com/maeurong/OMBRA/pull/62), con `docs/ricerca/05-crate-rlib.md`, è unita: il task 5 di A la cita;
- la ricerca `04-strato-python-monte.md` del ticket [#54](https://github.com/maeurong/OMBRA/issues/54), oggi sul branch `research/strato-python-monte`, è unita: il piano B cita le sue righe per decidere dove `umep-solweig` coincide e dove diverge. Il prefisso `04-` è già preso da `04-stato-termico.md`: all'unione uno dei due si rinumera.

## Global Constraints

Valgono per ogni task dei tre piani, che li danno per letti.

- **Velocità**: il caso di riferimento completo (50 × 50 celle a 1 m, 48 ore, due Scenari per due Periodi) sotto i **60 secondi su sola CPU**, in release. Tetto di progetto 200 × 200 m a 2 m per 24 ore in 10 minuti.
- **Binario singolo, nessuna dipendenza nativa**: niente Python, niente librerie C nel binario e nella CI di `ombra`. Prima di aggiungere un crate: `cargo tree -e normal | grep -iE "sys$|cc |bindgen"` vuoto. Python serve solo al generatore dei record (piano B) e al banco di parità (piano A), mai a `cargo test -p ombra`.
- **`vendor/solweig/src` è prodotto di `estrai.py`**, `lib.rs` compreso: si cambia lo script, mai il file. Ogni regola dichiara un conteggio in `PROVENIENZA.toml` e lo script si ferma al primo che non torna. `cargo fmt -p ombra`, mai `--all`.
- **Pin di monte**: `UMEP-dev/solweig@41840b74cd7c003f88dbde0c30fff736cb51bd5d` (v0.1.0b101), letto da `vendor/solweig/PROVENIENZA.toml`, mai riscritto a mano in un test.
- **Ogni costante e formula portata cita la riga di `solweig` al pin** in un commento (`// solweig@41840b7 pysrc/solweig/physics/wallalgorithms.py:10`). Dove `umep-solweig` coincide, la sua riga sta accanto; dove diverge, una riga dice in cosa.
- **Vocabolario di `CONTEXT.md`** nei nomi di tipi, funzioni, campi del Giornale e messaggi: Ora, Corsa, Giornale, Muro, Stato termico, Temperatura media radiante, Flusso radiativo, UTCI, Categoria di stress, Modello di cielo, Postura, Trasmissività lungo il raggio, Trasmissività verso il cielo, Chiome riunite, Strato di chioma. «Sky view factor» resta il nome dei due valori pubblicati e di nient'altro.
- **Le righe dei raster corrono da nord**: riga 0 è la più settentrionale, come in tutto il progetto.
- **Stringhe**: ogni messaggio per l'utente sta in `src/lingua.rs`, nelle due istanze `ITALIANO` e `INGLESE`. I nomi dei comandi non si traducono.
- **Codice, commenti e messaggi di commit in inglese**; documenti e file di dati in italiano.
- **Un commento non promette codice che non esiste**: un vincolo che verrà va al futuro e dice che verrà.
- **`git add` con percorsi espliciti**, mai `-A` o `.`.
- **Test prima dell'implementazione**, che fallisce per il motivo giusto prima della riga di produzione. Una tolleranza si allenta solo con la misura scritta accanto.
- **Nessun identificativo inventato**: classi di monte, righe, codici di specie vengono da una fonte citabile.
- **Comandi**: `export PATH=$HOME/.cargo/bin:$PATH` prima di `cargo`. Il cancello di CI è `cargo fmt -p ombra --check`, `cargo clippy -p ombra --all-targets --all-features -- -D warnings`, `cargo test -p ombra --all-features` (`.github/workflows/ci.yml:32-43`).
