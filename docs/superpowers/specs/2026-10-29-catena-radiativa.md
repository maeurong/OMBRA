# Spec — la catena radiativa fino a Tmrt e UTCI

Destinazione della [mappa #29](https://github.com/maeurong/OMBRA/issues/29). Questo
documento dice cosa si costruisce. Il perché di ogni scelta sta nel commento di
risoluzione del ticket citato accanto, e negli ADR da [0003](../../adr/0003-catena-radiativa-e-cucitura-a-monte.md)
a [0006](../../adr/0006-chiome-riunite.md). Le parole sono quelle di [`CONTEXT.md`](../../../CONTEXT.md).
I numeri di monte sono letti al pin `41840b7` (0.1.0b101).

## Il problema

Alla 0.3.0 OMBRA calcola ombre, ore di sole e Sky view factor, ma non la
Temperatura media radiante, e quindi nemmeno la UTCI. Il tesista che studia un
cortile ottiene dove cade l'ombra e quanta volta celeste resta, non quanto caldo
sente una persona. È la domanda per cui ha aperto il Progetto, ed è il numero che
il relatore gli chiederà. La UTCI è già nel binario, ma da sola non serve: le
manca la Tmrt in ingresso. ADR 0003 lo dice: la catena è tronca, e nessun
documento deve lasciar credere il contrario.

Il calcolo dell'Ora sta nel passo fuso di `UMEP-dev/solweig`. Dalla 0.1.0b99 monte
lo espone come funzione nominata, `compute_timestep_rust`. Il passo però riceve
ingressi che a monte prepara uno strato Python, e OMBRA non ha Python: quello
strato va portato in Rust. E il passo riceve la trasmissività delle chiome come
uno scalare per la Griglia intera, che a Bastia d'inverno sbaglia per costruzione
([ADR 0004](../../adr/0004-psi-per-cella.md)).

## La soluzione

Una Corsa calcola, per ogni Ora del Periodo e per ogni cella, la Tmrt e la UTCI.
Ai Punti di osservazione scrive le serie orarie di Tmrt, UTCI e dei quattro
Flussi radiativi. Il Giornale dice con quali scelte di modellazione: Modello di
cielo, Postura, soglia del Muro, proprietà del suolo, Stato termico. Riporta
anche tre verifiche nuove, che alzano la bandiera quando la fisica si rompe.

L'utente sceglie tre cose per il Progetto, tutte con un predefinito: il Modello di
cielo (`anisotropo`), la Postura (`in piedi`), la soglia del Muro (1,0 m). Il
resto lo ricava OMBRA dagli oggetti dello Scenario e dal file meteo del Periodo.

Il caso di riferimento completo resta sotto i 60 secondi su sola CPU, e OMBRA resta
un binario singolo, senza Python e senza librerie native.

## Storie d'uso

### Il tesista, utente primario

1. Come tesista, voglio la mappa della Tmrt all'Ora di picco, per mostrare dove un
   cortile scotta e dove no.
2. Come tesista, voglio la mappa della UTCI per la stessa Ora, per argomentare in
   Categorie di stress invece che in gradi.
3. Come tesista, voglio la Tmrt e la UTCI per ogni Ora, non solo una media sul
   Periodo, per dire quando il cortile diventa invivibile e per quante ore.
4. Come tesista, voglio confrontare due Scenari a parità di Periodo, di Modello di
   cielo, di Postura e di soglia del Muro, per attribuire la differenza al progetto
   e non a una scelta di calcolo.
5. Come tesista, voglio le serie orarie di Tmrt e UTCI ai Punti di osservazione, per
   mettere in relazione un grafico con un punto preciso del rilievo.
6. Come tesista, voglio le serie di K↓, K↑, L↓ e L↑ ai Punti, per spiegare perché la
   Tmrt sale: per esempio, un K↑ che cresce con l'albedo di una pavimentazione.
7. Come tesista, voglio sapere in quali Ore la UTCI di un Punto è estrapolata, e
   oltre quale limite, per non citare come buono un valore fuori dal polinomio.
8. Come tesista, voglio scegliere fra Postura in piedi e seduta, per studiare una
   panchina oltre che un percorso pedonale.
9. Come tesista, voglio che un Progetto senza scelte esplicite usi i predefiniti
   più fedeli, per ottenere il risultato migliore senza conoscere il Motore.
10. Come tesista, voglio scegliere il Modello di cielo `isotropo`, per confrontarmi
    con uno studio che non usa quello anisotropo.
11. Come tesista con un dominio al tetto di progetto, voglio poter preferire il
    tempo alla fedeltà con `isotropo`, senza che il resto della catena cambi.
12. Come tesista, voglio alzare la soglia del Muro, per escludere muretti e gradini
    che nel mio rilievo non sono pareti.
13. Come tesista, voglio un messaggio chiaro, prima del calcolo, se scrivo una
    soglia del Muro sotto 0,5 m, per non ricevere Muri orientati a nord in silenzio.
14. Come tesista, voglio che il Periodo cominci dove dico, senza Ore di avviamento,
    per non dover capire l'inerzia del suolo prima di lanciare una Corsa.
15. Come tesista, voglio che una Corsa con un valore meteo mancante si fermi prima
    del calcolo e dica quale, invece di inventare la Tmrt di quell'Ora.
16. Come tesista, voglio che un'Ora senza vento abbia la Tmrt e una UTCI senza
    valore, perché la Tmrt non dipende dal vento e la UTCI sì.
17. Come tesista, voglio che le Ore si leggano come intervalli («12–13», «21 giu»),
    per non confondere un'Ora con l'istante scritto nel file EPW.
18. Come tesista, voglio una vista tabellare di ogni campo calcolato accanto alla
    mappa, per leggere i valori senza affidarmi al colore.
19. Come tesista, voglio che il caso di riferimento completo stia sotto il minuto,
    per continuare a iterare sul progetto invece di aspettare.
20. Come tesista, voglio scaricare un file solo e aprirlo, senza installare Python,
    per usare OMBRA sul portatile che ho.

### Lo studente in esercitazione

21. Come studente, voglio che il Progetto del laboratorio giri con i predefiniti,
    per arrivare alla mappa della UTCI nella durata dell'esercitazione.
22. Come studente, voglio che la Categoria di stress abbia un nome, nella mia
    lingua e in inglese, per leggerla senza la tabella delle soglie.

### Il relatore e chi rilegge il risultato

23. Come relatore, voglio che il Giornale dichiari Modello di cielo, Postura,
    altezza del baricentro, assorbimenti, proprietà del suolo e dei Muri, per
    giudicare un risultato senza aprire il codice.
24. Come relatore, voglio che ogni costante portata da monte citi la riga da cui
    viene, per verificarla sulla fonte.
25. Come relatore, voglio che il Giornale dica quando il Motore avrebbe ripiegato
    in silenzio sull'isotropo, per non credere anisotropo un conto che non lo è.
26. Come relatore, voglio che il Giornale riporti l'inviluppo di Tmrt e UTCI con
    l'Ora e la cella di minimo e massimo, per trovare subito un valore strano.
27. Come relatore, voglio che una Tmrt fuori da −30/+70 K dall'aria, o un NaN, alzi
    la bandiera, per accorgermi di un difetto della fisica.
28. Come relatore, voglio una verifica che la notte piena sia buia, per sapere che
    il record meteo è abbinato all'Ora giusta.
29. Come relatore, voglio una verifica che la cella più aperta, al sole, riceva
    quanto il file meteo dichiara, per sapere che la radiazione entra nel verso
    giusto.
30. Come relatore, voglio il confronto fra l'ombra del passo e quella di OMBRA,
    per sapere che l'ombra verificata nel Giornale è quella entrata nella Tmrt.
31. Come relatore, voglio che una verifica che non ha potuto girare alzi la
    bandiera, per non leggerla come una verifica passata.
32. Come relatore, voglio che il Giornale dichiari gli scostamenti noti da monte,
    come l'acqua senza la sua temperatura, per pesarli invece di scoprirli.
33. Come relatore, voglio che il Giornale dica da quale record viene il clearness
    index della prima notte, o che è il ripiego a 1,0, per capire le prime Ore.
34. Come relatore, voglio che il Giornale conti le Ore che dipendono dallo Stato
    termico iniziale, per sapere se il Periodo è cominciato di giorno.
35. Come relatore, voglio che il Giornale conti i Muri, quelli che non toccano un
    Edificio e gli Edifici sul bordo della Griglia, per giudicare la geometria.
36. Come relatore, voglio che due Corse con scelte di Progetto diverse abbiano
    Impronte diverse, per non scambiare una Corsa per un'altra.
37. Come relatore, voglio che le Ore nel Giornale siano intervalli ISO 8601 con il
    fuso, per elaborarle senza convertire un indice.

### Chi mantiene OMBRA

38. Come manutentore, voglio prendere il passo dalla funzione nominata di monte
    con `estrai.py`, per aggiornare il pin rieseguendo uno script invece di
    rifare un taglio a mano.
39. Come manutentore, voglio che `estrai.py` si fermi al primo conteggio che non
    torna, per accorgermi quando monte cambia una riga su cui poggia una regola.
40. Come manutentore, voglio che un campo aggiunto o tolto a monte in
    `TimestepInputs` faccia fallire la compilazione, per non passare al passo un
    ingresso sbagliato in silenzio.
41. Come manutentore, voglio un banco che confronti il passo vendorato con quello
    del clone di monte, per sapere che con raster costanti la copia riproduce
    monte byte per byte.
42. Come manutentore, voglio che i record di riferimento portino il pin con cui
    sono stati generati, per veder fallire i test quando sposto il pin senza
    rigenerarli.
43. Come manutentore, voglio che Python serva solo a generare i record, per tenere
    la CI e il binario liberi da Python.
44. Come manutentore, voglio che i campi `psi` e `conifer` spariscano dalla copia,
    per veder fallire la compilazione se monte aggiunge una loro lettura.
45. Come manutentore, voglio un test su più passi dello Stato termico, per vedere
    una deriva che un test di un passo solo non vede.

## Decisioni di implementazione

### Il passo, da monte

- **Il passo si vendora dalla funzione nominata** `compute_timestep_rust`
  ([ADR 0003](../../adr/0003-catena-radiativa-e-cucitura-a-monte.md), aggiornamento
  dell'11 novembre). `estrai.py` estende la copia alla chiusura delle dipendenze del
  dossier `research/catena-radiativa.md` (§1): diciassette moduli, muri compresi,
  `skyview.rs` e `utci.rs` già dentro. Restano fuori `pipeline_bundles.rs`, il
  modulo `gpu`, `morphology.rs` e `pet.rs`.
- **Le trasformazioni nuove** sono quelle del dossier (§2), aggiornate dalle
  risoluzioni successive: `use numpy::{…}` e `use pyo3::{…}` su più righe; gli
  attributi `#[pyo3(get, set)]`; il `pub use … };` di `pipeline_bundles`, con una
  regola sua che si ferma su `};`; un controllo dei residui di Python che guarda il
  codice fuori dai commenti e cerca identificatori. Ogni regola dichiara un
  conteggio, e lo script si ferma al primo che non torna. Le regole e i conteggi li
  dichiara `PROVENIENZA.toml`.
- **Due raster al posto di `psi`**
  ([Il passo sulla firma di monte](https://github.com/maeurong/OMBRA/issues/60),
  [ADR 0004](../../adr/0004-psi-per-cella.md)). `estrai.py` aggiunge a
  `TimestepInputs` due campi obbligatori di tipo vista 2D in `f32`,
  `trasmissivita_raggio` e `trasmissivita_cielo`, e nella destrutturazione di
  `timestep_physics` le legature `trasmissivita_raggio_v` e `trasmissivita_cielo_v`.
  Sostituisce le undici righe di ADR 0004 in tre file. Si sostituiscono le
  legature, non le chiamate. I campi non sono `Option`: uno Scenario senza Alberi
  passa raster costanti a uno.
- **Gli scalari restano struct Rust** (#60). `WeatherScalars`, `HumanScalars` e
  `ConfigScalars` perdono gli attributi pyo3 e i blocchi `#[pymethods]`, e una
  regola a conteggio toglie i campi `psi` e `conifer`. Il commento di modulo di
  monte che parla di `#[new]` resta, dichiarato.
- **OMBRA costruisce `TimestepInputs` con un letterale** che nomina ogni campo,
  nel codice di OMBRA e non in `vendor/`. Un campo aggiunto o tolto a monte rompe
  la compilazione: il controllo incrociato dei campi in `estrai.py` non si scrive.
  Il letterale passa `ground_scheme: None`, quindi lo schema di suolo 2026a resta
  spento, e `output_mask: OUT_ALL`, costante del codice
  ([Giornale campo per campo](https://github.com/maeurong/OMBRA/issues/36)).
- **`gvf_cache`**: la geometria del ground view factor si costruisce una volta per
  Corsa con `precompute_gvf_geometry_cpu`, perché dipende solo dagli edifici, dai
  raster dei Muri, dall'albedo e dalle altezze ricavate dalla Postura. La firma la
  ammette facoltativa; qui la si passa sempre.
- **I raster hanno tutti la forma del modello di superficie**, e lo controlla OMBRA
  prima del passo: monte controlla solo le matrici d'ombra.

### Il Motore, in OMBRA

- **Il modulo `motore` prende una funzione per l'Ora**, accanto a `ombre` e
  `sky_view_factor`. Riceve i raster dello Scenario, i raster per l'Ora, gli
  scalari e lo Stato termico, e restituisce ciò che di `TimestepResultRaw` serve
  alla Corsa. È il seam del banco rlib. Gira nel pool a un thread che il modulo usa
  già ([Quanto costa la catena piena](https://github.com/maeurong/OMBRA/issues/24):
  fuori dal pool il costo per Ora cresce di un ordine di grandezza).
- **I Muri li calcola il Motore**, come lo Sky view factor
  ([I muri](https://github.com/maeurong/OMBRA/issues/20)). Una funzione accanto a
  `sky_view_factor` porta `findwalls` in Rust, citando la riga di monte, e ricava
  l'orientamento con `compute_wall_aspect_pure` vendorato. Lavora sul modello di
  superficie che la Derivazione produce, terreno compreso. I Muri sono derivati
  rigenerabili e non si salvano ([ADR 0001](../../adr/0001-oggetti-e-raster.md)).
  I casi degeneri si trattano come a monte e si contano. `has_walls` resta acceso.
- **Lo Sky view factor esce dalla passata che c'è già**, ma la Corsa ora ne tiene
  anche le tre matrici d'ombra e i direzionali, in memoria e solo per la durata
  della Corsa ([Il cielo anisotropo](https://github.com/maeurong/OMBRA/issues/27)).
  Con due Strati di chioma le matrici della vegetazione vengono dalla passata sulle
  Chiome riunite ([ADR 0006](../../adr/0006-chiome-riunite.md)). Con `isotropo` le
  matrici non si conservano. La struct che porta i direzionali non prende il nome
  Sky view factor: il termine resta i due valori pubblicati.
- **La Trasmissività verso il cielo diventa un raster**, per Scenario. Vale uno
  meno la detrazione divisa per la volta che le Chiome riunite tolgono (ADR 0006).
  La 0.3.0 calcola già la detrazione, dalla differenza fra i tre campi. Dove nessuna
  chioma copre la cella vale uno.
- **La Trasmissività lungo il raggio è un raster per Ora**: il prodotto delle
  trasmissività degli Strati che il raggio attraversa. La produce la Derivazione
  dalle ombre per Strato, come dice `CONTEXT.md`, e il passo la riceve al posto di
  `psi`.

### Lo strato Python portato

- **Si porta, non si vendora**, e ogni costante e formula cita la riga di `solweig`
  al pin. Dove `umep-solweig` coincide, la sua riga sta accanto; dove diverge, una
  nota dice in cosa ([Strato Python portato](https://github.com/maeurong/OMBRA/issues/34),
  [Dove vive lo strato Python](https://github.com/maeurong/OMBRA/issues/54)).
- **Le parti**: posizione del sole, `altmax`, clearness index e ripartizione fra
  diretta e diffusa, `svfalfa`, `findwalls`, proprietà del suolo, maschera degli
  edifici, Stato termico iniziale e dopo il passo, `snup`, `dectime`, `jday`,
  `timestep_dec`, le costanti dei Muri.
- **Il sole dell'Ora è uno solo, ed è l'SPA di monte portato** (Mario, 2026-11-22).
  Sostituisce l'algoritmo attuale per tutto ciò che la Corsa calcola in un'Ora:
  ombre, ore di sole, verifica dell'ombra, radiazione. Resta al centro dell'Ora
  ([ADR 0005](../../adr/0005-il-sole-al-centro-dell-ora.md)). `dectime` e `jday`
  restano sull'etichetta del record, come a monte. Fra i due algoritmi la
  [ricerca 02](../../ricerca/02-sole-meta-passo.md) misura al più 0,47°: le ombre
  diurne cambiano di poco, e la versione sale.
- **L'Ora a cavallo dell'alba** si tratta come `solweig` con un file meteo che
  porta le misure: la diretta e la diffusa sono quelle del file, anche con il sole
  sotto l'orizzonte al centro dell'Ora (Mario, 2026-11-23). La regola «diretta a
  zero, globale in diffusa» vale a monte solo senza misure, e un EPW le porta sempre.
- **Le proprietà del suolo stanno in una tabella nel codice**, come le specie
  ([Da dove vengono albedo, emissività e le temperature del suolo](https://github.com/maeurong/OMBRA/issues/19)).
  Erba alla classe di monte 5, terreno nudo alla 6, pavimentato alla 1, acqua alla
  7, celle sotto gli Edifici alla 2, cella senza Superficie al ciottolato, classe 0.
  Le classi di OMBRA, da 0 a 4, si traducono in quelle di monte prima del passo.
  L'acqua resta alla classe 7 senza la sua temperatura, perché il passo la cerca
  alla 3: è uno scostamento noto, dichiarato con il rimando a
  [UMEP-dev/umep-solweig#4](https://github.com/UMEP-dev/umep-solweig/issues/4), e non
  si corregge.
- **I Muri hanno un insieme solo di cinque valori**, i predefiniti di `solweig`:
  albedo 0,20, emissività 0,90, TgK 0,37, Tstart −3,41, TmaxLST 15,0.
- **La persona**: dalla Postura si ricava l'altezza del baricentro, 1,1 m in piedi
  e 0,75 m seduta. `abs_k = 0,70` e `abs_l = 0,97` sono costanti. Il commento di
  `abs_l` dichiara lo scarto da UMEP-QGIS (0,95, circa 0,4 K a 40 °C); quello
  dell'altezza seduta dichiara che monte non cita una fonte
  ([I parametri del corpo umano](https://github.com/maeurong/OMBRA/issues/28)).

### La Corsa

- **Abbinamento fra record e Ora**: l'Ora `k` del Periodo prende il record `k + 1`
  contato dall'ora 1 del primo giorno (ADR 0005). La Corsa lo ottiene dal lettore
  meteo, che rileva già un file che comincia a metà di un giorno: con l'abbinamento,
  quel file ferma la Corsa prima del calcolo.
- **Un valore meteo mancante che serve al passo** (temperatura, umidità, una delle
  tre radiazioni) ferma la Corsa prima del calcolo, e il messaggio dice quale e in
  quale Ora. Il vento mancante lascia senza valore solo la UTCI di quell'Ora, che
  conta nella frazione senza dato. Niente si riempie
  ([Cosa il Giornale scrive di Tmrt e UTCI](https://github.com/maeurong/OMBRA/issues/25)).
- **Lo Stato termico vive nella Corsa, in memoria**
  ([Lo stato termico e la forma della Corsa](https://github.com/maeurong/OMBRA/issues/21)).
  Parte dallo stato iniziale di monte, esce dal passo in `TimestepResultRaw` e
  rientra all'Ora successiva. La logica dopo il passo è portata riga per riga: di
  notte, con il sole al centro dell'Ora a quota ≤ 0, `firstdaytime` torna a 1 e
  `timeadd` a 0. Nessuna Ora di avviamento. Lo Stato termico non va su disco e il
  Progetto non lo conosce.
- **Il clearness index della prima notte** si cerca all'indietro nel file meteo con
  il criterio di monte: l'ultimo record con il sole sopra l'orizzonte e la
  radiazione sopra zero. La ricerca gira sul cerchio solo se il file è un anno
  intero, dal 1/1 ora 1 al 31/12 ora 24. Senza una sera il valore è 1,0, dichiarato.
- **Ogni Ora porta nella Corsa** il raster della Tmrt e quello della UTCI, per cella.
  La UTCI la calcola OMBRA dalla Tmrt con il polinomio vendorato, sul campo intero.
  L'ombra del passo resta in memoria per l'Ora, solo per il confronto; K↓, K↑, L↓ e
  L↑ diventano serie ai Punti e non si conservano come raster.
- **I campi della Corsa crescono**: Tmrt e UTCI per Ora e per cella, e ai Punti sei
  serie in più (Tmrt, UTCI, K↓, K↑, L↓, L↑) più l'elenco delle Ore con la UTCI
  estrapolata, ciascuna con il limite superato. La Categoria di stress non si scrive
  per Ora: si ricava dalle soglie che il Giornale scrive una volta.
- **Le Ore della Corsa hanno una scrittura**
  ([Come si scrive un'Ora](https://github.com/maeurong/OMBRA/issues/63)). Il Giornale
  scrive una volta la lista delle Ore come intervalli ISO 8601, inizio con il fuso
  e durata (`2026-06-21T12:00+01:00/PT1H`), e le serie si allineano a quella lista.
  L'Ora degli estremi usa la stessa scrittura. Il fuso è quello di `[sole]`, l'anno
  quello dell'inizio del Periodo. Nelle viste l'Ora si scrive «12–13», con il giorno
  in una colonna a parte («21 giu», «21 Jun») e il fuso dichiarato, senza ora
  legale. I nomi dei mesi stanno in `lingua`.

### Il Progetto

- **Tre scelte nuove sul Progetto**, nel manifesto: Modello di cielo (`anisotropo`
  o `isotropo`), Postura (`in piedi` o `seduta`), soglia del Muro in metri. Valgono
  per tutti gli Scenari e i Periodi. Un manifesto che non le scrive prende i
  predefiniti: `anisotropo`, `in piedi`, 1,0 m.
- **Una soglia del Muro sotto 0,5 m, o non finita, si rifiuta** alla scrittura e
  alla lettura del Progetto, prima del calcolo, e il messaggio cita la riga di monte.
  In alto non c'è limite.
- **Le tre scelte entrano nell'Impronta.** Oggi l'Impronta prende Griglia, Scenario
  e Periodo; una scelta di Progetto fuori dall'Impronta darebbe lo stesso nome a due
  Corse diverse.
- **La versione del binario sale due volte** (Mario, 2026-11-23), come è successo
  per la 0.2.0 e la 0.3.0: a 0.4.0 con il commit che porta il sole SPA, che cambia
  ombre e Giornale già prima della catena, e a 0.5.0 con il commit che porta la
  catena.

### Il Giornale

Sezioni nuove, ciascuna con le righe di monte citate dove un valore viene da monte.
Una sezione assente vuol dire «non calcolato», senza segnaposti.

- **`[cielo]`**: il Modello di cielo. Con `anisotropo` OMBRA controlla prima del
  passo che le tre matrici d'ombra ci siano, e se ne manca una la Corsa si ferma con
  un errore. Il ripiego silenzioso di monte sull'isotropo non arriva mai al passo.
- **`[persona]`**: Postura, altezza del baricentro, `abs_k`, `abs_l`, ciascuno con la
  sua riga.
- **`[suolo]`**: per ogni tipo di Superficie usato, la classe di monte, i cinque
  valori, la riga, il numero di celle; il numero di celle col predefinito; lo
  scostamento noto dell'acqua.
- **`[muri]`**: la soglia, il lato del filtro d'orientamento in celle (`floor(9 /
  passo)`, dispari, almeno 3), le celle-Muro, l'altezza massima, le celle-Muro che
  non confinano con un Edificio, gli Edifici sul bordo della Griglia, i cinque
  valori dei Muri.
- **`[stato_termico]`**: lo stato iniziale di monte, il criterio di notte, il
  clearness index portato con il record da cui viene o il ripiego a 1,0, le Ore che
  dipendono dallo stato iniziale.
- **`[utci]`**: il polinomio con la fonte (Bröde et al. 2012, copia vendorata), la
  regola del vento (a 10 m, dal file, mai ritoccato), le soglie delle dieci
  Categorie di stress, le celle-Ora con la UTCI estrapolata per limite superato.
- **Inviluppi**: Tmrt e UTCI su tutte le celle e le Ore, con l'Ora e la cella di
  minimo e massimo. I quattro Flussi radiativi sulle serie ai Punti, senza Ora né
  cella.

| Campo | Intervallo plausibile | La bandiera si alza per |
|---|---|---|
| Tmrt | Tmrt − Ta fra −30 e +70 K, con Ta dell'Ora | un valore fuori, o un NaN in qualunque cella |
| UTCI | nessuno | un NaN dove Tmrt e vento dell'Ora ci sono |
| K↓, K↑ | da 0 a 1410 W/m² | un valore fuori |
| L↓, L↑ | da 100 a 900 W/m² | un valore fuori |

La UTCI estrapolata non alza la bandiera: si conta in `[utci]`.

**Tre verifiche per Corsa nuove**, accanto a quella dell'ombra. Ognuna alza la
bandiera anche quando non ha potuto girare.

1. **Le due ombre.** Nelle Ore con il sole sopra 3° il Giornale conta le celle-Ora
   in cui l'ombra del passo e quella di OMBRA differiscono di più di 0,5, e la loro
   frazione. Le Ore con il sole fra 0° e 3° si contano a parte: lì il passo applica
   `min_sun_altitude = 3.0` e le due ombre differiscono per costruzione.
2. **La notte piena è buia.** In un'Ora con il sole sotto l'orizzonte all'inizio e
   alla fine, K↓ e K↑ valgono zero in ogni cella.
3. **Il cielo aperto torna con il file meteo.** In ogni Ora con il sole, nella cella
   illuminata con lo Sky view factor più alto, K↓ si confronta con DNI · sin h + DHI
   del record. Il Giornale scrive lo scarto massimo, con la sua Ora e la sua cella.

Le soglie della prima e della terza si misurano su Bastia nel piano.

### Vocabolario e lingua

- Le parole nuove sono già in `CONTEXT.md`: Temperatura media radiante, Flusso
  radiativo, Muro, Stato termico, UTCI, Categoria di stress. **Postura** attende la
  PR [#53](https://github.com/maeurong/OMBRA/pull/53).
- `comfort::fuori_intervallo` e `comfort::banda` si rinominano secondo il
  vocabolario: «fuori intervallo» resta solo al valore non plausibile
  dell'inviluppo, e la fascia è una Categoria di stress. I nomi delle Categorie
  stanno in `lingua`, in italiano e in inglese.

## Decisioni sui test

Un buon test guarda il comportamento che un utente o un modulo chiamante vede, non
come il codice ci arriva: un Giornale, un campo, un valore a una cella. Fallisce se
la logica si rompe. Niente framework, niente fixture elaborate. I casi degeneri si
enumerano prima di scrivere il codice.

Tre seam, confermati da Mario il 2026-11-22.

### 1. La Corsa

Il seam principale: `corsa::esegui` su un Progetto, leggendo il Giornale e i campi
restituiti. È il modo in cui i test della Corsa lavorano già, con la verifica
dell'ombra, gli inviluppi, l'Impronta e il cancello dei 60 secondi.

- **Due Corse registrate su Bastia**, con un meteo di 48 Ore estive costruito apposta:
  un giorno sereno, una sera coperta con il clearness index sotto 0,95 portato oltre
  la mezzanotte, un'Ora d'alba con il sole sotto l'orizzonte al centro e la globale
  sopra zero.
  - *Bastia uniforme*: uno Strato di chioma, trasmissività costante uguale allo
    scalare di `solweig`, Postura in piedi e seduta. Tmrt entro 1e-3 °C dal record.
  - *Bastia com'è*: le specie vere, Postura in piedi, contro la wheel con il `psi`
    che `solweig` calcola per la data, d'estate e anche d'inverno, con un secondo
    meteo di 48 Ore. D'estate c'è un solo Strato di chioma e solo l'inverno
    esercita le due Trasmissività di ADR 0004 (Mario, 2026-11-23). Alla prima
    generazione si misurano scarto massimo e 95° percentile, e la soglia si fissa
    poco sopra, con la misura scritta accanto.
  - Il record è il campo Tmrt intero, in `f32` binario con un'intestazione TOML: pin,
    comando, forma, sha256. La UTCI si registra ai Punti.
- **Lo Stato termico su più passi**: le due Corse attraversano due notti, e coprono
  la deriva che un passo solo non vede.
- **Il Giornale**, con un test per ciascuno di questi comportamenti:
  - ogni sezione nuova e ogni verifica;
  - una verifica mai eseguita alza la bandiera;
  - un manifesto senza le tre scelte prende i predefiniti;
  - una soglia del Muro sotto 0,5 m è rifiutata;
  - cambiare una sola delle tre scelte cambia l'Impronta;
  - un valore meteo mancante ferma la Corsa prima del calcolo, il vento mancante no;
  - con `anisotropo` e una matrice mancante la Corsa si ferma;
  - le Ore sono scritte come intervalli ISO 8601.
- **Il cancello dei 60 secondi** resta e copre la catena piena, in release.
- Prior art: i test della Corsa sul caso di riferimento e sul Giornale.

### 2. Il passo del Motore

La funzione di `motore` che fa un'Ora, contro `compute_timestep_rust` di un clone di
monte al pin, nel banco rlib.

- **L'invariante di ADR 0004**: con i due raster di trasmissività costanti, le uscite
  sono uguali a monte byte per byte, compreso lo Stato termico dopo l'ultima Ora.
  Un secondo caso usa due raster diversi fra loro, per vedere uno scambio fra raggio
  e cielo che la misura dell'11 novembre non poteva vedere.
- **Il banco vive in un crate con un suo `[workspace]`**, fuori dal binario
  distribuito. La via per collegare il crate di monte è quella raccomandata da
  [Il banco di parità e il crate di monte in rlib](https://github.com/maeurong/OMBRA/issues/61):
  `extension-module` tolto dal manifesto del clone dopo il checkout, con
  `PYO3_CONFIG_FILE` come riserva. Va misurata prima su `ubuntu-latest`, dove gira
  la CI.
- **I casi degeneri dei Muri**, un test per caso con l'oracolo di monte: Edificio
  largo una cella, Edificio sul bordo della Griglia, gradino del terreno senza
  Edificio, cella entro mezzo filtro dal bordo, Scenario senza Edifici con il raster
  dei Muri tutto a zero.
- Prior art: i test del Motore su ombre e Sky view factor.

### 3. Lo strato portato

Le funzioni portate da `solweig`, contro record per parte generati una volta dalla
wheel `solweig` alla versione esatta del pin.

- Sole, `altmax`, clearness index e ripartizione, `svfalfa`, proprietà del suolo,
  Stato termico iniziale e dopo il passo: tolleranza relativa 1e-9 sulle funzioni in
  `f64`, uguaglianza esatta per `findwalls` e per il suolo.
- **La corrispondenza fra classi di OMBRA e classi di monte** ha un test suo: una
  corrispondenza sbagliata sposta la Tmrt senza che il confronto la riconosca.
- **Un test confronta il pin di ogni record con quello di `PROVENIENZA.toml`.** Se il
  pin si sposta, il test fallisce finché i record non sono rigenerati nella stessa PR.
- **Python serve solo al generatore**, con la versione della wheel fissata. Non entra
  né nella CI né nel binario. Il generatore lo lancia Mario.
- Prior art: il lavoro `Vendor check`, che riesegue l'estrazione e confronta byte
  per byte, e la tabella delle specie con la sua fonte citata.

Una tolleranza si allenta solo con una misura scritta accanto.

## Fuori scope

- PET: la catena si ferma a UTCI.
- Il percorso GPU: solo CPU.
- La validazione contro misure di campo: è un piano a sé.
- Lo schema di suolo 2026a: `ground_scheme` resta `None`.
- La cache dello Sky view factor per geometria.
- L'invio di PR a UMEP-dev/solweig: è un atto pubblico di Mario.
- I file meteo sub-orari, rifiutati prima del calcolo.
- Un caso di riferimento oltre Bastia.
- Il materiale del Muro per Edificio: un insieme solo di valori per tutti i Muri.
- Lo Stato termico su disco, e la concatenazione di Corse.
- Le mappe per cella dei quattro Flussi radiativi.
- Gli scalari `abs_k` e `abs_l` modificabili dall'utente.
- La correzione dell'acqua alla classe di monte 3.

## Note

**Da misurare nel piano**, già annotato nei ticket:

- su quale lato del gradino cade il Muro: la lettura di `findwalls` dice la cella
  bassa, fuori dall'impronta, la docstring di monte il contrario. È il primo test, e
  conteggi e casi degeneri si riscrivono se lo smentisce;
- le soglie delle verifiche delle due ombre e del cielo aperto, su Bastia;
- quanto la classe 2 sotto i tetti sposta la Tmrt al suolo;
- la Trasmissività verso il cielo rimessa nella formula di monte in `f32`;
- quanto pesa l'unica Trasmissività verso il cielo per tutte le patch coperte;
- la doppia ombra per Ora nel tempo della Corsa, e il caso al tetto di progetto;
- la via del banco rlib su `ubuntu-latest`;
- il minuto dell'EPW di Bastia, che nessuna macchina usata finora ha letto.

**Ancora aperti fuori dalla mappa**: la PR [#53](https://github.com/maeurong/OMBRA/pull/53),
con la voce Postura, e la PR [#62](https://github.com/maeurong/OMBRA/pull/62), con la
ricerca sul crate rlib.

**Il sole**: la scelta dell'SPA di monte come sole unico è presa in questa spec, non
in un ticket della mappa. Le altre decisioni vengono dai ticket citati.
