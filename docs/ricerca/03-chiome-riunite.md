# 03 — Più Strati di chioma verso il cielo: cosa legge monte, cosa si misura

Ticket: maeurong/OMBRA#33 (parte di #29). Dossier d'ingresso: `research/catena-radiativa.md` §6.

**Fonti.** Monte: `UMEP-dev/solweig` al commit `c31c5758236078b6b5f7bc9a931e2fce264bfc88`
(v0.1.0b98, il pin della copia vendorata), letto in un clone. OMBRA: `main` a `d2045fb`,
versione 0.2.0. Ogni `percorso:riga` di monte è relativo a quel commit.

Tag: `[V]` letto nel sorgente · `[M]` misurato eseguendo · `[INF]` inferenza · `[NON MISURATO]`.

**Il banco non è versionato.** Le misure vengono da un crate di prova fuori dal repository, che
dipende per percorso da `ombra` e dalla copia vendorata e chiama `calculate_svf_inner` con gli
argomenti di `src/motore.rs:203-226`, su un pool a un thread. Le definizioni qui sotto bastano a
rifarlo. Toolchain: `rustc 1.98.0`.

---

## 1. Cosa consuma il passo fuso della geometria delle chiome

Il passo prende una `vegshmat` e una `vbshmat` (`rust/src/pipeline.rs:534-535`), più i campi del
bundle degli sky view factor. `[V]`

| Ingresso | Chi lo legge | Modello di cielo |
|---|---|---|
| `vegshmat` | `compute_ani_lum_from_packed` (`rust/src/pipeline_radiation.rs:401`), `anisotropic_sky_pure` (`rust/src/sky.rs:330`) | anisotropo |
| `vbshmat` | `anisotropic_sky_pure` (`sky.rs:331`) | anisotropo |
| `svf_veg`, attraverso `svfalfa` | `pysrc/solweig/components/svf_resolution.py:98-100`, poi `cylindric_wedge_pure_masked` (`pipeline.rs:1400`) | entrambi |
| `svfbuveg` | `compute_kup` (`pipeline.rs:1416`), `compute_kdown` (`:1762`, `:1867`) | entrambi |
| `svf_veg` direzionali, `svf_aveg*` | `compute_ldown`, `kside_veg_isotropic_pure`, `lside_veg_pure` (`pipeline.rs:1787-1840`) | solo isotropo |

Tutti `[V]`, sul percorso classico, che è quello che OMBRA esegue. Lì, nel ramo anisotropo fra
`pipeline.rs:1479` e `:1779`, nessuna riga nomina i direzionali della vegetazione. Nel ramo dello
schema di suolo 2026a, spento in OMBRA, li legge anche l'anisotropo (`pipeline.rs:899-905`,
`:984-997`).

**Dove agisce la trasmissività verso il cielo.** In un punto per patch,
`diff = sh − (1 − vsh) · (1 − psi)` (`pipeline_radiation.rs:402`), che dà la diffusa verso il
basso, e in `svfbuveg = svf − (1 − svf_veg) · (1 − psi)` (`svf_resolution.py:141`). In
`anisotropic_sky_pure` la chioma è opaca: una patch è cielo se `sh && vsh`, vegetazione se
`!vsh || !vbsh`, edificio se `!sh && vbsh` (`sky.rs:332-334`). La patch di vegetazione emette in
onda lunga e non dà diffusa laterale (`sky.rs:360-376`, `:469-478`). `[V]`

**Chi consuma le matrici legge patch per patch** e si ferma al numero di patch (`sky.rs:17`,
`pipeline_radiation.rs:397-401`): i bit di riempimento dell'ultimo byte non vengono letti. `[V]`

## 2. Come il nucleo scrive matrici e campi

Nella copia vendorata, al pin. `[V]`

- Le tre matrici sono allocate a zero (`vendor/solweig/src/skyview.rs:258-260`). Per ogni patch si
  accende un bit dove l'ombra vale almeno 0,5: bit a uno, patch libera (`:534-562`).
- Dalla **stessa** ombra per patch il nucleo accumula i campi: `svf*` dall'ombra degli edifici
  (`:567-579`), `svf_veg*` da quella della vegetazione (`:582-594`), `svf_veg_blocks_bldg_sh*`
  dalla terza (`:596-610`). Matrice e campo sono due scritture della stessa cosa.
- Senza chioma le due matrici della vegetazione diventano tutte a uno (`:746-750`) e i dieci
  campi della vegetazione restano a zero, come allocati (`:248-257`). A monte è Python a metterli
  a uno (`pysrc/solweig/models/precomputed.py:143-152`).
- L'ombra della vegetazione su una cella dipende, a ogni passo della marcia, dalla sola cella
  sorgente (`vendor/solweig/src/shadowing.rs:418-436`), e viene azzerata dove l'edificio fa già
  ombra (`:438-447`). Ne segue che l'ombra di due Strati senza celle in comune è l'unione delle
  due. `[INF]`, confermata dalla misura M2.

## 3. Misure

**Griglie.** `[M]` dove non è detto altro.

| Sigla | Che cos'è | Strati di chioma (trasmissività, celle con chioma) |
|---|---|---|
| SdF | Bastia, Scenario «stato-di-fatto», Periodo «inverno», 50 × 50 a 1 m | «chiome» 0,03 / 309 · «chiome spoglie» 0,5 / 307 |
| Int | Bastia, Scenario «interventi», Periodo «inverno», 50 × 50 a 1 m | «chiome» 0,03 / 309 · «chiome spoglie» 0,5 / 537 |
| Sin | di prova, piana, 41 × 41 a 1 m: un blocco 5 × 5 sempreverde e uno spoglio, staccati, cima a 10 m e zona-tronco a 3 m | 0,03 / 25 · 0,5 / 25 |
| Sov | di prova, come Sin ma con i blocchi sovrapposti su quindici celle: sempreverde con cima a 10 m e zona-tronco a 3 m, spoglio con cima a 14 m e zona-tronco a 8 m | 0,03 / 25 · 0,5 / 25 |

La geometria di Sin e Sov l'ha scelta il banco, non viene da una fonte.

**Passate.** Una per Strato, come alla 0.2.0, più una **riunita** su un solo raster con tutte le
chiome. Sulla cella che due Strati condividono il raster riunito è stato costruito in due modi:
*la più alta vince*, cioè la cima più alta con la sua zona-tronco, che è la regola di `posa`
dentro uno Strato (`src/derivazione.rs:148-163`); e *inviluppo*, cioè la cima più alta con la
zona-tronco più bassa.

### M1 — Bit di riempimento con chioma

153 patch in 20 byte: sette bit di riempimento nell'ultimo. In tutte e quattro le Griglie, in
tutte le passate con chioma e in tutte e tre le matrici, i sette bit sono a zero in ogni cella.

### M2 — Matrici della passata riunita contro l'AND bit a bit delle matrici per Strato

Confronto patch per patch, da 0 a 152.

| | SdF | Int | Sin | Sov, la più alta vince | Sov, inviluppo |
|---|---|---|---|---|---|
| Celle con chioma in entrambi gli Strati | 0 | 0 | 0 | 15 | 15 |
| `veg_sh_matrix`: coppie (cella, patch) diverse | 0 su 382.500 | 0 su 382.500 | 0 su 257.193 | 1980, in 1018 celle | 0 |
| `veg_blocks_bldg_sh_matrix`: coppie diverse | 0 | 0 | 0 | 0 | non misurato |
| `bldg_sh_matrix` fra le passate | identica | identica | identica | identica | — |

Le 1980 coppie di Sov vanno tutte nello stesso verso: libere nella riunita, coperte in uno Strato.

`veg_blocks_bldg_sh_matrix` è quasi vuota: ha 128 bit a zero in SdF e 4406 in Int, tutti dallo
Strato «chiome spoglie», e nessuno nelle Griglie di prova, che non hanno Edifici. Il suo zero di
differenze è il confronto di uno Strato con se stesso.

### M3 — Sky view factor con le chiome, quattro modi di calcolarlo

- **Conto patch per patch.** Somma, sui 153 pesi isotropi del nucleo
  (`vendor/solweig/src/skyview.rs:105-118`, copiati nel banco), di quanto passa da ogni patch:
  zero dove un edificio la copre, uno dove è libera, la trasmissività dello Strato che la copre,
  il prodotto delle due dove la coprono entrambi. È il riferimento degli scarti qui sotto: esatto
  rispetto ai bit del nucleo, non a una misura fisica. Controllo: la stessa somma sui bit degli
  edifici riproduce `svf` del nucleo con scarto zero.
- **Somma per Strato**: `svf − Σ (1 − svf_veg_s) · (1 − ψ_s)`, troncata a zero. È
  `con_le_chiome` alla 0.2.0 (`src/motore.rs:242-254`).
- **Media pesata**: la formula di monte sul campo della passata riunita, con
  `ψ = Σ (1 − svf_veg_s) · ψ_s / Σ (1 − svf_veg_s)`.
- **Differenza fra i tre campi.** Con `a` e `b` la volta tolta dai due Strati e `u` quella tolta
  dalla passata riunita, cioè `1 − svf_veg` di ciascuna: copre solo il primo `u − b`, solo il
  secondo `u − a`, entrambi `a + b − u`. Detrazione:
  `(u − b)(1 − ψ_a) + (u − a)(1 − ψ_b) + (a + b − u)(1 − ψ_a ψ_b)`.

| | SdF | Int | Sin | Sov, inviluppo |
|---|---|---|---|---|
| Media, somma per Strato | 0,706417 | 0,593398 | 0,944868 | 0,949704 |
| Media, media pesata | 0,711696 | 0,612056 | 0,945596 | — |
| Media, differenza fra i tre campi | 0,710361 | 0,607712 | 0,945348 | 0,956943 |
| Scarto assoluto dal conto patch per patch, somma per Strato: medio / massimo | 0,003944 / 0,033047 | 0,014315 / 0,188470 | 0,000480 / 0,008214 | 0,007239 / 0,044154 |
| … media pesata: medio / massimo | 0,001335 / 0,010454 | 0,004344 / 0,068889 | 0,000248 / 0,006884 | — |
| … differenza fra i tre campi: medio / massimo | 2,2 · 10⁻⁷ / 5,7 · 10⁻⁷ | 2,2 · 10⁻⁷ / 6,1 · 10⁻⁷ | 8,4 · 10⁻⁸ / 2,6 · 10⁻⁷ | 8,6 · 10⁻⁸ / 2,8 · 10⁻⁷ |
| Coppie (cella, patch) coperte da entrambi gli Strati | 4192 | 13.573 | 330 | 2967 |
| Celle in cui la somma per Strato viene troncata a zero | 0 | 0 | 0 | 0 |

In Sov con *la più alta vince* la differenza fra i tre campi sbaglia in media di 1,9 · 10⁻³ e al
più di 1,5 · 10⁻², e in 312 celle una delle tre quote esce negativa: la passata riunita toglie
meno volta dell'unione dei due Strati.

### M4 — Quante celle vedono entrambi gli Strati

Celle in cui ciascuno dei due Strati copre almeno una patch: 2483 su 2500 in SdF, 2487 su 2500 in
Int, tutte in Sin e in Sov.

### Tempi

Non cronometrati qui. `vendor/solweig/PROVENIENZA.toml`, voce `tempo_sky_view_factor`, riporta da
32 a 63 ms per Corsa sul caso di riferimento, con una passata d'estate e due d'inverno.

## Cosa resta non misurato

- **Quanto pesa una sola trasmissività verso il cielo per cella** quando le patch non hanno la
  stessa luminosità (cielo anisotropo) e quando le quattro direzioni non hanno la stessa copertura
  (isotropo). Il conto patch per patch qui usa i pesi isotropi. Il modello di Perez non è nella
  copia vendorata. `[NON MISURATO]`
- **L'inviluppo quando le due chiome della stessa cella stanno una sopra l'altra con un vuoto in
  mezzo.** In Sov si toccavano. `[NON MISURATO]`
- **`veg_blocks_bldg_sh_matrix` sotto l'inviluppo**, e in generale su una Griglia in cui entrambi
  gli Strati stiano davanti a un edificio. `[NON MISURATO]`
- **La trasmissività ricavata dalla detrazione e rimessa nella formula di monte in `f32`.** Il
  banco ha misurato lo sky view factor ricavato direttamente dalla differenza. La volta tolta dalla
  passata riunita, che finirebbe a denominatore, vale al minimo 0,047 in SdF e 0,13 in Int.
  `[NON MISURATO]`
- **La terza passata al tetto di progetto**, 200 × 200 m a 2 m. `[NON MISURATO]`
- **La correzione dell'ultimo anello sui direzionali della vegetazione.** Il nucleo la aggiunge a
  `svf_veg_s` e `svf_veg_w` solo dove la zona-tronco vale `0.0`
  (`vendor/solweig/src/skyview.rs:662-675`). OMBRA passa NaN dove non c'è albero
  (`src/derivazione.rs:124-134`), quindi la correzione non scatta in nessuna cella. `[V]` Che cosa
  passi monte in quelle celle non è stato letto. Riguarda l'isotropo. `[NON MISURATO]`
