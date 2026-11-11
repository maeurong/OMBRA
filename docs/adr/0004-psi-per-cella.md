# La trasmissività delle chiome è per cella, non per griglia

Il passo fuso di [`UMEP-dev/solweig`](https://github.com/UMEP-dev/solweig) riceve la
trasmissività delle chiome come **uno scalare per passo orario**, `psi`, valido per l'intera
griglia. OMBRA ha due trasmissività nella stessa griglia, i sempreverdi e le chiome spoglie
d'inverno, e la [risoluzione del ticket](https://github.com/maeurong/OMBRA/issues/18) le ha
confermate come vincolo. Un solo scalare sbaglia per costruzione proprio a Bastia d'inverno,
dove il sole sotto le chiome è la domanda per cui il caso esiste.

Si decide: **nella copia estratta il nucleo riceve `psi` come raster, per cella**, e ne riceve
due. La [trasmissività lungo il raggio](../../CONTEXT.md) per l'ombra del sole, calcolata ogni
ora, e la [trasmissività verso il cielo](../../CONTEXT.md) per la volta celeste, calcolata una
volta per Scenario. La sostituzione la fa `estrai.py` ed è dichiarata in `PROVENIENZA.toml`
con il conteggio dei punti toccati: uno per il raggio, sette per il cielo.

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
di parità, e un test lo verifica.

**Nel cielo anisotropo resta un'approssimazione**, e il Giornale la dichiara: una cella che vede
un sempreverde in una patch e una chioma spoglia in un'altra riceve una sola trasmissività verso
il cielo. Renderla esatta chiederebbe una matrice d'ombra per Strato, che è una modifica più
profonda di quella ammessa qui.

**`conifer` non entra nel ragionamento.** Nel nucleo Rust al commit pinnato è un campo mai letto:
vive nella struct degli scalari e nei metadati Python. Non si sostituisce e non si dichiara.

## Aggiornamento, 2026-11-11: il passo fuso lo scrive monte, la sostituzione resta nostra

Il 10 novembre a monte hanno accettato di dare un nome al passo fuso
([ADR 0003](0003-catena-radiativa-e-cucitura-a-monte.md), aggiornamento di quel giorno). La
funzione su cui cadrà la sostituzione ora la scrive monte, e la premessa dell'8 settembre, «la
mappa non ne dipende», andava riletta. Il [ticket](https://github.com/maeurong/OMBRA/issues/43)
l'ha riletta e conferma la decisione: **non si chiede a monte**. Le ragioni sono tre, e sono
nuove.

**Con un raster solo la modifica resterebbe.** A monte la trasmissività è una. A OMBRA ne
servono due, su siti diversi, e la copia andrebbe divisa lo stesso.

**Chiederne due sarebbe chiedere a monte questo ADR.** Il loro strato Python calcola un valore
per giorno dell'anno e riempirebbe il raster con una costante. Dovrebbero cambiare anche lo
shader GPU, che riceve la trasmissività come parametro, nella stessa release in cui hanno
promesso uscite identiche byte per byte.

**OMBRA vendora comunque.** Monte non prevede di mettere `pyo3` dietro una feature, quindi
`estrai.py` passerà su `pipeline.rs` anche dopo la release, chiunque abbia scritto la funzione.

Per ora a monte non si scrive. Se servirà dirlo, Mario lo dirà quando risponderà alla release.

### Il conteggio era sbagliato

«Uno per il raggio, sette per il cielo» contava i siti spenti e saltava le firme. Al pin
`c31c575` (0.1.0b98) la sostituzione è di undici righe in tre file:

| File | Cosa | Righe |
|---|---|---|
| `pipeline.rs` | l'espressione lungo il raggio (`:756`) | 1 |
| `pipeline.rs` | le due legature verso il cielo (`:838`, `:1395`) | 2 |
| `pipeline_radiation.rs` | firma e corpo di `compute_ani_lum_from_packed` (`:378`, `:402`, `:421`) | 3 |
| `vegetation.rs` | firma e corpo di `kside_veg_isotropic_pure` (`:363`, `:413-419`) | 5 |

**Si sostituiscono le legature, non le chiamate.** Cambiate le due `let psi = weather.psi;`, le
quattro chiamate che ne leggono il nome seguono da sole. Due delle quattro stanno nel ramo
dello schema di suolo 2026a, spento in OMBRA, ma passano per le stesse due funzioni: cambiata
la firma, devono ricevere il raster anche loro, o la copia non compila. Il sito dietro la
feature `gpu` resta scalare: accesa la feature, la copia non compilerebbe.

**`estrai.py` non ha ancora queste regole.** Ciascuna dichiarerà quante volte deve trovare il
proprio testo, come già fa `sostituzioni` in `PROVENIENZA.toml`, e lo script si fermerà al
primo conteggio che non torna. Mentre `pipeline.rs` cambiava intorno, l'espressione lungo il
raggio e la chiamata dell'anisotropo hanno tenuto lo stesso testo nei quaranta tag da 0.1.0b55
a 0.1.0b98.

### Sul passo intero l'invariante regge

Fin qui lo si era misurato solo sui kernel isolati. La misura dell'11 novembre confronta due
copie di monte a 0.1.0b98, compilate senza la feature `gpu`: l'originale, e una con le undici
righe sostituite e i due raster riempiti con lo scalare. Ciascuna esegue quattro calcoli di tre
ore consecutive su una scena sintetica di 60 × 60 celle, con due edifici e due macchie di
chioma: luglio con trasmissività 0,03 e gennaio con 0,5, ciascuno con i due Modelli di cielo.
Sei campi per ora, 72 griglie, 259.200 valori: **identici bit per bit**.

Con un secondo valore sulla metà destra del raster, la metà sinistra resta identica e a destra
cambiano 37.959 valori: il nucleo legge il raster cella per cella.

**Cosa la misura lascia fuori.** La sostituzione è fatta a mano nella chiusura di
`compute_timestep`: `estrai.py` e la funzione nominata restano da provare. Il ramo 2026a
compila, e nessun calcolo lo esegue. La scena è sintetica e la macchina una sola: Bastia resta
da provare. Gli script stanno fuori dal repository. Il test che questo ADR chiede si scriverà
quando la funzione nominata sarà in un tag.
