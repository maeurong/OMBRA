# 04 — Dove vive lo strato Python da citare: `UMEP-dev/solweig` contro `UMEP-dev/umep-solweig`

Ticket: maeurong/OMBRA#54 (parte di #29, blocca #34). Dossier d'ingresso:
`research/catena-radiativa.md:204-230` (§5, tabella dello strato Python da portare).

**Fonti.** Due cloni, nella scratchpad della sessione, non nel repo:

- `UMEP-dev/solweig` al pin `41840b74cd7c003f88dbde0c30fff736cb51bd5d` (tag `v0.1.0b101`, 2026-11-11
  12:47 UTC), lo stesso di `vendor/solweig/PROVENIENZA.toml:9` (`git -C <clone> rev-parse HEAD`).
  La testa di `main` il giorno della lettura è già `35c8e5bc8caab09ce72948e5febeb45bd17f378c`
  (0.1.0b103); fra il pin e la testa nessun file delle parti confrontate cambia (§4).
- `UMEP-dev/umep-solweig` alla testa di `main`, `ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e`
  (2026-10-27 15:07 +0100, merge della PR #5), 42 commit, un tag `2026.1.0` su `be96b52`
  (2026-10-06), una release GitHub con lo stesso nome, una release PyPI `2026.0.1`.

Nel testo `S:` precede un percorso del clone di `solweig` a `41840b7`, `U:` uno del clone di
`umep-solweig` a `ff5bc5c`. I permalink completi stanno in «Riferimenti». Codice di OMBRA dal branch
`research/strato-python-monte`, HEAD `d876aae`. Le pagine web mutabili (issue, PR, metadati PyPI e
GitHub) sono catturate in `fonti/`.

Tag: `[V]` letto nella fonte · `[M]` misurato con comando · `[INF]` inferenza · `[NON TROVATO]`.

## Premesse verificate

- **«Le due copie» non sono due copie.** Il ticket chiede se «le due copie divergono». Non condividono
  storia né codice: `git merge-base` fra i due `main` non trova antenati (uscita 1), il primo commit di
  `solweig` è del 2025-06-27 (Gareth Simons), quello di `umep-solweig` del 2026-06-16 (Mael Philippe), e
  dei 700 blob di `S:` e 126 di `U:` ne hanno in comune 6, tutti dati di prova (`dsm.tif`, `cdsm.tif`,
  `dem.tif`, `lc.tif`, `wallheight.tif`) più un `__init__.py`. `[M]` Sono due implementazioni:
  `solweig` è un nucleo Rust con uno strato Python riscritto (`S:pysrc/solweig/`), `umep-solweig` è il
  SOLWEIG Python classico di UMEP, numpy più una variante torch (`U:src/umep_solweig/functions/SOLWEIGpython/`),
  senza Rust. Nessuna delle due è un fork dell'altra (`fork: false`, `parent: null` per entrambe,
  `fonti/github-repo-solweig-umep-solweig.md`). `[V]` La domanda resta rispondibile: per ogni parte
  ho cercato il punto di `U:` che fa lo stesso lavoro. Correzione marginale, non cambia il compito.
- **I `percorso:riga` del dossier sono riferiti a `02246ab`.** Riverificati a `41840b7`
  (`git -C <clone> show 02246ab:<file>` contro il checkout):

  | Parte | Dossier (`02246ab`) | Al pin `41840b7` |
  |---|---|---|
  | mezzo passo | `weather.py:150-151` | `S:pysrc/solweig/models/weather.py:169-170` |
  | clearness index e ripartizione | `weather.py:216-219 e seguenti` | `S:pysrc/solweig/models/weather.py:236-285` |
  | costanti delle pareti | `computation.py:291-295` | `S:pysrc/solweig/computation.py:323-327` |
  | `svfalfa` | `components/svf_resolution.py:95-100` | invariato, `:95-100` |
  | `findwalls` | `physics/wallalgorithms.py:10` | invariato, `:10-58` |
  | `HumanParams` | `models/config.py:286-293` | `S:pysrc/solweig/models/config.py:288-298` (classe a `:265`) |
  | stato iniziale | `models/state.py:62-82` | invariato |
  | `timestep_dec` | `timeseries.py:409-412` | `S:pysrc/solweig/timeseries.py:463-471` |
  | stato dopo il passo | `computation.py:637-657` | `S:pysrc/solweig/computation.py:705-727` |

  Il contenuto delle righe è lo stesso; è cambiata solo la numerazione. `[V]`
- **La nota della mappa ha una fonte, e sta in `solweig`.** La frase «l'implementazione di
  riferimento è ora `UMEP-dev/umep-solweig`» compare quasi alla lettera nel messaggio del commit
  `f9890c5` di `solweig` (2026-11-10): «The reference implementation is now UMEP-dev/umep-solweig».
  Vedi §2. `[V]`

---

## 1. Relazione fra i due repository

| | `UMEP-dev/solweig` | `UMEP-dev/umep-solweig` |
|---|---|---|
| Descrizione GitHub | «Rust implementations of umep functions» | «A python package to run the SOLWEIG algorithm» |
| Creato | 2025-06-27 | 2026-06-16 |
| Ultimo push | 2026-11-11 16:25 UTC | 2026-10-27 14:07 UTC |
| Fork / archiviato | no / no | no / no |
| Pacchetto PyPI | `solweig` 0.1.0b102, 77 release | `umep-solweig` 2026.0.1, una release (2026-10-06) |
| Sommario PyPI | «High-performance SOLWEIG urban microclimate model (Rust + Python)» | «SOLWEIG algorithm implementation for urban thermal and radiation simulation» |
| Commit per autore (`git shortlog -sn`) | Gareth Simons / Gareth / `songololo` 404, `dev` 3 | Mael Philippe 19, `biglimp` 19, Nils Wallenberg 3, Eliott Bridoux 1 |

Fonti: `fonti/github-repo-solweig-umep-solweig.md`, `fonti/pypi-solweig.md`,
`fonti/pypi-umep-solweig.md`, `git log` dei due cloni. `[V]`

Il rapporto passa per i test, non per il codice. `solweig` vendora quattro file di `umep-solweig` in
`S:tests/reference/umep_2026a/` («Source: https://github.com/UMEP-dev/umep-solweig … Commit: ff5bc5c
(main, fetched 2026-11-10)», intestazione di ciascun file) e li usa come riferimento di parità per lo
schema di suolo 2026a. La parità del modello di base invece la fa ancora contro il pacchetto pip
`umep`, cioè `UMEP-dev/umep-core` (`fonti/pypi-umep.md`): «The golden generator and the baseline parity
tests still import the older `umep` pip package» (`S:CLAUDE.md:240`). Il test di parità di un passo
intero importa `umep.functions.SOLWEIGpython.Solweig_2025a_calc_forprocessing`
(`S:tests/spec/test_end_to_end_parity.py:81`), ma deriva gli argomenti come li deriva
`Solweig_run.py` di `umep-solweig`, citandone le righe (`:166-168`, «Line numbers refer to
umep-solweig `functions/SOLWEIGpython/Solweig_run.py`»). `[V]`

Esiste anche un terzo albero, il plugin QGIS `UMEP-dev/UMEP-processing` (testa `a7798db`, 2026-09-24),
che contiene `Solweig_run.py` e il file di parametri `parametersforsolweig.json`. `[V]`, via
`gh search code` e `gh api …/contents/parametersforsolweig.json?ref=a7798db`. `umep-solweig` non
porta un file di parametri nel pacchetto: ce n'è uno solo, fra i dati di prova
(`U:tests/tests_data/solweig_parameters.json`). `[V]` Questo pesa sulle costanti (§3.3, §3.5, §3.7).

Una differenza strutturale: il runner di `umep-solweig` chiama `Solweig_2026a_calc`
(`U:…/SOLWEIGpython/Solweig_run.py:19`, `from . import Solweig_2026a_calc_forprocessing as so`; chiamata
a `:742`), anche con lo schema di suolo spento. `solweig` per il modello di base riproduce
`Solweig_2025a_calc`. Le due funzioni di monte differiscono in due righe a schema spento,
segnalate da Simons in `umep-solweig#13` e non ancora discusse (`fonti/umep-solweig-issue-13.md`). `[V]`

## 2. Quale delle due è l'implementazione di riferimento

**Per monte, `umep-solweig`.** Lo dicono entrambi i lati, con forza diversa.

- `solweig` lo dichiara di sé, al pin: «This package is an experimental, compatibility-focused
  implementation of the SOLWEIG model, not the reference implementation. The science is taken from
  [UMEP](https://github.com/UMEP-dev/umep-solweig), where it continues to be developed, and parity tests
  pin this package against the reference Python» (`S:README.md:12`; la stessa frase in
  `S:docs/index.md:6`). Il link punta a `umep-solweig`. `[V]`
- Le regole di sviluppo di `solweig` lo fissano: «The reference implementation is
  `UMEP-dev/umep-solweig` (since 2026-11)» (`S:CLAUDE.md:240`) e «UMEP is the precedent. The UMEP
  SOLWEIG implementation is the reference; its current home is `UMEP-dev/umep-solweig`. … When in
  doubt, preserve UMEP behaviour» (`S:CLAUDE.md:257`). `[V]`
- Il commit `f9890c5` (2026-11-10, release 0.1.0b97) lo annuncia: «The reference implementation is now
  UMEP-dev/umep-solweig», e la tabella delle release lo ripete: «now the reference implementation»
  (`S:VALIDATION.md:424`). `[V]`
- `umep-solweig` si dichiara ufficiale nei metadati: `description = "Official SOLWEIG algorithm
  implementation for urban thermal and radiation simulation"` (`U:pyproject.toml:8`), riga introdotta
  da `biglimp` nel commit `24b0a2c` (2026-10-06 12:49 +0200). `[V]` Che `biglimp` sia Fredrik
  Lindberg, autore dichiarato dei file di SOLWEIG, è un'inferenza: il profilo GitHub non ha nome, e
  l'email di `U:pyproject.toml:13` è `fredrikl@gvc.gu.se`. `[INF]`

**Limiti della conferma.**

- La frase che lo dice in chiaro sta in `solweig`, cioè nel progetto che si dichiara a valle. Simons
  ha chiesto conferma a monte il 2026-11-10 («I just want to double-check that this is the new
  reference repository», `umep-solweig#7`), e la issue è aperta senza risposta (zero commenti,
  `fonti/umep-solweig-issue-7.md`). `[V]`
- La sola release PyPI di `umep-solweig` (2026.0.1, caricata il 2026-10-06 08:39 UTC) porta ancora il
  sommario senza «Official», aggiunto circa due ore dopo (10:49 UTC) (`fonti/pypi-umep-solweig.md`, commit
  `24b0a2c`). `[V]`
- Il README di `umep-solweig` non dice di essere il riferimento né nomina `solweig` (`U:README.md`,
  letto per intero). `[V]`
- Se ne ricava che **implementazione di riferimento** significa riferimento **scientifico**:
  `solweig` è un porto che si misura contro `umep-solweig`, e lo strato che OMBRA porta in Rust è
  quello di `solweig`, perché il Rust vendorato consuma esattamente gli ingressi che
  `S:pysrc/solweig/` prepara. `[INF]`

## 3. Parte per parte

Riepilogo. *Coincide* vuol dire stesso risultato a parità di ingressi sul caso di OMBRA (EPW orario,
latitudine nord, griglia di copertura del suolo con le classi del file di parametri); *diverge* vuol
dire che almeno un ingresso realistico produce un numero diverso.

| Parte | Esito | `solweig` @ `41840b7` | `umep-solweig` @ `ff5bc5c` |
|---|---|---|---|
| Sole a metà passo | coincide la regola, divergono `altmax`, clamp dello zenit, passo singolo | `S:pysrc/solweig/models/weather.py:164-233`, `S:pysrc/solweig/timeseries.py:97-119` | `U:src/umep_solweig/util/SEBESOLWEIGCommonFiles/Solweig_v2015_metdata_noload.py:25-87` |
| Clearness index e ripartizione | coincide di giorno a latitudine nord; divergono il CI notturno, l'orizzonte, l'emisfero sud | `S:pysrc/solweig/models/weather.py:236-285`, `S:pysrc/solweig/timeseries.py:131-143`, `S:pysrc/solweig/physics/clearnessindex_2013b.py`, `S:pysrc/solweig/physics/diffusefraction.py` | `U:…/SOLWEIGpython/Solweig_2026a_calc_forprocessing.py:233-250`, `U:…/SOLWEIGpython/Solweig_run.py:636,663-682`, `U:…/SEBESOLWEIGCommonFiles/clearnessindex_2013b.py`, `U:…/SEBESOLWEIGCommonFiles/diffusefraction.py` |
| Costanti delle pareti | coincidono i valori; diverge la fonte (codice contro file di parametri) | `S:pysrc/solweig/computation.py:323-327` | `U:…/SOLWEIGpython/Solweig_run.py:74-75,485-487`, `U:…/SOLWEIGpython/Tgmaps_v1.py:35-37` |
| `svfalfa` | coincide (scarto 1,1e-14 rad) | `S:pysrc/solweig/components/svf_resolution.py:95-100` | `U:…/SOLWEIGpython/Solweig_run.py:250-253` |
| Proprietà del suolo | coincidono le classi 0, 1, 2, 5, 6, 7; divergono il segno di `Tstart` senza copertura, le classi 3, 4, 8-99 e ≥ 100, e l'ordine acqua/ciottolato | `S:pysrc/solweig/models/surface.py:1913-1986` | `U:…/SOLWEIGpython/Solweig_run.py:464-487`, `U:…/SOLWEIGpython/Tgmaps_v1.py:4-48` |
| `findwalls` | coincide l'algoritmo; diverge la soglia predefinita (1 m contro 3 m) e il tipo (float32 contro float64) | `S:pysrc/solweig/physics/wallalgorithms.py:10-58`, chiamata a `S:pysrc/solweig/models/surface.py:1163` | `U:src/umep_solweig/functions/wallalgorithms.py:14-50` (`findwalls_sp`), chiamata a `U:src/umep_solweig/algorithms/wall_heightaspect_algorithm.py:147`, soglia a `:56` |
| `HumanParams` | coincidono postura e fattori di vista; divergono `abs_l` (0,97 contro 0,95) e la statura PET (1,75 contro 1,80 m) | `S:pysrc/solweig/models/config.py:265-331`, `S:pysrc/solweig/constants.py:43-57` | `U:…/SOLWEIGpython/Solweig_run.py:78-79,316-345` + file di parametri |
| Stato iniziale | coincide; diverge solo `timestep_dec` con un passo solo | `S:pysrc/solweig/models/state.py:62-82`, `S:pysrc/solweig/timeseries.py:463-471` | `U:…/SOLWEIGpython/Solweig_run.py:393-398,620-626,636` |
| Stato dopo il passo | coincide | `S:pysrc/solweig/computation.py:705-727` | `U:…/SOLWEIGpython/Solweig_2026a_calc_forprocessing.py:579,701-702` (2025a: `:479,520-521`) |

`U:…/SOLWEIGpython/` sta per `U:src/umep_solweig/functions/SOLWEIGpython/`,
`U:…/SEBESOLWEIGCommonFiles/` per `U:src/umep_solweig/util/SEBESOLWEIGCommonFiles/`.

Il confronto dei moduli di fisica condivisi è fatto sull'albero sintattico, non sul testo, perché
`umep-solweig` è formattato con black e `solweig` no (lo dice `S:CLAUDE.md:240`). Script
`scratchpad/scripts/ast_cmp.py`: analizza i file con `ast.parse`, toglie docstring e annotazioni di
tipo, confronta funzione per funzione; non esegue codice di monte. `[M]` Risultati:

| Modulo `U:` contro `S:` | Esito |
|---|---|
| `sun_position.py` | 15 funzioni su 16 identiche; `sun_topocentric_zenith_angle_calculate` differisce solo perché `U:` estrae lo scalare con `[0]` (`U:…/SEBESOLWEIGCommonFiles/sun_position.py:1205,1221` contro `S:pysrc/solweig/physics/sun_position.py:1036,1048`), verificato con `diff -w` |
| `sun_distance.py` | differisce: `np.dot(c, x)` in `U:` contro `c * x` in `S:`, uguali per scalari `[INF]` |
| `daylen.py` | identica |
| `diffusefraction.py` | differisce, §3.2 |
| `clearnessindex_2013b.py` | differisce, §3.2 |
| `wallalgorithms.py` (`findwalls`) | differisce nella forma, §3.6 |

### 3.1 Sole a metà passo

**La regola coincide: il sole di un'etichetta H si calcola a H − passo/2.** `[V]`

- `S:pysrc/solweig/models/weather.py:169-170`: `half_timestep = timedelta(minutes=self.timestep_minutes / 2.0)`,
  `sun_time = self.datetime - half_timestep`. Il passo viene da `timestep_minutes`, predefinito 60
  (`:87`).
- `U:…/SEBESOLWEIGCommonFiles/Solweig_v2015_metdata_noload.py:30`: `halftimestepdec = (dectime[1] - dectime[0]) / 2.0`;
  `:71-74`: `YMDHM = YMD + H + M - half`. Il passo viene dalle prime due righe del file meteo.

**Divergenze.**

1. **Un passo solo.** `U:` mette il mezzo passo a zero se il file ha una riga (`:27-28`); `S:` sottrae
   comunque 30 minuti, perché `timestep_minutes` vale 60 se nessuno lo passa. `[V]`
2. **Clamp dello zenit.** `U:` porta a 89° ogni zenit fra 89° e 90° (`:83-84`, «Hopefully fixes weird
   values in Perez et al. when altitude < 1.0»), quindi un sole appena sorto ha altezza 1°. In `S:`
   non c'è nessun clamp equivalente: `grep` di `89.0` e `> 89` su `S:pysrc/` e `S:rust/src/` trova solo
   costanti dell'SPA, un test di `perez.rs` e una soglia di `shadowing.rs:222` a 89,5° per il sole allo
   zenit. `[M]` L'altezza entra in `diffusefraction` e nel clearness index (§3.2): le due copie
   differiscono nel primo grado sopra l'orizzonte. `[INF]` sull'effetto.
3. **`altmax`.** `S:` prende il massimo di 96 campioni a 15 minuti dalle 00:00
   (`S:pysrc/solweig/timeseries.py:97-119`, stessa forma a `S:pysrc/solweig/models/weather.py:198-233`),
   poi `max(…, 0.0)`. `U:` parte dalle 10:15 e sale a passi di 15 minuti finché l'altezza cresce
   (`Solweig_v2015_metdata_noload.py:53-69`, `HM = timedelta(days=(60 * 10) / 1440.0 + fifteen)`), e lo
   ricalcola solo alla prima riga e alle righe di mezzanotte. I due campionamenti cadono sugli stessi
   istanti (10:15 è il 41° quarto d'ora); se il mezzogiorno solare cade dopo le 10:15 e l'altezza
   ha un solo massimo, il risultato è lo stesso. A Bastia, longitudine 12,6° E con UTC+1, il
   mezzogiorno solare cade verso le 12:10 ora solare. `[INF]` Non misurato: eseguire lo `sun_position`
   di monte nella sessione è stato negato (§5).
4. **Secondi.** `U:` passa `sec = 0` all'SPA (`:32`) e tronca i secondi del mezzo passo; `S:` passa
   `sun_time.second` (`weather.py:179`). Con passo orario non cambia nulla; con un passo di 15 minuti il
   sole di `U:` cade 30 secondi prima. `[V]` sul codice, `[INF]` sull'effetto.
5. **EPW.** Il runner di `U:` legge solo il formato meteo UMEP con `np.loadtxt`
   (`Solweig_run.py:267-269`); nessun lettore EPW in `U:src/umep_solweig/functions/` (`grep -ri epw`
   trova solo una chiave di configurazione in `U:src/umep_solweig/util/umep_solweig_export_component.py:95-97`). `[M]`
   Il percorso EPW di `S:` (`S:pysrc/solweig/io_epw.py`, già discusso in `02-sole-meta-passo.md`) non
   ha una controparte in `U:`.

L'SPA è la stessa (tabella sopra). `[M]`

### 3.2 Clearness index e ripartizione diretta/diffusa

**Di giorno, a latitudine nord, con il sole sopra 1°, coincidono.** Entrambe chiamano
`clearnessindex_2013b` e poi, se manca la misura della diffusa, `diffusefraction` di Reindl et al.
(`S:pysrc/solweig/models/weather.py:259-277` contro `U:…/SOLWEIGpython/Solweig_2026a_calc_forprocessing.py:236-250`).
Le costanti numeriche dei due moduli coincidono (`diff`: solo formattazione, guardie e nomi). `[V]`

**Divergenze.**

1. **CI notturno.** `S:` porta l'ultimo CI diurno su tutta la notte (`S:pysrc/solweig/timeseries.py:131-143`),
   con il commento «UMEP Python does not recompute CI when the sun is below the horizon; instead the
   last daytime CI value persists». In `U:` alla riga di mezzanotte il runner tenta di ricalcolare il CI
   del mattino dopo (`U:…/SOLWEIGpython/Solweig_run.py:663-682`, «Nocturnal cloudfraction from Offerle
   et al. 2003»), ma il ramo è condizionato a `daylines.__len__() > 1`, e `daylines` è la tupla di
   `np.where` su un vettore 1-D, di lunghezza 1: il ramo non si prende mai, e a mezzanotte il CI torna
   1,0 (`:681-682`). `[M]` (`scratchpad/scripts/misure.py`, che riscrive la riga a mano:
   `len(np.where(1-D)): 1 | ramo … preso: False`). Effetto: dopo mezzanotte `U:` ha CI 1,0, `S:` ha
   l'ultimo CI della sera; se questo è sotto 0,95 la correzione di `Ldown` per cielo coperto
   (`S:rust/src/pipeline.rs:781-782`, `U:…/Solweig_2026a_calc_forprocessing.py:818-819`) scatta in `S:` e
   non in `U:`. `[INF]` sull'effetto.
2. **Clamp del CI.** `U:` porta a 1 ogni CI maggiore di 1 o infinito (`Solweig_2026a_calc_forprocessing.py:239-240`).
   `S:` non lo fa (`weather.py:272`). Nelle righe lette di `S:rust/src/pipeline.rs` il CI compare solo
   nel test `ci < 0.95` (`:781-782`, `:1445-1447`), dove un CI sopra 1 e un CI uguale a 1 si
   comportano allo stesso modo. `[V]` sulle righe lette, `[INF]` che non ci siano altri usi.
3. **Sole all'orizzonte.** `S:` restituisce CI infinito e `I0 = 0` se l'altezza è sotto 0,01°
   (`S:pysrc/solweig/physics/clearnessindex_2013b.py:43-45,102-111`) e mette la diretta a zero se
   `sin(altezza) < 0,01`, cioè sotto 0,57° (`S:pysrc/solweig/physics/diffusefraction.py:37-38`). `U:`
   non ha guardie (`U:…/SEBESOLWEIGCommonFiles/diffusefraction.py:50`) ma non ci arriva mai, perché
   lo zenit è già portato a 89° (§3.1). Fra 0° e 1° di altezza le due copie danno diretta e diffusa
   diverse. `[V]` sul codice, `[INF]` sull'effetto.
4. **Emisfero sud.** `S:` sceglie i coefficienti di Crawford e Duchon su `abs(latitudine)`
   (`S:pysrc/solweig/physics/clearnessindex_2013b.py:55`); `U:` sulla latitudine col segno
   (`U:…/SEBESOLWEIGCommonFiles/clearnessindex_2013b.py:37-54`), e ogni latitudine negativa cade nella
   prima fascia. Non tocca Bastia. `[V]`
5. **Unità della pressione.** `U:` moltiplica per 10 (`clearnessindex_2013b.py:25`, col commento «Convert
   from hPa to millibars», che non torna con il ×10); `S:` usa il valore così com'è, in hPa
   (`S:…/clearnessindex_2013b.py:34`, con la nota a `:25-31`). Le due righe danno lo stesso numero
   se `U:` riceve kPa, come nel formato meteo UMEP (colonna 12, `Solweig_run.py:284`). `[V]` sul
   codice, `[INF]` sull'unità del file UMEP: non ho trovato in `U:` un documento che la fissi.
   `[NON TROVATO]`
6. **Notte senza radiazione.** `S:` calcola la ripartizione solo se `sun_altitude > 0` **e**
   `global_rad > 0` (`weather.py:259`), `U:` solo con `altitude > 0` (`:233`). Con globale nulla
   `diffusefraction` di `U:` dà comunque diretta e diffusa nulle. `[INF]`

### 3.3 Costanti delle pareti

**I valori coincidono:** `tgk 0.37`, `tstart −3.41`, `tmaxlst 15.0`, `albedo 0.20`, `emis 0.90`.

- `S:pysrc/solweig/computation.py:323-327`, scritti nel codice. `tgk`, `tstart` e `tmaxlst` si possono
  sostituire con un materiale di parete (`:328-337`); albedo ed emissività no (la stessa funzione, a
  `:786`, lo chiama «the unconditional 0.20»). `[V]`
- `U:` li legge dal file di parametri: albedo ed emissività a `Solweig_run.py:74-75`
  (`param["Albedo"]["Effective"]["Value"]["Walls"]`, `param["Emissivity"]["Value"]["Walls"]`), le altre
  tre a `:485-487` senza copertura del suolo e a `Tgmaps_v1.py:35-37` con la copertura. `Tstart` della
  parete non è negato su nessuno dei due rami. `[V]`
- I valori `Walls` sono gli stessi in tre file: `U:tests/tests_data/solweig_parameters.json`,
  `parametersforsolweig.json` di `UMEP-processing` @ `a7798db` (radice e `processor/`), e
  `S:pysrc/solweig/parametersforsolweig.json`. `[M]` (confronto chiave per chiave dei tre JSON con uno
  script Python in sessione).

La divergenza sta nella fonte: in `U:` le costanti sono un file che l'utente può cambiare, in `S:`
sono codice. Per una citazione di OMBRA la riga di `S:` dice dove il valore vive nel codice portato;
la voce `Walls` del file di parametri dice da dove viene il valore. `[INF]`

### 3.4 `svfalfa`

**Coincide.**

- `U:…/SOLWEIGpython/Solweig_run.py:250-253`: `tmp = svf + svfveg - 1.0`, `tmp[tmp < 0.0] = 0.0`,
  `svfalfa = np.arcsin(np.exp((np.log((1.0 - tmp)) / 2.0)))` («matlab crazyness around 0»).
- `S:pysrc/solweig/components/svf_resolution.py:98-100`: `tmp = np.clip(svf + svf_veg - 1.0, 0.0, 1.0)`,
  `svfalfa = np.arcsin(np.sqrt(np.clip(1.0 - tmp, 0.0, 1.0)))`.

`exp(log(x)/2)` e `sqrt(x)` sono la stessa funzione per x ≥ 0; il clamp superiore di `S:` scatta solo
se `svf + svf_veg > 2`, impossibile con valori in [0, 1]. Su 10⁶ coppie uniformi in [0, 1] lo scarto
massimo è **1,1 × 10⁻¹⁴ rad**; con `svf = svf_veg = 1` entrambe danno 0 (la forma di `U:` passa per
`log(0) = −inf` con un avviso). `[M]` (`python3 -I scratchpad/scripts/misure.py`, che riscrive a mano le
due espressioni.)

Senza vegetazione `U:` pone tutti gli SVF di vegetazione a 1 (`Solweig_run.py:238-248`). `[V]`

### 3.5 Proprietà del suolo

**Con griglia di copertura, le classi 0, 1, 2, 5, 6, 7 coincidono.** La tabella di
`S:pysrc/solweig/models/surface.py:1959-1966` riporta per le sei classi gli stessi albedo, emissività,
`Ts_deg`, `Tstart` e `TmaxLST` delle voci di `parametersforsolweig.json` di `UMEP-processing`, che
`Tgmaps_v1` legge (`U:…/SOLWEIGpython/Tgmaps_v1.py:17-33`). `[V]` (confronto a mano delle sei righe
con il JSON.) Confronto chiave per chiave fra `S:pysrc/solweig/parametersforsolweig.json` e i due file di
`UMEP-processing` @ `a7798db`: sulle chiavi comuni differiscono solo `Tmrt_params.absL` (0,97 contro
0,95) e `Thermal_conductivity` delle classi di suolo (−9999 in `S:`, valori dello schema 2026a in
`UMEP-processing`); albedo, emissività, `Ts_deg`, `Tstart` e `TmaxLST` coincidono tutti. `[M]`

**Divergenze.**

1. **Segno di `Tstart` senza copertura.** `U:` scrive `Tstart = Knight - param["Tstart"]["Value"]["Cobble_stone_2014a"]`
   (`Solweig_run.py:478`; la issue la cita come l.483, cinque righe più in basso del clone a `ff5bc5c` che dichiara di leggere), quindi +3,41; con la copertura `Tgmaps_v1` usa il valore così com'è, −3,41.
   `S:` usa −3,41 su entrambi i rami (`surface.py:1947`). Simons l'ha segnalato a monte come probabile
   errore, con l'effetto misurato su una scena sintetica (Lup fino a 39,3 W/m², Tmrt fino a 2,4 °C),
   e `solweig` lo tiene come scostamento voluto finché monte non decide (`umep-solweig#14`,
   `fonti/umep-solweig-issue-14.md`; `S:tests/spec/test_end_to_end_parity.py:51-54`). `[V]`
2. **Albedo del suolo senza copertura.** `S:` usa 0,20 (`surface.py:1941-1944`, «it was 0.15 up to
   0.1.0b99»), lo stesso valore di `Cobble_stone_2014a` nel JSON di `UMEP-processing` e in
   `S:pysrc/solweig/parametersforsolweig.json`. `U:` lo legge dal file di parametri che riceve, e
   l'unico file presente nel repo, quello di prova, porta **0,15** (`U:tests/tests_data/solweig_parameters.json`,
   `Albedo.Effective.Value.Cobble_stone_2014a`). `[M]` (stesso confronto dei JSON di §3.3.) Il valore
   di `U:` dipende quindi dal file che l'utente gli dà.
3. **Classi 3 e 4.** In `U:` mancano da `Names` e `Tgmaps_v1` solleva `KeyError`; in `S:` prendono i
   valori del ciottolato (`surface.py:1962-1963`). `[V]` sul codice di `S:`; per `U:` lo conferma
   Simons nella PR aperta `umep-solweig#15` («classes 3 and 4 still raise `KeyError`»,
   `fonti/umep-solweig-pr-15.md`). Non eseguito in sessione.
4. **Classi ≥ 100.** `U:` le porta a 2, tetti (`Tgmaps_v1.py:8`); `S:` le lascia ai valori di
   riempimento, che per l'albedo valgono **0,15** (`surface.py:1970`), non 0,20 né 0,18. `[V]`
5. **Classi da 8 a 99.** In `U:` restano col proprio numero come valore del parametro («class 9 gets
   albedo 9», PR #15); in `S:` prendono i valori di riempimento. `[V]` su `S:`, `[V]` sulla PR per `U:`.
6. **Acqua e ciottolato nella stessa griglia.** `Tgmaps_v1` sostituisce i valori in posto e scorre la
   classe di ogni pixel, non le classi uniche (`:9-10`): se un pixel d'acqua precede in ordine di raster
   un pixel di classe 0, l'acqua prende `TgK` e `Tstart` del ciottolato, perché i suoi valori, 0,0,
   coincidono con l'identificativo 0. La PR #15 lo documenta con due griglie d'esempio. `S:` usa
   maschere sulla griglia delle classi (`surface.py:1976-1984`) e non ha il difetto. `[V]` sul codice,
   `[V]` sulla PR per l'esempio.

La gestione dell'acqua nel nucleo (codice 3 contro 7, `umep-solweig#4`, `fonti/umep-solweig-issue-4.md`)
sta nel Rust e in `sunonsurface_2018a`, fuori dallo strato Python confrontato qui. Non verificata.
`[NON TROVATO]`

### 3.6 `findwalls`

**L'algoritmo coincide:** per ogni pixel, massimo dei quattro vicini cardinali meno il pixel stesso,
azzerato sotto la soglia, bordo a zero.

- `S:pysrc/solweig/physics/wallalgorithms.py:10-58`, vettoriale con `np.maximum` a coppie, uscita
  `float32`.
- Il runner di `U:` **non chiama `findwalls`**: legge altezza e orientamento dei muri da raster su disco
  (`Solweig_run.py:255-260`), scritti dallo strumento `wall_heightaspect_algorithm`, che usa
  `findwalls_sp` (`U:src/umep_solweig/algorithms/wall_heightaspect_algorithm.py:147`): `maximum_filter`
  di scipy con impronta a croce su un raster imbottito ai bordi (`U:src/umep_solweig/functions/wallalgorithms.py:14-50`),
  uscita `float64`. La `findwalls` a doppio ciclo (`:53-85`) esiste ma nessun file di `U:src/` la
  chiama (`grep 'findwalls('`, uscita 1). `[M]`

**Divergenze.**

1. **Soglia predefinita.** `S:` calcola i muri con soglia **1 m** (`S:pysrc/solweig/models/surface.py:1163`;
   predefinito anche in `S:pysrc/solweig/walls.py:33`); lo strumento di `U:` ha predefinito **3 m**
   (`wall_heightaspect_algorithm.py:56`). Un gradino fra 1 e 3 m è muro in `S:` e non in `U:`. `[V]`
2. **Tipo.** `float32` in `S:`, `float64` in `U:`. `[V]`

Uguaglianza numerica delle tre forme non misurata: l'esecuzione delle funzioni di monte nella sessione
è stata negata (§5). L'equivalenza sull'interno del raster la ricavo dalla lettura. `[INF]`

### 3.7 `HumanParams`

| Parametro | `S:` | `U:` (file di parametri) | Esito |
|---|---|---|---|
| postura predefinita | `standing` (`config.py:288`) | `Standing` (`Tmrt_params.posture`) | coincide |
| `abs_k` | 0,70 (`:289`) | 0,70 | coincide |
| `abs_l` | **0,97** (`:290`) | **0,95** (prova di `U:` e i due JSON di `UMEP-processing`) | **diverge** |
| altezza del baricentro, in piedi / seduto | 1,1 / 0,75 (`constants.py:56-57`, `config.py:321-331`) | 1,1 / 0,75 (`Posture.*.height`, letta a `Solweig_run.py:331,337`) | coincide |
| `Fside`, `Fup`, `Fcyl` in piedi | 0,22 / 0,06 / 0,28 (`constants.py:43-45`) | 0,22 / 0,06 / 0,28 | coincide |
| `Fside`, `Fup` seduto | 1/6 (`constants.py:49-50`) | 0,166666 | scarto 6,7 × 10⁻⁷ |
| `Fcyl` seduto | 0,20 (`:51`) | 0,2 | coincide |
| età, peso, attività, abbigliamento PET | 35, 75 kg, 80 W, 0,9 clo (`config.py:293-298`) | 35, 75, 80, 0,9 | coincide |
| statura PET | **1,75 m** (`:295`) | **180** (cm, `PET_settings.Height`) | **diverge** |

`[V]` su codice e JSON. `abs_l = 0,97` è una scelta dichiarata di `solweig`, che cita ISO 7726
(`S:specs/tmrt.md:58`, `S:pysrc/solweig/data/default_params.json:5`). `[V]`

Due differenze di comportamento, documentate da `solweig` stesso: il modello a scatola si accende per
la postura seduta (`cyl = human.is_standing`), mentre in `U:` `cyl` è un'impostazione separata
(`S:tests/spec/test_end_to_end_parity.py:55-57`). `[V]` E il GVF di `S:` usa l'altezza della postura
solo dalla 0.1.0b100 (commit `2ff52fb`, «as in UMEP»), quindi dal pin `c31c575` in poi questo numero è
cambiato. `[V]`

### 3.8 Stato iniziale

**Coincide**, salvo il passo singolo.

| Grandezza | `S:` | `U:` |
|---|---|---|
| `tgmap1`, `_e`, `_s`, `_w`, `_n` | zeri (`S:pysrc/solweig/models/state.py:72-79`) | zeri (`Solweig_run.py:394-398`) |
| `firstdaytime` | 1,0 (`state.py:80`) | 1,0 (`:626`) |
| `timeadd` | 0,0 (`:81`) | 0,0 (`:625`) |
| `timestep_dec` | 0,0 in `ThermalState.initial` (`:82`), poi differenza delle prime due etichette, **1/24 con un passo solo** (`S:pysrc/solweig/timeseries.py:463-471`) | differenza delle prime due righe, **0 con un passo solo** (`:620-624`) |
| CI all'inizio | 1,0 (`timeseries.py:138`) | 1,0 (`:636`, «If metfile starts at night») |
| `tgout1` | zeri (`state.py:79`) | nel runner 2026a non c'è: lo stato del ritardo per la temperatura del suolo puntuale è `Tgdiff` (`umep-solweig#13`, punto 2) |

`[V]`

### 3.9 Aggiornamento dello stato dopo il passo

**Coincide.**

- `S:pysrc/solweig/computation.py:705-727`: `timeadd` e `tgmap*` tornano dal Rust; poi
  `if weather.is_daytime: firstdaytime = 0.0`, altrimenti `firstdaytime = 1.0` e `timeadd = 0.0`.
  `is_daytime` è `sun_altitude > 0` (`S:pysrc/solweig/models/weather.py:290-292`).
- `U:…/SOLWEIGpython/Solweig_2026a_calc_forprocessing.py`: ramo diurno con `if altitude > 0` (`:233`),
  `firstdaytime = 0` a fine ramo (`:579`); ramo notturno con `timeadd = 0`, `firstdaytime = 1`
  (`:701-702`). Identico in `Solweig_2025a_calc_forprocessing.py:219,479,520-521`. Il runner
  riprende `firstdaytime`, `timeadd` e `Tgmap1*` dalla tupla restituita (`Solweig_run.py:700-718`).

Il criterio giorno/notte è lo stesso (altezza > 0); il clamp dello zenit di `U:` (§3.1) alza l'altezza
a 1° ma non ne cambia il segno. `[V]` Il ritardo `TsWaveDelay_2015a` che produce `timeadd`
(`U:…/SOLWEIGpython/TsWaveDelay_2015a.py:4-27`) non l'ho confrontato con il Rust di `S:`: è parte del
passo fuso, coperto dal banco di parità, non dello strato Python.

### 3.10 Fuori elenco, ma sullo stesso confine

- **Maschera degli edifici** (riga del dossier `components/gvf.py:39`). Senza copertura del suolo
  `S:` usava una dilatazione di 25 m attorno ai muri, corretta nella 0.1.0b100 in `dsm - dem >= 2`, come
  `U:` (`Solweig_run.py:409-412`; `S:tests/spec/test_end_to_end_parity.py:39-43`;
  `S:VALIDATION.md:428`). Con copertura `U:` mette a 1 le classi 3-7 e a 0 la classe 2
  (`Solweig_run.py:401-408`) e lascia al loro valore le classi 0 e 1, quindi un pixel di classe 0 conta come edificio; `S:` parte da una griglia di uni e mette a 0 solo la classe 2 (`S:pysrc/solweig/components/gvf.py:81-84`). Sulla classe 0 le due maschere divergono. `[V]` sul codice, `[INF]` che la classe 0 compaia in griglie reali: il formato UMEP usa 1-7.
- **Perez.** `solweig` tiene di proposito le forme pubblicate di chiarezza del cielo e massa d'aria,
  che in `U:` hanno le parentesi spostate (`S:CLAUDE.md:240`; `umep-solweig#8`). È nel Rust
  vendorato (`perez.rs`), non nello strato Python. `[V]`

## 4. Cosa resta incerto

- **Monte non ha confermato.** La dichiarazione esplicita che `umep-solweig` sia il riferimento sta in
  `solweig`; a monte c'è il «Official» di `pyproject.toml` e una domanda di conferma senza risposta
  (`umep-solweig#7`). `[V]`
- **`umep-solweig` è giovane e si muove poco.** 42 commit, ultimo il 2026-10-27, una release; quattro
  issue e tre PR di Simons aperte dal 10 e 11 novembre (#8, #12, #13, #14; PR #9, #10, #11, #15), nessuna
  ancora discussa. Due di queste (#14, PR #15) toccano parti di questo elenco: se monte le accetta,
  `U:` si sposta verso `S:`. `[V]`
- **Le costanti di `U:` non hanno una casa nel pacchetto.** Vivono nel file di parametri che l'utente
  passa, e l'unico nel repo è un dato di prova che su albedo del ciottolato (0,15) non concorda con il
  file di `UMEP-processing` (0,20). Quale file sia «il» riferimento per i valori non lo dice nessuna
  fonte letta. `[NON TROVATO]`
- **Il pin è già indietro.** La testa di `solweig` è `35c8e5b` (0.1.0b103), due release dopo il pin.
  `git diff --stat 41840b7 35c8e5b` su `computation.py`, `models/weather.py`, `models/state.py`,
  `models/config.py`, `components/`, `physics/` non dà output; `surface.py` e `timeseries.py` cambiano,
  ma nessuna riga aggiunta o tolta contiene `albedo`, `tstart`, `tgk`, `timestep_dec`, `clearness`,
  `firstdaytime`, `altmax` o `half`. `[M]`
- **Equivalenze non misurate.** `altmax` (§3.1) e `findwalls` (§3.6) le ricavo dalla lettura: §5.

## 5. Cosa non ho potuto fare

L'esecuzione delle funzioni di monte (`sun_position`, `findwalls`, `findwalls_sp`, `Tgmaps_v1`,
caricate dai cloni in un ambiente virtuale della scratchpad) è stata negata dal classificatore dei
permessi come esecuzione di codice esterno. Non l'ho aggirata. Le misure `[M]` di questa ricerca sono
quindi solo: confronti di testo, di albero sintattico e di JSON, `grep`, `git`, e le due espressioni
riscritte a mano in `scratchpad/scripts/misure.py`. Con il permesso, lo stesso script misurerebbe
`altmax` su un anno a Bastia e lo scarto di `findwalls` su un DSM rettangolare.

## 6. Raccomandazione

**Raccomandazione, non decisione.** Spetta a #34.

1. **Citare `solweig` @ `41840b7` come riga primaria** per ogni costante e formula dello strato
   portato. È il codice che OMBRA porta davvero, e il Rust vendorato consuma i valori che quello
   strato prepara: citare `umep-solweig` come fonte primaria vorrebbe dire citare un codice diverso da
   quello portato, in nove parti su nove con forma diversa e in sei (sole, clearness index, suolo, `findwalls`,
   `HumanParams`, stato iniziale) con almeno un ingresso che dà numeri diversi.
2. **Affiancare la riga di `umep-solweig` @ `ff5bc5c`** dove l'esito è *coincide*, come riferimento
   scientifico. È ciò che `solweig` stesso dichiara di seguire (`S:README.md:12`, `S:CLAUDE.md:257`), e
   regge la citabilità meglio di un porto sperimentale.
3. **Dove divergono, la citazione deve dirlo** e nominare il valore scelto. Le divergenze che toccano
   il caso di Bastia sono: `abs_l` 0,97 contro 0,95; statura PET 1,75 contro 1,80 m; segno di
   `Tstart` senza copertura; CI dopo mezzanotte; soglia dei muri 1 contro 3 m; `altmax` e clamp dello
   zenit nel primo grado sopra l'orizzonte; classi di copertura fuori dalle sei definite. Per ciascuna
   #34 deve scegliere quale delle due seguire; la scelta va nel Giornale.
4. **Per i valori delle costanti, citare anche la voce del file di parametri** (`Walls`,
   `Cobble_stone_2014a`, …) di `UMEP-processing` @ `a7798db`, perché in `umep-solweig` non esiste un
   file di parametri del pacchetto.

---

## Riferimenti

- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/README.md#L12 · [V]
  **perché conta qui** è la dichiarazione di `solweig` su sé stesso, al pin vendorato
  **cosa se ne prende** «not the reference implementation … The science is taken from UMEP (umep-solweig)»: risposta alla domanda 1
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/CLAUDE.md#L240-L257 · [V]
  **perché conta qui** regole di sviluppo di `solweig`: riferimento, file vendorati, scostamenti voluti
  **cosa se ne prende** «reference implementation is `UMEP-dev/umep-solweig` (since 2026-11)»; parità di base ancora sul pip `umep`; Perez tenuto di proposito; confronto per AST
- **URL** https://github.com/UMEP-dev/solweig/commit/f9890c5a6c4e2a9bd045f192635c083faa097d8d · [V]
  **perché conta qui** commit che annuncia il cambio di riferimento ed è la fonte probabile della nota della mappa
  **cosa se ne prende** data del cambio (2026-11-10, 0.1.0b97) e i due allineamenti dello schema 2026a
- **URL** https://github.com/UMEP-dev/solweig/commit/9f2c6364083991ccf2866fdd8eac60f6fc1c189e · [V]
  **perché conta qui** introduce il test di parità di un passo intero, che deriva gli ingressi come `Solweig_run.py` di `umep-solweig`
  **cosa se ne prende** albedo 0,15→0,20, maschera degli edifici, segno di `Tstart`, `cyl` per la postura seduta (§3.5, §3.7, §3.10)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/tests/spec/test_end_to_end_parity.py#L1-L62 · [V]
  **perché conta qui** è il solo punto in cui `solweig` mette le sue righe a fianco di quelle di `umep-solweig`
  **cosa se ne prende** l'elenco degli scostamenti misurati e la mappatura delle righe di `Solweig_run.py`
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/VALIDATION.md#L424-L428 · [V]
  **perché conta qui** registro delle release 0.1.0b97 e 0.1.0b100 con gli allineamenti a `umep-solweig`
  **cosa se ne prende** cosa è cambiato fra `c31c575` e `41840b7` nelle parti di questo elenco
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/models/weather.py#L164-L292 · [V]
  **perché conta qui** sole a metà passo, `altmax`, clearness index, ripartizione, criterio giorno/notte di `S:`
  **cosa se ne prende** le righe primarie di §3.1, §3.2, §3.9
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/timeseries.py#L97-L143 · [V]
  **perché conta qui** `altmax` precalcolato e CI portato avanti di notte
  **cosa se ne prende** la divergenza sul CI notturno (§3.2.1) e il campionamento di `altmax` (§3.1.3)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/timeseries.py#L463-L471 · [V]
  **perché conta qui** `timestep_dec` di `S:`
  **cosa se ne prende** 1/24 con un passo solo (§3.8)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/computation.py#L323-L337 · [V]
  **perché conta qui** costanti delle pareti nel codice di `S:`
  **cosa se ne prende** i cinque valori e quali si possono sostituire (§3.3)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/computation.py#L705-L727 · [V]
  **perché conta qui** aggiornamento dello stato dopo il passo
  **cosa se ne prende** la riga primaria di §3.9, ex `computation.py:637-657` a `02246ab`
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/components/svf_resolution.py#L95-L100 · [V]
  **perché conta qui** `svfalfa` di `S:`
  **cosa se ne prende** la forma con `sqrt` e clamp (§3.4)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/models/surface.py#L1913-L1986 · [V]
  **perché conta qui** proprietà del suolo per classe in `S:`
  **cosa se ne prende** tabella delle classi, valori senza copertura, riempimento a 0,15 (§3.5)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/physics/wallalgorithms.py#L10-L58 · [V]
  **perché conta qui** `findwalls` di `S:`
  **cosa se ne prende** forma vettoriale e uscita `float32` (§3.6)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/models/surface.py#L1163 · [V]
  **perché conta qui** dove `S:` chiama `findwalls`
  **cosa se ne prende** soglia 1 m (§3.6.1)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/models/config.py#L265-L331 · [V]
  **perché conta qui** `HumanParams` di `S:`
  **cosa se ne prende** `abs_l` 0,97, statura 1,75 m, altezza della postura (§3.7)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/models/state.py#L62-L82 · [V]
  **perché conta qui** stato iniziale di `S:`
  **cosa se ne prende** zeri, `firstdaytime` 1, `timeadd` 0 (§3.8)
- **URL** https://github.com/UMEP-dev/solweig/blob/41840b74cd7c003f88dbde0c30fff736cb51bd5d/pysrc/solweig/physics/clearnessindex_2013b.py#L11-L116 · [V]
  **perché conta qui** clearness index di `S:`, diverso da quello di `U:` in quattro punti
  **cosa se ne prende** pressione in hPa, `abs(latitudine)`, uscite anticipate all'orizzonte (§3.2)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/pyproject.toml#L8-L13 · [V]
  **perché conta qui** metadati del pacchetto che si dichiara ufficiale
  **cosa se ne prende** «Official SOLWEIG algorithm implementation», introdotto da `biglimp` in `24b0a2c` (§2)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/src/umep_solweig/functions/SOLWEIGpython/Solweig_run.py · [V]
  **perché conta qui** il runner di riferimento: è lui che deriva gli ingressi del passo
  **cosa se ne prende** righe 19, 74-79, 250-260, 316-345, 393-412, 464-487, 620-636, 663-682, 700-742 (§3)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/src/umep_solweig/functions/SOLWEIGpython/Solweig_2026a_calc_forprocessing.py#L233-L250 · [V]
  **perché conta qui** clearness index e ripartizione nel passo che il runner chiama
  **cosa se ne prende** clamp del CI a 1 (§3.2.2); `firstdaytime` e `timeadd` a `:579,701-702` (§3.9)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/src/umep_solweig/util/SEBESOLWEIGCommonFiles/Solweig_v2015_metdata_noload.py#L25-L87 · [V]
  **perché conta qui** sole a metà passo, `altmax` e clamp dello zenit in `U:`
  **cosa se ne prende** le righe di §3.1
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/src/umep_solweig/functions/SOLWEIGpython/Tgmaps_v1.py#L4-L48 · [V]
  **perché conta qui** proprietà del suolo per classe in `U:`
  **cosa se ne prende** mappatura delle classi ≥ 100 a 2, sostituzione in posto (§3.5)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/src/umep_solweig/functions/wallalgorithms.py#L14-L85 · [V]
  **perché conta qui** `findwalls_sp` e `findwalls` di `U:`
  **cosa se ne prende** stesso algoritmo, `float64` (§3.6)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/src/umep_solweig/algorithms/wall_heightaspect_algorithm.py#L56-L147 · [V]
  **perché conta qui** lo strumento che in `U:` produce i raster dei muri
  **cosa se ne prende** soglia predefinita 3 m e chiamata a `findwalls_sp` (§3.6.1)
- **URL** https://github.com/UMEP-dev/umep-solweig/blob/ff5bc5c1dc2512740e02fc5ae81d23cd2bd5e91e/tests/tests_data/solweig_parameters.json · [V]
  **perché conta qui** l'unico file di parametri nel repo di `U:`
  **cosa se ne prende** `absL` 0,95, albedo del ciottolato 0,15, valori `Walls` (§3.3, §3.5, §3.7)
- **URL** https://github.com/UMEP-dev/UMEP-processing/blob/a7798db1698c29fa5ba667e082e7ad564c20e8e8/parametersforsolweig.json · [V]
  **perché conta qui** file di parametri del plugin QGIS, l'altro albero UMEP che porta `Solweig_run.py`
  **cosa se ne prende** albedo del ciottolato 0,20, `absL` 0,95, `Tstart` −3,41 (§3.3, §3.5, §3.7)
- **URL** https://github.com/UMEP-dev/umep-solweig/issues/7 · [V] · `fonti/umep-solweig-issue-7.md`
  **perché conta qui** la domanda di conferma a monte sul repository di riferimento
  **cosa se ne prende** che la conferma manca: zero commenti al 2026-11-11 (§2, §4)
- **URL** https://github.com/UMEP-dev/umep-solweig/issues/14 · [V] · `fonti/umep-solweig-issue-14.md`
  **perché conta qui** segno di `Tstart` senza copertura del suolo
  **cosa se ne prende** righe, effetto misurato da Simons, e che `solweig` tiene −3,41 finché monte non decide (§3.5.1)
- **URL** https://github.com/UMEP-dev/umep-solweig/issues/13 · [V] · `fonti/umep-solweig-issue-13.md`
  **perché conta qui** il runner di riferimento chiama 2026a, `solweig` riproduce 2025a
  **cosa se ne prende** le due righe in cui differiscono a schema spento; `tgout1` assente in 2026a (§1, §3.8)
- **URL** https://github.com/UMEP-dev/umep-solweig/pull/15 · [V] · `fonti/umep-solweig-pr-15.md`
  **perché conta qui** difetti di `Tgmaps_v1` documentati con esempi
  **cosa se ne prende** ordine acqua/ciottolato, classi 3-4 in `KeyError`, classi 8-99 (§3.5)
- **URL** https://github.com/UMEP-dev/umep-solweig/issues/4 · [V] · `fonti/umep-solweig-issue-4.md`
  **perché conta qui** codice dell'acqua (3 contro 7) nel ramo classico
  **cosa se ne prende** che la questione esiste e sta fuori dallo strato Python confrontato (§3.5)
- **URL** https://pypi.org/pypi/umep-solweig/json · [V] · `fonti/pypi-umep-solweig.md`
  **perché conta qui** metadati del pacchetto pubblicato di `U:`
  **cosa se ne prende** una sola release, 2026.0.1, con sommario senza «Official» (§2)
- **URL** https://pypi.org/pypi/solweig/json · [V] · `fonti/pypi-solweig.md`
  **perché conta qui** metadati del pacchetto pubblicato di `S:`
  **cosa se ne prende** URL sorgente `UMEP-dev/solweig`, 77 release (§1)
- **URL** https://pypi.org/pypi/umep/json · [V] · `fonti/pypi-umep.md`
  **perché conta qui** il pip `umep` contro cui `solweig` fa ancora la parità di base
  **cosa se ne prende** che punta a `UMEP-dev/umep-core`, terzo albero (§1)
- **URL** https://api.github.com/repos/UMEP-dev/solweig e https://api.github.com/repos/UMEP-dev/umep-solweig · [V] · `fonti/github-repo-solweig-umep-solweig.md`
  **perché conta qui** relazione formale fra i due repository
  **cosa se ne prende** nessun fork, nessun archivio, date, release e tag (§1)
