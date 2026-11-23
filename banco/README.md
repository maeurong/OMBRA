# Il banco di parità

Confronta il passo di OMBRA, cioè la copia vendorata estesa da ADR 0004, con il
passo di monte (`compute_timestep_rust` del crate `rustalgos`), sullo stesso
ingresso, bit per bit. Il clone di monte è al commit che
`vendor/solweig/PROVENIENZA.toml` fissa.

```bash
bash banco/prepara.sh
cargo test --manifest-path banco/Cargo.toml --release
```

`prepara.sh` clona monte in `banco/monte/`, ne fa il checkout al commit letto da
`PROVENIENZA.toml` e toglie `extension-module` dalle feature di `pyo3` nel
manifesto del clone. È la via A2 di `docs/ricerca/05-crate-rlib.md`: così `pyo3`
collega `libpython`, e il binario di test si collega. Serve quindi una Python 3
con libreria condivisa, alla compilazione e all'esecuzione. Lo script stampa
quella che `pyo3` userà. Se il clone o la modifica al manifesto falliscono, esce
con un codice diverso da 0: un banco che si salta da solo darebbe verde senza
confrontare niente.

Il banco è un crate a sé, con un proprio `[workspace]`: `pyo3` e il clone non
entrano mai nel `Cargo.lock` di OMBRA né nel suo binario.

Il `Cargo.lock` del banco parte da quello di monte, quindi nel banco OMBRA si
compila con le versioni di monte e non con quelle del `Cargo.lock` della radice:
`rayon` 1.11.0 invece di 1.12.0, `matrixmultiply` 0.3.10 invece di 0.3.11. La
parità di bit è fra i due nuclei sullo stesso insieme di dipendenze.

## La via, misurata

| Macchina | Via | Python | Profilo | Esito |
|---|---|---|---|---|
| macOS 27.2.1 arm64, rustc 1.99.1 di Homebrew | A2 | `/opt/homebrew/bin/python`, 3.14.10 di Homebrew (framework, `libpython3.14`) | `test` e `release` | collega e passa i quattro casi |
| `ubuntu-latest` | A2 | 3.12 di `actions/setup-python` | `release` | da misurare: serve il primo giro del lavoro `parita` |

Su macOS `pyo3` ha scelto `python` (3.14) e non `python3` (3.15), perché cerca
in quest'ordine: `PYO3_PYTHON`, `python`, `python3`. `otool -L` sul binario di
test mostra il framework della 3.14.

Il profilo `release` è quello predefinito del banco. Cargo ignora il
`[profile.release]` del clone, con LTO `fat`, perché è il profilo di una
dipendenza, e lo segnala a ogni compilazione.

## Cosa confronta

Un ingresso costruito nel test, 16 × 16 celle a 1 m: un Edificio di 4 × 4 celle
alto 6 m, un Albero con chioma da 5 a 8 m e zona tronco fino a 2 m, Muri ricavati
dalla copia vendorata di `compute_wall_aspect_pure`, suolo uniforme ai valori
della classe 1 di monte, persona in piedi, cielo anisotropo nel secondo e nel terzo caso, entrambi i
cieli nel primo e nel quarto. Sei Ore del
21 giugno, dalle 14 alle 20, a 43,07 N 12,55 E. Entrambi i lati girano in un pool
`rayon` a un thread.

I due lati hanno in comune l'ingresso e nient'altro. Il lato di monte ricava con
le chiamate di monte sky view factor, campi direzionali, matrici d'ombra,
`svfbuveg`, `svfalfa` e cache della geometria del GVF, mai dalla
`PassataDelCielo` di OMBRA. Un campo copiato da OMBRA porterebbe nel lato di monte
lo stesso errore, e il confronto gli darebbe ragione.

- `con_trasmissivita_costanti_il_passo_e_monte_bit_per_bit`: raggio e cielo a
  0,03, monte con `psi = 0.03`, con il cielo anisotropo e con quello isotropo.
  Per ogni Ora, `tmrt`, `shadow`, `kdown`, `kup`, `ldown`, `lup` con gli stessi
  bit; dopo la sesta, `timeadd` e i sei raster dello Stato termico. Nel cielo
  isotropo è l'unico caso che confronta `tmrt`, `kup`, `ldown` e `lup` nelle Ore
  di sole: il quarto li confronta solo nell'ultima. Lo Stato termico passa da un'Ora all'altra come lo passa monte
  (`pysrc/solweig/computation.py:723-727`): dopo un'Ora di giorno
  `firstdaytime = 0`, e il passo legge la storia che riceve.
- `raggio_e_cielo_non_si_scambiano`: raggio a 0,5, cielo a 0,03. `shadow` uguale
  a monte con `psi = 0.5`, `kdown` sotto la chioma diverso.
- `la_trasmissivita_si_legge_cella_per_cella`: raggio a 0,03 a ovest e a 0,5 a
  est. Ciascuna metà di `shadow` uguale a monte con il proprio `psi`.
- `le_due_trasmissivita_si_leggono_cella_per_cella_in_entrambi_i_cieli`: raggio e
  cielo a 0,03 a ovest e a 0,5 a est, con il cielo anisotropo e con quello
  isotropo. In ogni Ora, ciascuna metà di `shadow` e `kdown` uguale a monte con il
  proprio `psi`. `tmrt`, `kup`, `ldown` e `lup` si confrontano solo nell'Ora in cui
  l'ombra non dipende da `psi`, l'ultima, col sole sotto i 3 gradi. Qui
  `firstdaytime` resta a 1, e il passo non legge la storia: portata come nel
  primo caso, la storia delle Ore di sole entrerebbe nell'ultima, che non sarebbe
  più esatta.

## Cosa non vede

- `tmrt`, `kup`, `ldown` e `lup` nelle Ore di sole del quarto caso. Leggono il
  suolo intorno alla cella fino a 22 celle di distanza, quindi su 16 celle
  ciascuna metà risente dell'ombra dell'altra. Misurato: in quelle Ore
  differiscono da monte fino in tutte le colonne di una metà, e nessuna distanza
  dalla mezzeria basta a renderle esatte.
- Uno scambio fra raggio e cielo nel quarto caso, dove i due raster sono uguali.
  Lo vede il secondo caso.
- La storia dello Stato termico nel quarto caso, per la ragione detta sopra. La
  vede il primo caso.
- Il ramo notturno di `dopo_l_ora` (`firstdaytime = 1`, `timeadd = 0`): non gira
  mai, perché tutte e sei le Ore sono di giorno. Anche l'ultima, col sole a 2,6
  gradi, conta come giorno.
- Una Trasmissività letta dalla cella sbagliata nel cielo isotropo, nelle Ore di
  sole. Nel primo caso il raster è costante, quindi ogni cella vale quanto le
  altre. Misurato: con `kside_veg_isotropic_pure` che legge sempre la cella
  `(0, 0)` il primo caso resta verde, e il quarto diventa rosso solo all'ultima
  Ora.
- La preparazione dei dati di monte. Le chiome arrivano ai due lati con `NaN`
  dove non c'è albero, come le porta uno Strato di chioma; il Python di monte le
  riempirebbe con la quota del suolo (`fill_nan`). Il banco verifica il passo,
  non quella conversione.

## Le letture della Trasmissività che il banco non copre

- `vendor/solweig/src/pipeline.rs:414`, `let psi = trasmissivita_cielo_v;` dentro
  `if use_scheme` (`:384`): è il ramo dello schema di suolo 2026a, e OMBRA passa
  sempre `ground_scheme: None`. Non gira né in OMBRA né nel banco.
- `vendor/solweig/src/pipeline_radiation.rs:422`: è il ripiego per un raster
  d'uscita non contiguo. Il raster d'uscita nasce da `Array2::zeros` (`:384`) ed è
  sempre contiguo, quindi gira il ramo di `:403` e quel ripiego non gira mai, né
  in OMBRA né nel banco.
- `vendor/solweig/src/vegetation.rs:411`, `:413`, `:415`, `:417`, le direzioni E, S,
  W e N: le mutazioni provate le cambiano tutte e quattro insieme. Un errore in
  una direzione sola non è stato provato.
