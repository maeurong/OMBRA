# 04 — Lo stato termico di monte fra un'Ora e la successiva, e se serve un avviamento

Ticket: maeurong/OMBRA#21, «Lo stato termico e la forma della Corsa». Ricerche d'ingresso:
`01-passo-fuso.md:216-229` (stato dopo il passo, letto a `02246ab`) e
`research/catena-radiativa.md:228` (stato iniziale).

**Fonti.** Codice di monte `UMEP-dev/solweig` al commit pinnato
`41840b74cd7c003f88dbde0c30fff736cb51bd5d` (tag `v0.1.0b101`, `vendor/solweig/PROVENIENZA.toml:9`),
letto in un clone fuori dal repository (`git -C <clone> log -1` → `41840b7… (HEAD, tag: v0.1.0b101)`).
Ogni `percorso:riga` di monte senza altra indicazione è relativo a quel commit. Per il confronto
con il SOLWEIG originale: `UMEP-dev/UMEP-processing` al commit
`a7798db1698c29fa5ba667e082e7ad564c20e8e8`, con gli estratti catturati in
`fonti/umep-processing-a7798db-stato-notturno.md`. OMBRA: worktree partito da `main` a `90f65c0`.

Tag: `[V]` letto nella fonte · `[M]` misurato con comando · `[INF]` inferenza · `[NON TROVATO]`.

**Banco di misura.** La wheel `solweig==0.1.0b101` di PyPI (`cp311-abi3-macosx_11_0_arm64`), in un
virtualenv Python 3.12.13 creato con `uv 0.12.9`, su macOS arm64. I sorgenti Python della wheel
coincidono file per file con `pysrc/solweig/` del clone al pin (`diff -rq`, nessuna differenza
oltre a `rustalgos.abi3.so`). Che la parte Rust compilata venga da quel commit lo deduco dal tag,
non l'ho verificato. `[INF]` Tutte le misure sotto sono eseguite con `SOLWEIG_NO_GPU=1`, cioè
sul percorso CPU, che è quello di OMBRA; il primo giro, con la GPU attiva, dà le stesse
conclusioni (§4). Gli script sono in `fonti/04-stato-termico-misura.py` e
`fonti/04-stato-termico-epw.py`.

---

## Premesse del brief, verificate

- `01-passo-fuso.md:216-229` descrive la logica dopo il passo a `02246ab`. Al pin è la stessa e si
  è spostata a `pysrc/solweig/computation.py:705-727`. Il `[INF]` di quella ricerca sul ritardo
  termico è ora `[V]` sul codice e `[M]` sulla misura (§1, §4). `[V]`
- `research/catena-radiativa.md:228`: lo stato iniziale è ancora quello, a
  `pysrc/solweig/models/state.py:61-83`. `[V]`
- Il passo è `compute_timestep_rust` (`rust/src/pipeline.rs:571`), che valida e chiama
  `timestep_physics` (`:576`); lo stato entra da `ThermalStateViews`
  (`rust/src/pipeline_inputs.rs:63-74`). `[V]`
- **Correzione a `01-passo-fuso.md:205`.** Lì si legge che di notte `clearness_index = 1.0`. È vero
  per `Weather.compute_derived` (`pysrc/solweig/models/weather.py:253-258`, `:278-284`), ma in una
  serie temporale quel valore viene poi sovrascritto con l'ultimo indice di giorno
  (`pysrc/solweig/timeseries.py:131-143`). Il riporto c'era già a `02246ab`: entra con il commit
  `59e6c81` (0.1.0b56), antenato di `02246ab` (`git merge-base --is-ancestor`). Non cambia la
  domanda di #21, ma è la memoria che la smonta in parte (§3). `[V]`

---

## 1. Lo stato al pin: campi, stato iniziale, aggiornamento

**Campi che entrano** (`ThermalStateViews`, `rust/src/pipeline_inputs.rs:63-74`): `firstdaytime: i32`,
`timeadd: f32`, `timestep_dec: f32` (frazione di giorno), e sei raster `f32`: `tgmap1`,
`tgmap1_e`, `tgmap1_s`, `tgmap1_w`, `tgmap1_n`, `tgout1`. `[V]`

**Campi che escono** (`TimestepResultRaw`, `rust/src/pipeline.rs:249-267`): `timeadd` e gli stessi
sei raster. `firstdaytime` e `timestep_dec` non escono: li decide Python. Escono inoltre `tg`,
`rn`, `rn_past`, `g`, che sono `Some` solo con lo schema di suolo 2026a. `[V]`

**Stato iniziale** (`ThermalState.initial`, `pysrc/solweig/models/state.py:61-83`): i sei raster a
zero `float32`, `firstdaytime = 1.0`, `timeadd = 0.0`, `timestep_dec = 0.0`. La serie temporale
crea lo stato una volta per tile e ci scrive subito il passo
(`pysrc/solweig/timeseries.py:587-588`). `[V]`

**Dopo ogni passo** (`pysrc/solweig/computation.py:705-727`): `state.timeadd = result.timeadd` e i
sei raster copiati dal risultato (`:707-721`). Poi, se `weather.is_daytime`,
`firstdaytime = 0.0`; altrimenti `firstdaytime = 1.0` e `timeadd = 0.0` (`:723-727`). `[V]`

**Il ritardo termico** (`ts_wave_delay_batch_pure`, `rust/src/ground.rs:134-221`):

1. con `firstdaytime == 1`, i sei raster di storia vengono sostituiti dagli ingressi del passo
   corrente: `tgmap1* ← lup*` del GVF, `tgout1 ← tg_temp` (`:159-166`);
2. soglia `59/1440` di giorno e costante `-33.27` (`:168-169`);
3. se `timeadd ≥ soglia`: peso `w = exp(-33.27 · timeadd)`, uscita
   `corrente · (1 − w) + storia · w`, la storia diventa l'uscita, e `timeadd` torna a
   `timestep_dec` se questo supera la soglia, altrimenti a zero (`:171-200`);
4. altrimenti `timeadd += timestep_dec`, stessa media pesata, storia invariata (`:201-220`). `[V]`

Con il passo orario, `timestep_dec = 1/24 = 60/1440 > 59/1440`: ogni passo di giorno dopo il primo
prende il ramo 3, con `w = exp(-33.27/24) ≈ 0,25`. La storia è una media mobile esponenziale che
dimezza il proprio peso due volte all'ora. `[V]` sul codice, aritmetica mia.

**La catena giorno-notte che ne segue**, con passo orario. `[V]` sul codice, `[M]` in §4:

- **primo passo di giorno**: entra con `firstdaytime = 1` (lasciato dalla notte) e `timeadd = 0`.
  La storia viene sostituita dal presente; l'uscita non dipende da nulla di prima;
- **passi di giorno successivi**: media con la storia di quel giorno soltanto;
- **primo passo di notte**: entra con `firstdaytime = 0` e `timeadd = 1/24`, lasciati dall'ultimo
  passo di giorno. Il Rust lo calcola come un passo qualunque (nessun ritorno anticipato,
  `rust/src/pipeline.rs:1287-1307`), quindi **porta ancora la memoria del giorno**. Solo dopo
  Python scrive `firstdaytime = 1`, `timeadd = 0`;
- **dal secondo passo di notte in poi**: la storia viene sostituita a ogni passo, e il ritardo è
  l'identità.

La memoria termica dello stato dura quindi dal primo passo di giorno al primo passo di notte
compreso. Non attraversa mai una notte di almeno due ore.

**Differenza dal SOLWEIG originale.** In UMEP-processing il ramo di notte non chiama il ritardo
termico: `Lup` di notte è `SBC · emis · (Ta + Tg + 273.15)^4` con `Tg = 0`, e `Tgmap1` non si
aggiorna (`Solweig_2025a_calc_forprocessing.py:389-411` dentro il ramo di giorno, `:482-522` per la
notte; estratti in `fonti/umep-processing-a7798db-stato-notturno.md`). Il primo passo di notte di
monte, che porta la memoria del giorno, non ha quindi un corrispettivo in UMEP. `[V]` su entrambi i
codici; l'effetto in kelvin non l'ho misurato. `[NON TROVATO]`

## 2. La condizione di «notte»

**Per lo stato termico, notte vuol dire `sun_altitude <= 0`.** `computation.py:723` legge
`weather.is_daytime`, che è `self.sun_altitude > 0` (`pysrc/solweig/models/weather.py:289-292`).
Lo stesso confronto passa al Rust come `is_daytime` (`computation.py:384`), dove azzera `Tg` e
`Tg_wall` (`rust/src/pipeline.rs:711-723`). `[V]`

- L'altitudine è quella al centro dell'intervallo del record: `datetime − timestep_minutes/2`
  (`weather.py:164-191`), con `timestep_minutes = 60` di default (`weather.py:87`). `[V]`
- Non c'entra `min_sun_altitude = 3.0` (`rust/src/pipeline.rs:671`): vale solo per le ombre. `[V]`
- Non c'entra la radiazione. `[V]`

**Per il riporto dell'indice di serenità, notte vuol dire un'altra cosa:**
`sun_altitude > 0 and global_rad > 0` è giorno, tutto il resto è notte
(`pysrc/solweig/timeseries.py:140`). Un'ora con il sole sopra l'orizzonte e `global_rad = 0` è
quindi giorno per lo stato termico e notte per l'indice. Succede in un EPW reale: Perugia, 24/1,
record delle 09:00, altitudine 8,2°, GHI = 0 nel file. `[V]` sul codice, `[M]` nel file.

## 3. Le altre memorie fra un'Ora e la successiva

Con `ground_scheme: None`, cioè il percorso che OMBRA esegue.

| Candidato | Memoria fra un passo e l'altro? | Fonte |
|---|---|---|
| `tgmap1*`, `tgout1`, `timeadd`, `firstdaytime` | sì, quella descritta in §1; azzerata dalla notte | `computation.py:705-727` `[V]` |
| `timestep_dec` | no: costante per tutta la serie, fissata dai primi due record | `timeseries.py:462-471`, `:588` `[V]` |
| temperatura dei muri `tg_wall` | no: sinusoide di `altmax`, `dectime`, `snup`, ricalcolata a ogni passo | `rust/src/ground.rs:75-83` `[V]` |
| `tg` del suolo (modello classico) | no: stessa sinusoide, per pixel | `ground.rs:47-73` `[V]` |
| `gvf_cache` | no: geometria pura (azimut, `gvfalbnosh`), chiave sulla geometria | `rust/src/gvf_geometry.rs:47-62`; `computation.py:438-491`; `timeseries.py:615-629` `[V]` |
| `altmax` | no: funzione della data, cache per giorno | `timeseries.py:88-124`; `weather.py:36-39` `[V]` |
| accumulatori del riepilogo (`GridAccumulator`) | no: leggono il risultato, non rientrano nel passo | `timeseries.py:657` `[V]` |
| **`clearness_index` di notte** | **sì: l'ultimo indice di giorno resta per tutte le ore di notte che seguono, anche oltre la mezzanotte, fino alla prima ora con sole e radiazione** | **`timeseries.py:131-143`** `[V]`, `[M]` in §4 |
| stato dello schema 2026a (`tg`, `tm`, `rn`, `rn_past`, `g`, `shadow_past`) | sì, ma solo con lo schema attivo; inizializzato dalla serie di `Ta` del primo giorno | `computation.py:729-737`; `timeseries.py:590-606` `[V]` |

**Il riporto dell'indice è la sola memoria che la notte non azzera.** Dove pesa: l'indice entra in
`Ldown` con la correzione per le nuvole quando vale meno di 0,95, sia nel cielo anisotropo
(`rust/src/pipeline.rs:1443-1451`) sia nell'isotropo (`rust/src/pipeline_radiation.rs:134`).
Quando parte una serie, il valore riportato vale 1,0 (`timeseries.py:138`). Una Corsa che comincia
a mezzanotte, nelle ore prima dell'alba, ha quindi l'indice 1,0. La stessa notte preceduta dal
giorno prima ha l'indice di quella sera. `[V]`

Nel SOLWEIG originale l'indice notturno funziona in un altro modo. A ogni mezzanotte viene
**ricalcolato dal primo record dopo l'alba del giorno che comincia**, e tagliato a 1
(`Solweig_run.py:861-881`). Fra il tramonto e la mezzanotte resta l'ultimo valore di giorno,
tagliato a 1 anch'esso (`Solweig_2025a_calc_forprocessing.py:220-227`). In UMEP le ore dopo la
mezzanotte guardano quindi in avanti, al mattino, e non indietro. Monte dichiara il contrario nel
commento di `timeseries.py:132-133` («the last daytime CI value persists into the nighttime
hours»). Il commento vale solo per le ore prima della mezzanotte. `[V]` su entrambi i codici.

## 4. Misura: Corsa A (giorni 1-3) contro Corsa B (giorni 2-3)

**Impianto.** DSM 40 × 40 a 1 m, un blocco 10 × 10 alto 12 m; `SurfaceData.prepare` calcola muri e
SVF. Record orari con l'etichetta alla fine dell'ora (la convenzione di monte, `weather.py:164-170`),
dal giorno 1 01:00 al giorno 4 00:00. Il meteo sintetico: `Ta` sinusoidale fra 18 e 31 °C, UR 50 %,
vento 1 m/s, `global_rad` pari all'`I0` di `clearnessindex_2013b` (sereno) moltiplicato per un
fattore per giorno. Corsa B prende i record dal giorno 2 01:00. Il confronto passa per
`solweig.calculate`; una spia su `solweig.api._calculate_single` copia dopo ogni passo Tmrt, Ldown,
Lup e lo stato. Uguaglianza con `np.array_equal`, altrimenti il massimo dello scarto assoluto sulla
griglia. Comando:

```
SOLWEIG_NO_GPU=1 <venv>/bin/python -I docs/ricerca/fonti/04-stato-termico-misura.py <out.json>
```

| Caso | Passi in comune | Passi con uno scarto | Dove e quanto |
|---|---|---|---|
| Bastia (43,07 N), 14-16/7, tre giorni sereni | 48 | **0** | Tmrt, Ldown, Lup, `tgmap1`, `tgout1`, `timeadd`, `firstdaytime` uguali bit per bit a ogni ora |
| Bastia, giorno 1 coperto (0,35 × sereno), giorni 2-3 sereni | 48 | **5** | 15/7 dalle 01:00 alle 05:00, prima dell'alba: Tmrt 0,69-0,71 K, Ldown 10 W/m². Lup, `tgmap1`, `tgout1`, `timeadd`, `firstdaytime` uguali. Indice 0,888 in A, 1,000 in B. Dall'alba in poi tutto uguale |
| Svalbard (78,22 N), 20-22/6, sole di mezzanotte, sereno | 48 | **17** | vedi §7 |

Tutti `[M]`. Il primo giro, con la GPU attiva, dà gli stessi numeri nei primi due casi. Nel terzo
dà 13 passi diversi invece di 17: la coda di uno scarto di 3·10⁻⁵ si spegne prima. `[M]`

**Lettura.** Lo stato termico non chiede avviamento: una Corsa che parte a mezzanotte dallo
stato iniziale ha `tgmap1*`, `tgout1` e Lup uguali bit per bit a quelli di una Corsa preceduta dal
giorno prima, dalla prima ora. Tmrt no: nelle ore prima dell'alba del primo giorno può scostarsi,
se la sera prima ha lasciato un indice di serenità sotto 0,95. `[M]`

**Su un EPW reale.** Perugia-Sant'Egidio, TMYx (`ITA_UM_Umbria-Perugia-d-Assisi.AP.161810_TMYx.epw`,
da `https://climate.onebuilding.org/WMO_Region_6_Europe/ITA_Italy/UM_Umbria/ITA_UM_Umbria-Perugia-d-Assisi.AP.161810_TMYx.zip`,
sha256 dell'EPW `daba2108588587ed44ca610e88bd060a66a42350dcbd13e1dbbf5aa9c3a3d795`; il file non
è nel repository). Ho portato i record in un solo anno, il 2023, perché un TMYx prende i mesi da
anni diversi. Comando:

```
SOLWEIG_NO_GPU=1 <venv>/bin/python -I docs/ricerca/fonti/04-stato-termico-epw.py <file.epw>
```

- Con il `_precompute_weather` di monte su tutto l'anno, l'ora 1 è notte in 365 giorni su 365.
  **In 40 giorni (11 %) eredita un indice sotto 0,95.** Per mese: gen 19 %, feb 25 %, mar 16 %,
  apr 7 %, mag 19 %, giu 0 %, lug 0 %, ago 3 %, set 13 %, ott 26 %, nov 3 %, dic 0 %. Quantili
  dell'indice ereditato (10/25/50/75/90 %): 0,943 / 1,344 / 1,679 / 2,422 / 4,086. I valori
  oltre 1 sono il residuo di ore a sole basso, che monte non taglia. `[M]`
- **Corse A e B sui dati reali**, stesso DSM. A comincia il giorno prima di B. Con B al 15/1, 15/4,
  15/7 e 15/10 lo scarto in Tmrt è zero su 48 ore. Ho poi scelto i quattro giorni con l'indice
  ereditato più basso, uno per mese, quindi i casi peggiori dell'anno:

  | B dal | Indice in A | Ore con scarto | Scarto massimo di Tmrt |
  |---|---|---|---|
  | 24/1 | 0,692 | 9 (01:00-09:00) | 1,84 K |
  | 9/3 | 0,580 | 7 (01:00-07:00) | 2,56 K |
  | 11/9 | 0,500 | 7 (01:00-07:00) | 2,39 K |
  | 19/10 | 0,543 | 7 (01:00-07:00) | 2,77 K |

  In B l'indice è 1,0 in tutte quelle ore. Lo scarto finisce alla prima ora con sole e
  radiazione: il 24/1 dura fino alle 09:00 perché il file ha GHI = 0 alle 08:00 e alle 09:00 con il
  sole già alto (§2). `[M]`

## 5. Monte prevede un avviamento?

**No.** Nel clone al pin, `grep -rni` di `spin-up`, `warm-up`, `burn-in` trova solo
`_warm_tile_pages`, che precarica in memoria le pagine della tile successiva
(`timeseries.py:554-582`). `[V]`

- Il commento di `ThermalState` dice che la temperatura delle superfici «depends on accumulated
  heating throughout the day» (`pysrc/solweig/models/state.py:29-30`): nel giorno, non oltre.
  La spec chiede di usare `calculate()` su una serie perché un passo singolo non cattura
  l'inerzia (`specs/ground_temperature.md:179`), ma non chiede ore da scartare. `[V]`
- La stessa spec, alla riga 175, dice che `firstdaytime` si azzera «on first timestep after
  sunrise», e alle righe 187-190 che di notte «the TsWaveDelay model handles smooth transitions».
  Il codice fa un'altra cosa: rimette `firstdaytime = 1` a ogni passo di notte e azzera la storia
  dal secondo passo di notte (§1). Fa fede il codice. `[V]`
- `INVARIANTS.md:63-70` chiede soltanto che la serie sia strettamente crescente. `[V]`
- L'unica inizializzazione legata ai dati è quella dello schema 2026a, opzionale: parte dalla serie
  di `Ta` del primo giorno del file (`timeseries.py:590-606`). Con `ground_scheme: None` non
  conta. `[V]`
- Nella documentazione UMEP pubblicata non ho trovato una raccomandazione di giorni di avviamento
  per SOLWEIG. Ho fatto una sola ricerca web; il risultato più vicino è la pagina della fisica di
  monte, che è la stessa `specs/ground_temperature.md`. `[NON TROVATO]`

## 6. `timestep_dec` e `timeadd` con un buco o un passo irregolare

**Monte non rifiuta e non ricalcola: tratta ogni record come se distasse dal precedente quanto il
secondo dal primo.** `[V]`, `[M]`

- **Rifiuta** solo l'ordine non crescente o un istante doppio (`pysrc/solweig/api.py:488-500`). `[V]`
- `timestep_dec = (dt1 − dt0)/86400`, dai **primi due** record e per tutta la serie
  (`timeseries.py:462-471`); con un solo record, `1/24`. `timeadd` cresce di quel valore a ogni
  passo, qualunque sia il salto vero (`ground.rs:173-177`, `:202`). `[V]`
- Un salto diverso produce solo un avviso, sui totali in ore del riepilogo
  (`timeseries.py:34-61`). La docstring dice «Tmrt and the per-timestep outputs do not depend on
  it». Vale per il conteggio delle ore, non per l'inerzia, che usa il passo fisso. `[V]`
- Misura, un giorno a Bastia:
  - **buco dalle 11:00 alle 15:00** (record alle 10:00, poi alle 16:00): avviso «gaps from 1 h to
    6 h»; `timestep_dec` resta 60 min, `timeadd` resta 60 min anche attraverso il buco. Il passo
    delle 16:00 pesa la storia delle 10:00 come se fosse di un'ora prima (`w ≈ 0,25` invece di
    `exp(-33,27 · 6/24) ≈ 2·10⁻⁴`). `[M]`
  - **primo intervallo di 2 h, poi orario**: `timestep_dec = 120 min` per tutto il giorno, quindi
    `w = exp(-33,27/12) ≈ 0,06` a ogni ora invece di 0,25. Il calcolo non dà errori, solo
    l'avviso. `[M]`
- Il mezzo passo del sole non si ricava dai salti: è `Weather.timestep_minutes`, 60 di default
  (`weather.py:87`, `:169`). `[V]`
- Il SOLWEIG originale fa lo stesso: `timestepdec = dectime[1] − dectime[0]`
  (`Solweig_run.py:809-812`). `[V]`

## 7. Un Periodo senza ore di notte (sole di mezzanotte)

**Lo stato non si azzera mai, e ogni Corsa porta il proprio inizio per qualche ora.** `[V]`, `[M]`

- Nel codice non c'è nessun caso speciale. Senza ore con `sun_altitude <= 0`, dopo il primo passo
  `firstdaytime` resta 0 e `timeadd` resta `timestep_dec` (`computation.py:723-724`,
  `ground.rs:173-177`). La storia è una media mobile esponenziale senza fine, con peso 0,25 per ora
  sul passato. Valori limitati, nessun accumulo che cresca. `[V]`
- L'indice di serenità non viene mai riportato: ogni ora con radiazione ha il suo. `[V]`
- Misura a Svalbard, 78,22 N, 20-22/6, sereno: Corsa B parte con `firstdaytime = 1` e sostituisce
  la storia con il presente, A no. Lo scarto cala di un fattore 4 a ogni ora:

  | Ora di B | Tmrt | `tgmap1` (W/m²) | `tgout1` (K) |
  |---|---|---|---|
  | 1ª (21/6 01:00) | 0,397 K | 4,46 | 0,843 |
  | 2ª | 0,101 K | 1,11 | 0,211 |
  | 3ª | 0,025 K | 0,279 | 0,053 |
  | 4ª | 0,006 K | 0,070 | 0,013 |
  | 13ª | 3·10⁻⁵ K | 3·10⁻⁵ | 4·10⁻⁶ |

  Tmrt torna uguale bit per bit dalla 14ª ora (14:00). `tgmap1` porta un residuo di 3·10⁻⁵ W/m²
  fino alla 17ª ora, poi è uguale anche lui. `[M]`
- Il caso opposto, la notte polare: l'ora non è mai giorno, la storia si sostituisce a ogni passo,
  l'indice resta 1,0. Nessuna memoria. `[V]` sul codice, non misurato.

---

## Riferimenti

- **URL** https://github.com/UMEP-dev/solweig/tree/41840b74cd7c003f88dbde0c30fff736cb51bd5d · [V]
  **perché conta qui** è il codice vendorato da OMBRA, al pin; tutte le righe di monte citate
  sopra sono a quel commit
  **cosa se ne prende** la logica di stato da portare riga per riga (`computation.py:705-727`,
  `ground.rs:134-221`) e il riporto dell'indice (`timeseries.py:131-143`), da portare o da
  scartare per decisione esplicita
- **URL** https://github.com/UMEP-dev/UMEP-processing/tree/a7798db1698c29fa5ba667e082e7ad564c20e8e8/functions/SOLWEIGpython · [V]
  (estratti in `fonti/umep-processing-a7798db-stato-notturno.md`, sidecar accanto)
  **perché conta qui** è il SOLWEIG originale di cui monte dichiara la parità
  **cosa se ne prende** due punti in cui monte si scosta da UMEP, e che OMBRA eredita seguendo monte: il primo passo di notte con la
  memoria del giorno, e l'indice notturno che in UMEP si ricalcola a mezzanotte dal mattino dopo
  mentre monte lo riporta dalla sera prima
- **URL** https://pypi.org/project/solweig/0.1.0b101/ · [M]
  **perché conta qui** è il binario con cui sono state fatte le misure di §4, §6 e §7
  **cosa se ne prende** i numeri delle Corse A e B; con la stessa wheel, gli script in `fonti/`
  rifanno le misure
- **URL** https://climate.onebuilding.org/WMO_Region_6_Europe/ITA_Italy/UM_Umbria/ITA_UM_Umbria-Perugia-d-Assisi.AP.161810_TMYx.zip · [M]
  **perché conta qui** è un EPW reale vicino a Bastia, per stimare quanto spesso il riporto
  dell'indice morde
  **cosa se ne prende** l'11 % di giorni, e lo scarto massimo di 1,8-2,8 K nelle ore prima dell'alba

## Cosa resta incerto

- **Effetto del primo passo di notte di monte contro UMEP**, in kelvin. Non misurato. `[NON TROVATO]`
- **Sensibilità alla geometria.** Gli scarti di Tmrt in §4 vengono da un solo DSM, con un blocco.
  La correzione nuvolosa di `Ldown` pesa con il cielo visto, quindi in un canyon stretto lo scarto
  può cambiare. Non misurato. `[INF]`
- **Tramonto dopo le 23:00.** A latitudini alte d'estate l'ora 1 può essere il *primo* passo di
  notte. Una Corsa preceduta dal giorno prima porterebbe allora anche la memoria dello stato,
  oltre a quella dell'indice. Dedotto da §1, non misurato. `[INF]`
- **La parte Rust della wheel** viene dal tag `v0.1.0b101`: dedotto, non verificato con una build
  dal sorgente. `[INF]`

## Raccomandazione (non decisione)

Per lo stato termico un avviamento non serve: la notte lo azzera, e la misura lo conferma bit per
bit. Resta una sola memoria che attraversa la mezzanotte, l'indice di serenità della sera prima, e
riguarda solo le ore prima dell'alba del primo giorno del Periodo. Per tenerla non serve far girare
il Motore su ore in più: basta un numero, l'indice dell'ultima ora con sole e radiazione prima
dell'inizio del Periodo. Lo si ricava dai record EPW di quella sera, senza calcolare passi.

Le strade che vedo, da decidere nel ticket #21:

1. portare il riporto com'è e dare alla Corsa quel numero dalla sera prima nel file meteo;
2. portarlo com'è e lasciarlo a 1,0 all'inizio del Periodo, come fa monte, dichiarandolo nel
   Giornale;
3. seguire UMEP invece di monte: ricalcolare l'indice a mezzanotte dal mattino che comincia.

Con la 3 il Periodo non dipende più da nulla che stia prima del suo inizio. In cambio OMBRA si
scosta da monte proprio dove monte dichiara di seguire UMEP, e lo scostamento va scritto. Nel
Periodo che si avvolge sul file (`src/meteo.rs:675-681`, `si_avvolge`), la sera prima del 1/1
sarebbe il 31/12 dello stesso file, anche con la strada 1.
