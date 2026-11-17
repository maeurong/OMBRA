# 05 — Il banco di parità e il crate di monte in `rlib`

Ticket: maeurong/OMBRA#61, «Il banco di parità e il crate di monte in rlib», sub-issue della
mappa #29. Che la parità si faccia contro un clone di monte usato come `rlib`, e non contro la
wheel Python, è già deciso e qui non si riapre.

**Fonti.** Codice di monte `UMEP-dev/solweig` al commit pinnato
`41840b74cd7c003f88dbde0c30fff736cb51bd5d` (tag `v0.1.0b101`, `vendor/solweig/PROVENIENZA.toml:9`),
letto in un clone fuori dal repository (`git -C <clone> log -1` → `41840b7… (HEAD, tag: v0.1.0b101)`,
`git status --short` vuoto anche a fine prova). Ogni `percorso:riga` di monte senza altra
indicazione è relativo a quel commit. PyO3 alla versione che monte risolve nel suo `Cargo.lock`,
`0.24.2`, letto nei sorgenti scaricati da cargo (`~/.cargo/registry/src/index.crates.io-*/pyo3-*-0.24.2/`).
OMBRA: worktree partito da `main` a `b510c2f`.

Tag: `[V]` letto nella fonte · `[M]` misurato con comando · `[INF]` inferenza · `[NON TROVATO]`.

**Banco di prova.** macOS 27.2.1 su arm64, `ld-27037.1`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)` e
`cargo 1.99.1` di Homebrew. Python di Homebrew: `pyo3-build-config` ha trovato da sé
`/opt/homebrew/opt/python@3.14/bin/python3.14`, con la libreria condivisa nel framework. Linux
non l'ho potuto provare: niente Docker né Podman su questa macchina (§4).

**Il binario di prova.** Un crate `probe` con un solo test d'integrazione,
`fonti/05-crate-rlib-passo.rs`. Il test costruisce una Griglia 12 × 12 a 1 m con un blocco di
6 m, cielo isotropo, niente vegetazione né muri, nessun `gvf_cache`. Chiama una volta
`compute_timestep_rust` con `OUT_ALL` e scrive in `$PROBE_OUT` i dodici raster d'uscita più
`timeadd`, come `f32` little-endian: 6916 byte. Il manifesto del crate di prova è questo, con la
variante del percorso indicata in ciascuna via:

```toml
[dependencies]
rustalgos = { path = "<clone>/rust", default-features = false }
ndarray = "0.16.1"

[workspace]
```

Il `Cargo.lock` del crate di prova parte da una copia di `rust/Cargo.lock` di monte. A risoluzione
fatta contiene 37 pacchetti: tutti, tranne `probe`, alle stesse versioni del lock di monte
(`comm -23` fra i due elenchi nome-versione restituisce solo `"probe" "0.0.0"`). Con
`default-features = false`, `wgpu` non entra nel lock; `pyo3` e `numpy` sì. `[M]`

---

## Premesse del brief, verificate

- `rust/Cargo.toml:2` `name = "rustalgos"`; `:8` `crate-type = ["cdylib", "rlib"]`; `:11`
  `pyo3 = { version = "0.24.2", features = ["extension-module", "abi3-py311"] }`; `:26`
  `default = ["gpu"]`; `:29-33` `[profile.release]` con `lto = "fat"`, `codegen-units = 1`,
  `panic = "abort"`, `strip = "symbols"`. `[V]`
- `rust/src/api.rs:17-28` riesporta `compute_timestep_rust`, `TimestepInputs`, le struct d'ingresso
  e i tre scalari. `rust/src/pipeline_scalars.rs:8` importa `pyo3::prelude::*`, e `:11`, `:100`,
  `:130` marcano `#[pyclass]` i tre scalari. A questi si aggiunge `rust/src/lib.rs:30`, il
  `#[pymodule]` del crate. Quindi il crate in `rlib` porta pyo3 comunque. `[V]`
- `vendor/solweig/Cargo.toml:13-20` ha solo `ndarray 0.16.1` e `rayon`: la copia di OMBRA non
  dipende da pyo3. `[V]`
- **Precisazione marginale.** Il brief chiede `export PATH=$HOME/.cargo/bin:$PATH`. Su questa
  macchina `~/.cargo/bin` non esiste, e `cargo` e `rustc` vengono da Homebrew
  (`/opt/homebrew/Cellar/rust/1.99.1`). Non cambia nulla della prova. `[M]`
- **Precisazione al ticket.** Il ticket riporta l'avviso di monte: «a binary that links this crate
  directly will not find the Python symbols» (`fonti/umep-solweig-issue-14.md:121`). Sul macOS di
  prova il primo collegamento che fallisce però non è quello del binario di test: è quello della
  `cdylib` di monte, che cargo costruisce anche quando il crate entra come dipendenza (§1). Il
  binario di test, nel profilo `test`, non contiene alcun riferimento a simboli di Python (§2).
  `[M]`

---

## 1. Il binario di test si collega? No, su macOS

Comando, dal crate di prova che punta al clone non toccato:

```
cargo test --manifest-path <scratch>/r61/probe-orig/Cargo.toml
```

Esito: errore di collegamento, con 91 simboli non risolti, tutti dell'API C di Python
(`_PyExc_AttributeError`, `_Py_InitializeEx`, `__Py_NoneStruct`, ...). Le righe decisive:

```
   Compiling rustalgos v0.1.0 (<clone>/rust)
error: linking with `cc` failed: exit status: 1
  = note:  "cc" ... "-o" ".../target/debug/deps/librustalgos.dylib" "-Wl,-dead_strip" "-dynamiclib" "-nodefaultlibs"
          Undefined symbols for architecture arm64:
          ld: symbol(s) not found for architecture arm64
error: could not compile `rustalgos` (lib) due to 1 previous error
```

L'uscita è `librustalgos.dylib` con `-dynamiclib`: fallisce la `cdylib` di monte, prima ancora che
cargo arrivi al binario di test. `[M]` Il motivo è scritto nella documentazione di PyO3. La feature
`extension-module` spegne il collegamento a `libpython` sui target Unix
(`fonti/pyo3-0-24-2-building-and-distribution.md:175`). Su macOS una libreria dinamica con simboli
non risolti richiede allora argomenti di linker in più, che maturin passa da sé e una build fatta a
mano no (`:122`). Lo stesso testo avverte che su macOS `extension-module` fa fallire `cargo test`
(`:171`). Il sorgente lo conferma: `pyo3-ffi` emette `cargo:rustc-link-lib` solo se
`is_linking_libpython()` è vero (`pyo3-ffi-0.24.2/build.rs:205-206`), e per macOS e Linux questo
equivale a `!is_extension_module()` (`pyo3-build-config-0.24.2/src/impl_.rs:877-884`). `[V]`

Che cargo costruisca tutti i `crate-type` di una dipendenza di percorso, `cdylib` compresa, l'ho
misurato e non l'ho trovato scritto nella documentazione di cargo. `[NON TROVATO]`

---

## 2. Le vie che lo fanno collegare senza toccare il codice di monte

Tre profili per ogni via. Nessuna delle vie tocca un file `.rs` di monte.

- **`test`**: il profilo di `cargo test`, senza ottimizzazioni.
- **`release`**: `cargo test --release` con il `[profile.release]` predefinito.
- **`release` come monte**: `cargo test --release` con lo stesso `[profile.release]` di
  `rust/Cargo.toml:29-33`, copiato nel manifesto del crate di prova. Va copiato perché cargo ignora
  i profili dichiarati nelle dipendenze (`fonti/cargo-profiles.md:15`). E `panic = "abort"` non
  vale per i test, che richiedono `unwind` (`:124`). `[V]`

| Via | Righe | Python in CI | `test` | `release` | `release` come monte |
|---|---|---|---|---|---|
| A1. patch al clone: `crate-type = ["rlib"]` | 1 (manifesto del clone) | no | collega e passa | collega e passa | **non collega** |
| A2. patch al clone: via `extension-module` da `:11` | 1 (manifesto del clone) | sì, con libreria condivisa, a build e a esecuzione | collega e passa | collega e passa | collega e passa |
| B1. `RUSTFLAGS="-C link-arg=-undefined -C link-arg=dynamic_lookup"` | 0, una variabile d'ambiente | no | collega e passa | collega e passa | **collega, poi abortisce** |
| B2. `PYO3_CONFIG_FILE` con `extra_build_script_line` | 0 nei manifesti; un file di 12 righe | sì, con libreria condivisa, a build e a esecuzione | collega e passa | collega e passa | collega e passa |
| B3. `pyo3` con `auto-initialize` nel crate di prova, più `PYO3_PYTHON` | 1 (manifesto della prova) | sì | **non collega** | — | — |
| C. `[patch]` o unificazione delle feature dal lato di OMBRA | — | — | non praticabile (sotto) | — | — |

Tutte le celle sono `[M]` su questa macchina, tranne la riga C.

**A1, `crate-type = ["rlib"]`.** Con un solo `sed` sul `Cargo.toml` del clone, cargo non
costruisce più la `cdylib`. Il binario di test si collega senza Python: `otool -L` mostra solo
`libiconv` e `libSystem`, e `nm -u` non elenca simboli `_Py*`. Funziona anche con
`PYO3_NO_PYTHON=1`, cioè senza nessun interprete a build. `[M]` Nel profilo «come monte», con LTO
`fat` e una sola unità di codice, il collegamento del binario di test fallisce con 87 simboli
`_Py*` non risolti (`"_PyBaseObject_Type", referenced from:` è il primo). `[M]` Il perché lo
deduco senza averlo verificato. Con LTO il codice di pyo3 resta raggiungibile da qualche simbolo
esportato, probabilmente il `#[pymodule]` di `lib.rs:30`. Senza LTO il linker scarta gli oggetti
che lo contengono. `[INF]`

**A2, via `extension-module`.** Togliere `"extension-module"` da `rust/Cargo.toml:11`
(`features = ["abi3-py311"]`) rimette il collegamento a `libpython`. Il build script di `pyo3-ffi`
emette `cargo:rustc-link-lib=python3.14` con il percorso del framework di Homebrew, e sia
`librustalgos.dylib` sia il binario di test caricano
`/opt/homebrew/opt/python@3.14/Frameworks/Python.framework/Versions/3.14/Python` (`otool -L`).
Serve quindi una Python con libreria condivisa alla compilazione e anche all'esecuzione del test. È
l'unica via con la patch al clone che regge tutti e tre i profili. `[M]` È anche la prima delle
soluzioni che la FAQ di PyO3 propone per `cargo test` (`fonti/pyo3-0-24-2-faq.md:18-29`). `[V]`

**B1, `-undefined dynamic_lookup`.** È l'argomento che la documentazione di PyO3 indica per macOS
(`fonti/pyo3-0-24-2-building-and-distribution.md:122-147`). Passato con `RUSTFLAGS`, vale per ogni
collegamento: la `cdylib` di monte si collega, e il binario di test pure. Nei profili `test` e
`release` il binario non contiene riferimenti a Python, e il test passa. Nel profilo «come monte»
il binario si collega con 87 riferimenti `_Py*` lasciati al caricatore, che all'avvio si ferma:

```
dyld[29471]: symbol not found in flat namespace '_PyBaseObject_Type'
  process didn't exit successfully: `.../passo-2a3bcbfb4072904d` (signal: 6, SIGABRT: process abort signal)
```

`[M]` Funziona solo su macOS. Il flag è del linker Apple, e su Linux il problema si presenta in
un'altra forma (§4). La stessa coppia di flag si può scrivere in `.cargo/config.toml` alla voce
`[target.aarch64-apple-darwin]` (`:132-147`). `[V]` Cargo però legge quel file in base alla
cartella da cui parte il comando: se sta nella radice di OMBRA, i flag arrivano anche al binario
distribuito. Per non toccarlo, la variabile va limitata al passo del banco. `[INF]`

**B2, `PYO3_CONFIG_FILE`.** È l'unica via provata che non tocca nessun manifesto e regge tutti e
tre i profili. Il file di configurazione di PyO3 accetta righe `extra_build_script_line`, che
`pyo3-ffi` stampa anche con `extension-module` acceso (`pyo3-ffi-0.24.2/build.rs:214-215`, dopo il
blocco condizionato di `:205-206`). `[V]` Con queste due righe:

```
extra_build_script_line=cargo:rustc-link-lib=python3.14
extra_build_script_line=cargo:rustc-link-search=native=/opt/homebrew/opt/python@3.14/Frameworks/Python.framework/Versions/3.14/lib
```

il collegamento a `libpython` arriva alla `cdylib` di monte e al binario di test, e il test passa
in tutti e tre i profili. `[M]` Il file intero sono 12 righe, con percorsi che valgono solo per
quella macchina. In CI andrebbe generato al momento con `PYO3_PRINT_CONFIG=1`, la via indicata
dalla documentazione (`fonti/pyo3-0-24-2-building-and-distribution.md:21-53`). `[V]` Che la
generazione funzioni su un runner Linux non l'ho provato. `[INF]`

**B3, `auto-initialize` e `PYO3_PYTHON`.** Ho aggiunto
`pyo3 = { version = "0.24.2", features = ["auto-initialize"] }` al crate di prova, con
`PYO3_PYTHON` puntato alla 3.14. Esito: lo stesso errore di §1 su `librustalgos.dylib`, e nessuna
riga `cargo:rustc-link-lib=python` nell'uscita del build script di `pyo3-ffi`. `[M]` Il motivo è
l'unificazione. Le feature di una dipendenza condivisa sono l'unione di quelle chieste da tutti
(`fonti/cargo-features.md:182-186`), quindi `extension-module` resta acceso, e con lui il ramo che
non collega. `PYO3_PYTHON` sceglie l'interprete (`fonti/pyo3-0-24-2-building-and-distribution.md:17`),
non il collegamento. `[V]`

**C, `[patch]` e unificazione dal lato di OMBRA.** Nessuna feature si può spegnere dall'esterno:
l'unificazione aggiunge e non toglie (`fonti/cargo-features.md:182-186`). `[V]` `[patch]` sostituisce
un'intera sorgente con un'altra copia del crate, e si dichiara solo al livello più alto
(`fonti/cargo-overriding-dependencies.md:116-120`). `[V]` Ci sono due bersagli possibili, e
nessuno dei due conviene. Sostituire `rustalgos` con una copia dal manifesto corretto equivale ad
A1 o A2, con un passaggio in più. Sostituire `pyo3` da crates.io con un fork in cui
`extension-module` non fa nulla toccherebbe un crate di terzi, e da mantenere. Non provato.
`[INF]`

**Il binario distribuito di OMBRA.** Nessuna via lo tocca, se il banco vive in un crate a sé con il
proprio `[workspace]`, come già fa `vendor/solweig/Cargo.toml:27-30`. Se invece il clone entrasse
come `dev-dependency` di OMBRA, le sue feature non si unificherebbero con quelle del binario,
tranne quando si compilano i test (`fonti/cargo-resolver.md:282-289`). `[V]` Il `Cargo.lock` di
OMBRA però si allungherebbe di `pyo3`, `numpy` e delle loro dipendenze. `[INF]`

---

## 3. Una via cambia i risultati? No, su questo ingresso

Stesso ingresso sintetico, stesso compilatore, uscita confrontata con `shasum -a 256` sui 6916 byte.

| Via | `test` | `release` | `release` come monte |
|---|---|---|---|
| A1 `rlib` | `3fe58ade…4f310b` | `3fe58ade…4f310b` | non collega |
| A1 `rlib`, `PYO3_NO_PYTHON=1` | `3fe58ade…4f310b` | — | — |
| A2 senza `extension-module` | `3fe58ade…4f310b` | `3fe58ade…4f310b` | `3fe58ade…4f310b` |
| B1 `dynamic_lookup` | `3fe58ade…4f310b` | `3fe58ade…4f310b` | abortisce all'avvio |
| B2 `PYO3_CONFIG_FILE` | `3fe58ade…4f310b` | `3fe58ade…4f310b` | `3fe58ade…4f310b` |

Impronta intera: `3fe58ade95b498023e10ad4e8e1c05ab7dbb8cf674d55f7bd58c4814e64f310b`. Undici uscite
su undici sono identiche byte per byte, fra vie e fra profili. Tre ripetizioni di A1 in `release`
danno la stessa impronta, con rayon su 6 core. `[M]` L'uscita non è banale: Tmrt va da 30,59 a
53,93 °C, e 16 celle su 144 sono in ombra. `[M]`

Comando tipo, uno per via e profilo, ciascuno con la propria cartella `CARGO_TARGET_DIR`:

```
PROBE_OUT=<scratch>/r61/out-noext-up.bin CARGO_TARGET_DIR=<scratch>/r61/t-noext-up \
  cargo test --release --manifest-path <scratch>/r61/probe-noext-up/Cargo.toml --test passo
```

Che la via non cambi i numeri ha una spiegazione semplice. Le vie toccano solo come si collega
pyo3, e `compute_timestep_rust` non passa da pyo3: valida e chiama `timestep_physics`
(`rust/src/pipeline.rs:571-576`). `[V]` La misura lo conferma su un solo ingresso, non lo dimostra
in generale. `[INF]`

---

## 4. Linux

**Non provato.** Su questa macchina non c'è un ambiente Linux. Conta perché la CI di OMBRA gira su
`ubuntu-latest` (`.github/workflows/ci.yml:18`, `.github/workflows/vendor-check.yml:36`). `[V]`

Cosa dice la documentazione di PyO3 0.24.2:

- `extension-module` spegne il collegamento a `libpython` **su tutti i target Unix**, Linux
  compreso, e per questo «binaries, tests, and examples ... will fail to build»
  (`fonti/pyo3-0-24-2-building-and-distribution.md:175-179`). `[V]`
- Gli argomenti di linker in più li chiede **solo per macOS**
  (`fonti/pyo3-0-24-2-building-and-distribution.md:120-147`). Su Linux la documentazione non
  prescrive nulla per la `cdylib`. `[V]`
- La FAQ cita l'errore nella forma del linker GNU, «Undefined reference to _PyExc_SystemError»,
  come sintomo di `cargo test` con `extension-module` acceso (`fonti/pyo3-0-24-2-faq.md:18-20`). `[V]`

Ne deduco, senza averlo provato, due differenze con macOS. Su Linux la `cdylib` di monte si
collega senza argomenti extra, perché il linker accetta simboli non risolti in una libreria
condivisa. Il binario di test invece fallisce, se gli resta codice che chiama Python. Che nel
profilo `test` il linker di Linux scarti quel codice come fa `ld` di Apple con `-dead_strip`
non lo so. `[INF]` A2 e B2 non dipendono da questo dettaglio, perché collegano `libpython`. Serve
però una Python con libreria condivisa sul runner, e su `ubuntu-latest` non l'ho verificato. Il
runner ha `python3`, che `vendor-check.yml:54` già usa. `[INF]`

**Stato di PyO3 dopo il pin.** Da PyO3 0.26.0 (2025-08-29) la variabile `PYO3_BUILD_EXTENSION_MODULE`
fa lo stesso effetto della feature (`fonti/pyo3-changelog-0-26-0.md`). La documentazione corrente
chiede ai progetti di abbandonare la feature, perché spegneva il collegamento anche per test e
benchmark (`fonti/pyo3-main-building-and-distribution.md:202-208`). `[V]` Monte al pin resta a 0.24.2,
dove la variabile non esiste: se monte passasse a ≥ 0.26 e alla variabile, un `cargo test` del
crate collegherebbe `libpython` da sé, come in A2, senza patch. `[INF]`

---

## Riferimenti

- **URL** https://github.com/UMEP-dev/solweig/tree/41840b74cd7c003f88dbde0c30fff736cb51bd5d · [V]
  **perché conta qui** è il crate che il banco deve collegare, al pin
  **cosa se ne prende** `rust/Cargo.toml:8`, `:11`, `:29-33`; `pyproject.toml:50-54`;
  `rust/src/lib.rs:30`; `rust/src/pipeline_scalars.rs:8`; `rust/src/pipeline.rs:571-576`
- **URL** https://github.com/UMEP-dev/solweig/issues/14 · [V]
  (catturata in `fonti/umep-solweig-issue-14.md`, sidecar accanto)
  **perché conta qui** è l'avviso di monte sui simboli di Python, e il rifiuto di mettere pyo3
  dietro una feature
  **cosa se ne prende** il limite dichiarato (`:121`) e il punto 4 aperto a una PR (`:48`), da
  confrontare con la misura di §1
- **URL** https://pyo3.rs/v0.24.2/building-and-distribution.html · [V]
  (catturata in `fonti/pyo3-0-24-2-building-and-distribution.md`, sidecar accanto)
  **perché conta qui** è la documentazione della versione di PyO3 che monte risolve
  **cosa se ne prende** cosa fa `extension-module` su Unix (`:175-179`), gli argomenti per macOS
  (`:120-147`), il file di configurazione (`:21-53`)
- **URL** https://pyo3.rs/v0.24.2/faq.html · [V]
  (catturata in `fonti/pyo3-0-24-2-faq.md`, sidecar accanto)
  **perché conta qui** descrive proprio il guasto di `cargo test` con `extension-module`
  **cosa se ne prende** le soluzioni ufficiali (`:18-45`), di cui A2 è la prima applicata al clone
- **URL** https://pyo3.rs/main/building-and-distribution.html · [V]
  (catturata in `fonti/pyo3-main-building-and-distribution.md`, sidecar accanto)
  **perché conta qui** dice dove va PyO3 dopo il pin
  **cosa se ne prende** la feature è superata da `PYO3_BUILD_EXTENSION_MODULE` (`:202-208`)
- **URL** https://raw.githubusercontent.com/PyO3/pyo3/main/CHANGELOG.md · [V]
  (estratto in `fonti/pyo3-changelog-0-26-0.md`, sidecar accanto)
  **perché conta qui** data il cambio
  **cosa se ne prende** `PYO3_BUILD_EXTENSION_MODULE` entra in 0.26.0, 2025-08-29
- **URL** https://doc.rust-lang.org/cargo/reference/features.html · [V]
  (catturata in `fonti/cargo-features.md`, sidecar accanto)
  **perché conta qui** decide se OMBRA può spegnere `extension-module` dall'esterno
  **cosa se ne prende** l'unificazione per unione (`:182-186`): no, e per questo B3 e C non funzionano
- **URL** https://doc.rust-lang.org/cargo/reference/profiles.html · [V]
  (catturata in `fonti/cargo-profiles.md`, sidecar accanto)
  **perché conta qui** decide con quale profilo si compila il codice di monte nel banco
  **cosa se ne prende** il profilo della dipendenza è ignorato (`:15`); `panic` è ignorato nei test (`:124`)
- **URL** https://doc.rust-lang.org/cargo/reference/resolver.html · [V]
  (catturata in `fonti/cargo-resolver.md`, sidecar accanto)
  **perché conta qui** dice se una `dev-dependency` raggiunge il binario distribuito
  **cosa se ne prende** le feature delle dev-dependency non si unificano fuori dai test (`:282-289`)
- **URL** https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html · [V]
  (catturata in `fonti/cargo-overriding-dependencies.md`, sidecar accanto)
  **perché conta qui** è la via (c) del ticket
  **cosa se ne prende** `[patch]` sostituisce un crate intero e va al livello più alto (`:116-120`)
- **URL** https://crates.io/crates/pyo3/0.24.2 · [V] sul sorgente scaricato da cargo
  **perché conta qui** il comportamento di collegamento sta nei build script, non nella documentazione
  **cosa se ne prende** `pyo3-ffi-0.24.2/build.rs:205-215` e
  `pyo3-build-config-0.24.2/src/impl_.rs:863-884`: quando si collega `libpython` e quando le righe
  extra passano comunque

Il test di prova è in `fonti/05-crate-rlib-passo.rs`. I crate di prova, le copie patchate del
manifesto e le uscite binarie sono rimasti nello scratchpad della sessione, fuori dal repository.

## Limiti della prova

- **Un solo ingresso**: cielo isotropo, senza vegetazione, muri, `gvf_cache`, schema di suolo 2026a
  né matrici d'ombra. I rami anisotropo e vegetazione non sono stati eseguiti. `[M]`
- **Una sola macchina e un solo compilatore.** L'uguaglianza fra vie non dice nulla
  sull'uguaglianza con la wheel di PyPI, compilata da monte con maturin e con la feature `gpu`
  (`pyproject.toml:54`). Quel confronto non è oggetto del ticket. `[INF]`
- **Linux non provato** (§4), e la CI di OMBRA è Linux.
- **Il nesso fra LTO e i riferimenti a Python** (§2, A1 e B1) è dedotto. Il binario «come monte» è
  spogliato dei simboli (`strip = "symbols"`), e non ho cercato quale simbolo esportato trattiene il
  codice di pyo3. `[INF]`

## Raccomandazione (non decisione)

Le vie si dividono in due gruppi. A1 e B1 non chiedono Python, ma funzionano solo finché il linker
scarta il codice di pyo3. Cade già col profilo di monte su macOS, e su Linux non è verificato. A2
e B2 collegano `libpython` e reggono tutti i profili provati, ma chiedono una Python con libreria
condivisa sul runner, alla compilazione e all'esecuzione. Su questo ingresso nessuna via cambia un
byte.

Se fossi io a scegliere, partirei da A2: una riga di `sed` sul manifesto del clone, dopo il
`git checkout` del pin che `vendor-check.yml` già fa, e il banco in un crate a sé col proprio
`[workspace]`. La prima cosa da fare è misurarla su `ubuntu-latest`, perché è la domanda rimasta
aperta. B2 resta come riserva senza patch, al prezzo di generare la configurazione in CI. A1 è
la più economica, se la prova su Linux nel profilo `test` dovesse passare, purché il banco non usi
mai LTO `fat`.

Da proporre a monte, se lo si ritiene utile. `pyproject.toml:54` passa già `pyo3/extension-module`
a maturin, cioè la terza soluzione della FAQ (`fonti/pyo3-0-24-2-faq.md:39-45`). Togliere la
feature da `rust/Cargo.toml:11` renderebbe A2 la configurazione di monte, senza patch dal lato di
OMBRA. Che la wheel resti identica lo deduco dalla FAQ, non l'ho verificato. `[INF]`
