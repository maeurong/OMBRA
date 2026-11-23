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

## La via, misurata

| Macchina | Via | Python | Profilo | Esito |
|---|---|---|---|---|
| macOS 27.2.1 arm64, rustc 1.99.1 di Homebrew | A2 | `/opt/homebrew/bin/python`, 3.14.10 di Homebrew (framework, `libpython3.14`) | `test` e `release` | collega e passa |
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
della classe 1 di monte, cielo anisotropo, persona in piedi. Sei Ore del
21 giugno, dalle 14 alle 20, a 43,07 N 12,55 E. Entrambi i lati girano in un pool
`rayon` a un thread.

I due lati hanno in comune l'ingresso e nient'altro. Il lato di monte ricava con
le chiamate di monte sky view factor, campi direzionali, matrici d'ombra,
`svfbuveg`, `svfalfa` e cache della geometria del GVF, mai dalla
`PassataDelCielo` di OMBRA. Un campo copiato da OMBRA porterebbe nel lato di monte
lo stesso errore, e il confronto gli darebbe ragione.

- `con_trasmissivita_costanti_il_passo_e_monte_bit_per_bit`: raggio e cielo a
  0,03, monte con `psi = 0.03`. Per ogni Ora, `tmrt`, `shadow`, `kdown`, `kup`,
  `ldown`, `lup` con gli stessi bit; dopo la sesta, `timeadd` e i sei raster dello
  Stato termico.
- `raggio_e_cielo_non_si_scambiano`: raggio a 0,5, cielo a 0,03. `shadow` uguale
  a monte con `psi = 0.5`, `kdown` sotto la chioma diverso.
- `la_trasmissivita_si_legge_cella_per_cella`: raggio a 0,03 a ovest e a 0,5 a
  est. Ciascuna metà di `shadow` uguale a monte con il proprio `psi`.

## Cosa non vede

- Il cielo isotropo. I tre casi sono anisotropi, quindi le quattro righe di
  `vegetation.rs` che ADR 0004 cambia non vengono eseguite: un errore lì lascia
  il banco verde.
- Una lettura della prima cella nel cielo anisotropo. Il cielo è costante in
  tutti e tre i casi, e su un raster costante `psi[[0, 0]]` e `psi[[r, c]]`
  hanno lo stesso valore.
- La preparazione dei dati di monte. Le chiome arrivano ai due lati con `NaN`
  dove non c'è albero, come le porta uno Strato di chioma; il Python di monte le
  riempirebbe con la quota del suolo (`fill_nan`). Il banco verifica il passo,
  non quella conversione.
