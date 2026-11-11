# La trasmissività delle chiome è per cella, non per griglia

Il passo fuso di [`UMEP-dev/solweig`](https://github.com/UMEP-dev/solweig) riceve la
trasmissività delle chiome come **uno scalare per Ora**, `psi`, valido per l'intera
griglia. OMBRA ha due trasmissività nella stessa griglia, i sempreverdi e le chiome spoglie
d'inverno, e la [risoluzione del ticket](https://github.com/maeurong/OMBRA/issues/18) le ha
confermate come vincolo. Un solo scalare sbaglia per costruzione proprio a Bastia d'inverno,
dove il sole sotto le chiome è la domanda per cui il caso esiste.

Si decide: **nella copia estratta il nucleo riceve `psi` come raster, per cella**, e ne riceve
due. La [trasmissività lungo il raggio](../../CONTEXT.md) per l'ombra del sole, calcolata ogni
ora, e la [trasmissività verso il cielo](../../CONTEXT.md) per la volta celeste, calcolata una
volta per Scenario. La sostituzione la farà `estrai.py` e sarà dichiarata in `PROVENIENZA.toml`
con il conteggio dei punti toccati. All'11 novembre 2026 né la regola né la dichiarazione
esistono ancora, e il conteggio scritto qui, «uno per il raggio, sette per il cielo», era
sbagliato: vedi l'aggiornamento dell'11 novembre 2026.

## Perché non le alternative

**Un solo `psi` con una regola di scelta** (per stagione, o pesato sull'area delle chiome) costa
zero e sbaglia in una delle due direzioni per ogni cella mista: o i sempreverdi lasciano passare
il 50%, o le chiome spoglie fanno ombra piena.

**Due Corse del Motore, una per Strato, ricomposte a valle**, non si ricompongono: il passo fuso
porta lo stato termico del suolo da un'ora alla successiva, Tmrt è una radice quarta e non è
lineare nelle ombre, e la cella sotto entrambe le chiome verrebbe contata due volte.

**Chiedere a monte un `psi` per cella** resta possibile, ma la mappa non ne dipende e la
decisione, presa l'8 settembre 2026, è di non chiederlo: la modifica resta nostra.

## Cosa comporta

**È la prima modifica di significato nella copia vendorata.** L'[ADR 0002](0002-vendoring-del-motore.md)
ammette di adattare per far compilare e vieta di migliorare. Questa decisione apre una terza
classe, la trasformazione additiva dichiarata, a una condizione che non si negozia: **con un
raster costante il nucleo deve riprodurre a monte byte per byte.** È ciò che tiene validi i test
di parità, e un test lo verificherà. Per ora c'è una misura e non un test: vedi l'aggiornamento
dell'11 novembre 2026.

**Nel cielo anisotropo resta un'approssimazione**, e il Giornale la dichiarerà: una cella che vede
un sempreverde in una patch e una chioma spoglia in un'altra riceve una sola trasmissività verso
il cielo. Renderla esatta chiederebbe una matrice d'ombra per Strato, che è una modifica più
profonda di quella ammessa qui. Come due Strati si compongono in una sola geometria, e come si
calcola quell'unica trasmissività, lo decide l'[ADR 0006](0006-chiome-riunite.md).

**`conifer` non entra nel ragionamento.** Nel nucleo Rust al commit pinnato è un campo mai letto:
vive nella struct degli scalari e nei metadati Python. Non si sostituisce e non si dichiara.

## Aggiornamento, 2026-11-11: la funzione la scriveranno a monte, la sostituzione resta nostra

Il 10 novembre a monte hanno accettato di scrivere la funzione nominata: la chiusura di
`compute_timestep` estratta in una funzione con un nome, chiamabile da Rust
([ADR 0003](0003-catena-radiativa-e-cucitura-a-monte.md), aggiornamento di quel giorno). Uscirà
con la prossima release, e ancora non esiste.

L'8 settembre non si chiedeva la trasmissività per cella perché «la mappa non ne dipende»: il
passo fuso l'avremmo estratto noi, e la sostituzione sarebbe caduta su un testo tagliato da
noi. Quella premessa non vale più: la sostituzione cadrà su una funzione scritta a monte, e
chiedere la trasmissività per cella prima che ne fissino la firma era tornata un'opzione. Il
[ticket](https://github.com/maeurong/OMBRA/issues/43) l'ha pesata e conferma la decisione:
**non si chiede a monte**. La conclusione regge: nemmeno la
[mappa della catena radiativa](https://github.com/maeurong/OMBRA/issues/29), aperta dopo quella
dell'8 settembre, dipende da una risposta di monte. Le ragioni sono tre, e sono nuove.

**Con un solo parametro raster la modifica resterebbe.** A monte la trasmissività è una sola.
A OMBRA ne servono due, quella lungo il raggio e quella verso il cielo, in punti diversi del
passo: ricevuto un raster solo, la copia vendorata dovrebbe comunque sdoppiarlo e portare
ciascuno al suo punto.

**Chiedere due raster sarebbe chiedere a monte questo ADR.** Il loro strato Python calcola un
valore per giorno dell'anno e riempirebbe il raster con una costante. Dovrebbero cambiare anche
lo shader GPU, che riceve la trasmissività come parametro, nella release in cui l'estrazione
della funzione deve lasciare identiche byte per byte le loro uscite di riferimento (golden) e
di parità ([UMEP-dev/solweig#14](https://github.com/UMEP-dev/solweig/issues/14)). Il valore per
giorno dell'anno l'abbiamo letto nel sorgente di `UMEP-dev/solweig`. L'implementazione di
riferimento è ora `UMEP-dev/umep-solweig`, e lì resta da leggere.

**OMBRA vendora comunque.** A monte non prevedono di mettere `pyo3` dietro una feature, quindi
`estrai.py` dovrà passare su `pipeline.rs`, chiunque abbia scritto la funzione.

Per ora a monte non si manda nessuna richiesta sulla trasmissività. Se servirà parlarne, lo
farà Mario su UMEP-dev/solweig#14, quando uscirà la release.

### Il conteggio era sbagliato

«Uno per il raggio, sette per il cielo» torna contando le righe di `pipeline.rs` in cui `psi`
è letto: l'espressione lungo il raggio, poi due legature (`let psi = weather.psi;`) e cinque
chiamate, compresa quella dietro la feature `gpu`, che non si tocca. Saltava firme e corpi
delle due funzioni che lo ricevono. Questo ADR non diceva cosa contasse: la ricostruzione è
della [ricerca sul passo fuso](../ricerca/01-passo-fuso.md), ed è un'inferenza.

Al pin `c31c575` (0.1.0b98) la sostituzione toccherà undici righe in tre file:

| File | Cosa | N. righe |
|---|---|---|
| `pipeline.rs` | l'espressione lungo il raggio (`:756`) | 1 |
| `pipeline.rs` | le due legature verso il cielo (`:838`, `:1395`) | 2 |
| `pipeline_radiation.rs` | firma e corpo di `compute_ani_lum_from_packed` (`:378`, `:402`, `:421`) | 3 |
| `vegetation.rs` | firma e corpo di `kside_veg_isotropic_pure` (`:363`, `:413`, `:415`, `:417`, `:419`) | 5 |

Undici conta solo le righe da sostituire. L'ingresso dei due raster nella funzione si conterà
quando la firma di monte esisterà: `weather.psi` è un `f32`, e due raster entrano solo da
parametri o campi nuovi.

**Si sostituiscono le legature, non le chiamate.** Cambiate le due `let psi = weather.psi;`, le
quattro chiamate fuori dalla feature `gpu` che ne leggono il nome seguono da sole. Due delle
quattro stanno nel ramo dello schema di suolo 2026a, che OMBRA terrà spento, ma passano per le
stesse due funzioni: cambiata la firma, devono ricevere il raster anche loro, o la copia non
compila. Per questo «tre siti e due firme», il conteggio del ticket, non bastava. La quinta
chiamata, dietro la feature `gpu`, legge lo stesso nome e lo passa allo shader come scalare:
con la feature accesa la copia non compilerebbe. Lo dicono i tipi, non una compilazione. Nella
copia vendorata il caso non si presenta: a monte `gpu` è accesa di default, ma la copia non la
dichiara e non ha il modulo `gpu`.

**Cade una conseguenza della [risoluzione sulla forma del passo](https://github.com/maeurong/OMBRA/issues/32)**:
lì `weather.psi` restava scalare, e lo leggevano soltanto i siti 2026a e GPU. Dopo la
sostituzione nessuna riga compilata della copia leggerà quel campo. Che il campo resti è da
decidere: `WeatherScalars` è una `#[pyclass]`, e `estrai.py` toglie quegli elementi per intero
se una regola non li dichiara da tenere. I nomi decisi lì per i due raster restano:
`trasmissivita_raggio_v` e `trasmissivita_cielo_v`.

**`estrai.py` non ha ancora le regole per queste undici righe**: si scriveranno quando la
funzione nominata sarà in un tag. Ciascuna dichiarerà quante volte deve trovare il proprio
testo, e lo script si fermerà al primo conteggio che non torna, come già fa con il dizionario
`SOSTITUZIONI`. In `vegetation.rs` il numero da dichiarare dipende dall'ordine delle passate di
`estrai.py`: la gemella di `kside_veg_isotropic_pure`, `#[pyfunction] kside_veg`, porta la
stessa riga di parametro e, nel corpo, lo stesso testo quattro volte. Finché l'estrazione non
l'ha tolta, quei testi contano il doppio.

**Negli ultimi quaranta tag una regola a conteggio avrebbe fermato lo script due volte.**
L'espressione lungo il raggio è la stessa da 0.1.0b1. Da 0.1.0b55 a 0.1.0b98 (i numeri 59, 66,
69 e 85 non hanno un tag) non sono cambiati nemmeno i testi di `vegetation.rs`. Gli altri due
gruppi sì: a 0.1.0b86 `compute_ani_lum_from_packed` è passata da `pipeline.rs` a
`pipeline_radiation.rs`, e a 0.1.0b90 le legature sono diventate due. Due arresti in quaranta
tag bastano a tenere in piedi la sostituzione: ogni arresto costa rileggere una regola, ed è il
meccanismo che lavora.

### Sul passo intero la condizione regge

La condizione di questo ADR, con un raster costante uscite uguali a quelle di monte byte per
byte, fin qui era misurata solo sulle formule isolate. La misura dell'11 novembre confronta due
cloni di monte a 0.1.0b98, compilati senza la feature `gpu`, tolta in entrambi da
`pyproject.toml` e da `rust/Cargo.toml`: l'originale, e uno con le undici righe sostituite e i
due raster riempiti con lo scalare. Ciascuno esegue quattro calcoli di tre Ore consecutive:
luglio con trasmissività 0,03 e gennaio con 0,5, entrambi i mesi con i due Modelli di cielo. La
scena è sintetica, 60 × 60 celle con due edifici e due macchie di chioma. Le uscite sono sei
campi per Ora: `tmrt`, `shadow`, `kdown`, `kup`, `ldown`, `lup`. In tutto 72 raster e 259.200
valori, **identici byte per byte**.

Con un secondo valore sulla metà destra dei due raster, in questa scena la metà sinistra resta
identica e a destra cambiano 37.959 valori su 129.600: il clone modificato legge i raster cella
per cella.

**Cosa la misura lascia fuori**:

- `estrai.py` e la funzione nominata: la sostituzione è fatta a mano nella chiusura di
  `compute_timestep`;
- due raster diversi fra loro: i due provati hanno sempre lo stesso contenuto, e uno scambio
  fra raggio e cielo passerebbe identico;
- il ramo 2026a in esecuzione: compila, e nessun calcolo lo esegue;
- Bastia: la scena è sintetica;
- una seconda macchina: questa è arm64, con rustc 1.98.0;
- lo stato termico del suolo dopo l'ultima Ora, che non è fra le sei uscite.

Gli script della misura stanno fuori dal repository, in una cartella di sessione: per ripetere
la misura vanno riscritti.

Il test della condizione resta a carico nostro: si scriverà nel repository quando la funzione
nominata sarà in un tag.
