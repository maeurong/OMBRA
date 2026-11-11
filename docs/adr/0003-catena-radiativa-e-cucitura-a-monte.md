# La catena radiativa si prende modulo per modulo, e per il passo fuso si chiede a monte

[ADR 0002](0002-vendoring-del-motore.md) ha deciso che il nucleo radiativo si vendora.
Funzionava per le ombre. Per la catena completa — sky view factor, radiazione, temperatura
media radiante — **non funziona allo stesso modo**, e questo ADR dice perché e cosa si fa.

## Il fatto

Il sorgente di `UMEP-dev/solweig` al commit pinnato è di 12.823 righe in 23 moduli. Letto,
non descritto:

**`shadowing.rs` e `utci.rs` hanno una porta d'ingresso nativa.** `calculate_shadows_rust`
prende `ArrayView2<f32>` e restituisce una struct Rust; `utci_single` prende quattro `f32`.
Python compare solo ai bordi, e toglierlo è ciò che `estrai.py` sa fare. Sono infatti i due
moduli già presi.

**Il passo fuso non ce l'ha.** Il calcolo di un'ora sta dentro `compute_timestep`, che è un
`#[pyfunction]`. Le sue righe 502-707 legano array numpy; le 708-1954 sono Rust puro dentro
una chiusura `allow_threads_unchecked(py, || …)` che restituisce `TimestepResultRaw`; dalla
1955 in poi si riconverte per Python. La fisica è quindi già separata — ma è una **chiusura
che cattura una quarantina di viste**, non una funzione, e nessuno script la trasforma in una
funzione in modo che regga il prossimo aggiornamento.

**I moduli intermedi hanno un ostacolo minore ma reale.** `skyview.rs`, `gvf.rs`, `ground.rs`
e `sky.rs` usano `PyResult` nelle firme del percorso Rust: `calculate_svf_inner` restituisce
`PyResult<SvfIntermediate>` perché `create_patches` fallisce con un `PyValueError` su
un'opzione sconosciuta. In `skyview.rs` sono quattro firme e due costruzioni di errore.

Dichiarare `pyo3` fra le dipendenze della copia vendorata non è una via d'uscita: senza la
feature `extension-module` il crate si lega a `libpython`, e il **binario singolo senza
dipendenze native** è un vincolo di progetto, non una preferenza.

## Cosa si è scartato

**Scrivere la nostra orchestrazione** sopra le funzioni di fisica vendorate. Sono le ~1250
righe che accoppiano ombre, SVF, radiazione e inerzia termica del suolo, ed è esattamente il
posto dove stanno le costanti e le euristiche che [ADR 0002](0002-vendoring-del-motore.md)
dice non stare nei paper. Riscriverle contraddirebbe la ragione per cui la reimplementazione
era stata scartata, e in più farebbe perdere i test di parità contro l'implementazione di
riferimento, che sono il motivo principale per cui quel codice merita fiducia.

## Cosa si fa

**Si prende modulo per modulo, dal basso e dall'alto verso il centro.** Ogni modulo che ha una
porta nativa entra subito; ogni modulo che chiede una trasformazione entra solo se la
trasformazione è **dichiarata, meccanica e capace di fermarsi con errore** quando a monte
cambia — la stessa disciplina che `estrai.py` già applica alle visibilità e agli attributi,
e che dal `2026-09-01` la CI verifica rieseguendo l'estrazione e confrontandola byte per byte.

Il prossimo modulo è `skyview.rs`, e chiede una classe di trasformazione nuova: `PyResult<T>`
diventa `Result<T, String>`. Due siti di errore, dichiarabili uno per uno in `PROVENIENZA.toml`.

**Per il passo fuso si chiede a monte.** Serve `compute_timestep_rust`: la chiusura di
`compute_timestep` estratta in una funzione che prende viste e restituisce
`TimestepResultRaw`, con il `#[pyfunction]` che le si appoggia sopra. È il rifattorizzamento
che a monte hanno già fatto per `calculate_shadows_rust`, quindi è nella loro forma e non nella
nostra, e non toglie niente a nessuno: Python continua a vedere la stessa API.

**Chi lo chiede è Mario, non un agente.** [ADR 0002](0002-vendoring-del-motore.md) l'ha già
stabilito — «aprire una richiesta a nome di Mario su un progetto altrui è un atto pubblico» —
e vale identico qui.

**Il ripiego, se a monte non risponde o dice di no**, è estendere la trasformazione dichiarata
anche a `compute_timestep`: `estrai.py` sostituisce il blocco dei parametri e cancella le due
regioni di conversione, delimitate da marcatori su cui lo script si ferma se non li trova più.
È più fragile della porta nativa, e per questo è il ripiego e non il piano.

## Cosa comporta

**Fino ad allora la catena è tronca**, e il Giornale lo dice campo per campo come già fa per
tutto il resto. Ciò che c'è oggi — ombre di edifici e chiome, con la trasmissività — è una
grandezza vera e verificabile; non è ancora la temperatura media radiante, e nessun documento
del progetto deve lasciar credere il contrario.

**UTCI è già in casa e non serve a niente da solo.** Non è uno spreco: è il pezzo che chiude la
catena dall'alto, ed è preso adesso perché è aritmetica pura e perché la promessa che un numero
di OMBRA sia confrontabile con un numero di SOLWEIG andava resa vera almeno per una grandezza.

**Se a monte accettasse la porta nativa**, questo ADR si chiude e ADR 0002 va riletto: il
vendoring resterebbe la forma, ma con una porta d'ingresso costruita apposta la distanza dal
fork si accorcia, e la scelta andrebbe rimotivata invece che ereditata.

## Aggiornamento, 2026-09-02: `skyview.rs` è dentro

Il primo passo è fatto, e ha corretto due numeri di questo ADR.

**Le firme erano quattro, e restano tre.** `svf_intermediate_to_py` ne portava
una, e sparisce con l'estrazione: converte il risultato in array numpy e non è
marcata da nessun attributo, quindi `estrai.py` la nomina invece di
riconoscerla. Insieme a lei se ne va un `impl Default` che senza la sua struct
non compilerebbe.

**Le costruzioni di errore erano due sul percorso CPU, e sono tre.** Oltre alle
due `PyValueError` — opzione di patch sconosciuta, finestra di ritaglio non
valida — c'è una `PyInterruptedError` per l'annullamento fra un patch e l'altro,
che questo ADR non aveva contato. E altre otto stanno dietro
`#[cfg(feature = "gpu")]`: undici in tutto nel testo del file.

Le otto spente si sostituiscono lo stesso. Una regola con un ritaglio di regione
è una regola che qualcuno dovrà rileggere, e se un giorno la feature si
accendesse `pyo3` non ci sarebbe comunque. La sostituzione è quindi sul
**costruttore** e non sulla costruzione: `pyo3::exceptions::Py…Error::new_err`
diventa `String::from`, l'argomento resta identico parentesi comprese, e la
stessa regola vale per un letterale e per un `format!`. Dettagli e conteggi
attesi in [`PROVENIENZA.toml`](../../vendor/solweig/PROVENIENZA.toml).

**Una premessa della spec è caduta.** [La spec](../spec.md) mette in cache lo
sky view factor per Scenario, e dice che è la differenza fra rispettare il
budget dei 60 secondi e sforarlo. Quel giudizio veniva dai 29-36 s misurati sul
percorso Python in [issue #2](https://github.com/maeurong/OMBRA/issues/2). Con
il pool a un thread che [ADR 0002](0002-vendoring-del-motore.md) ha già motivato,
sul dominio di 2500 celle del caso di riferimento il calcolo costa **da 32 a 63
ms per Corsa**, e le quattro Corse complete passano da 0,25 a 0,47 s. La cache
risparmierebbe 0,19 s su 60. Non si costruisce.

**Quello che l'utente ha adesso** è un campo in più nel Giornale, in due versioni
— quello che lasciano edifici e terreno, e quello che resta tolte le chiome,
ciascuna pesata per la sua trasmissività secondo la formula di monte
`svf - (1 - svf_veg) * (1 - psi)` — più il valore in ogni Punto di osservazione.
Sul caso di riferimento, d'estate, i tre Punti passano dal 66, 60 e 44 per cento
di cielo visibile al 41, 39 e 25 dopo la piantumazione.

Resta vero tutto il resto: **il passo fuso non ha ancora una porta d'ingresso**.

## Aggiornamento, 2026-09-03: la richiesta è aperta

[UMEP-dev/solweig#14](https://github.com/UMEP-dev/solweig/issues/14), aperta da
Mario. Chiede quattro cose in ordine di costo crescente, e dichiara che la prima
— dare un nome alla chiusura di `compute_timestep` — è l'unica di cui abbiamo
bisogno. Le altre tre servirebbero a dipendere dal crate invece di vendorarlo, e
il testo dice esplicitamente che senza la quarta, che è cara, le altre due non
comprano niente.

Il ripiego di questo ADR resta il piano se la risposta è no o non arriva.

## Aggiornamento, 2026-11-10: a monte hanno detto sì, e il ripiego si ferma

La risposta è arrivata sulla stessa
[richiesta](https://github.com/UMEP-dev/solweig/issues/14), da un collaboratore
del progetto. La funzione nominata per il passo fuso la scrivono loro, nella
prossima release, insieme a `crate-type = ["cdylib", "rlib"]`. Per l'API pubblica
preferiscono un `pub mod api` stretto, che riesporta solo ciò che OMBRA chiama,
e ne chiedono l'elenco. `pyo3` e `numpy` dietro una feature non sono in programma.

**La release non ha una data**, e la funzione non esiste ancora: questo ADR non
si chiude oggi. Si chiude quando la porta nativa è in un tag.

**Il ripiego si ferma** (Mario, 2026-11-10). `pipeline.rs` è cambiato sia in
0.1.0b97 sia in 0.1.0b98, ed è il file che il ripiego dovrebbe tagliare per
marcatori: costruirlo adesso significa scrivere una trasformazione da buttare
alla release. Non è fissata una data oltre la quale si riprende; se serve, la
decide Mario.

**Il pin va spostato comunque**, e non per il passo fuso. 0.1.0b97 corregge un
difetto di `skyview.rs`: senza strato di chioma, le matrici d'ombra della
vegetazione che restituisce sono sbagliate per il cielo anisotropo. I valori di
sky view factor non ne sono toccati, e OMBRA oggi legge solo quelli. Il Modello
di cielo anisotropo le matrici le userà: il pin deve essere almeno a 0.1.0b97
prima di allora.

**Spostato lo stesso giorno**, a 0.1.0b98
(`c31c5758236078b6b5f7bc9a931e2fce264bfc88`). Senza strato di chioma i campi di
sky view factor che il nucleo restituisce sono identici bit per bit a quelli del
pin precedente, e cambia solo la matrice che la correzione nomina. Cosa è stato
rimisurato e cosa no sta in `vendor/solweig/PROVENIENZA.toml`, alla voce
`a_quale_presa`.

L'implementazione di riferimento che il pacchetto segue è ora
[UMEP-dev/umep-solweig](https://github.com/UMEP-dev/umep-solweig).

## Aggiornamento, 2026-11-11: la porta nativa è in un tag, e questo ADR si chiude

A monte hanno chiuso la [richiesta](https://github.com/UMEP-dev/solweig/issues/14) come
completata, con la 0.1.0b99 (commit `92c7b8f`). Dei quattro punti ne sono arrivati tre.

**Il passo fuso ha un nome.** Al pin, che dall'11 novembre è 0.1.0b101
(`41840b74cd7c003f88dbde0c30fff736cb51bd5d`), la funzione è questa, in
`rust/src/pipeline.rs:571`:

```rust
pub fn compute_timestep_rust(inputs: &TimestepInputs) -> Result<TimestepResultRaw, String>
```

Controlla gli ingressi e chiama `timestep_physics`, definita a `:576`, dove sta la fisica che
prima era nella chiusura. `TimestepInputs` e le struct che contiene, in `pipeline_inputs.rs`, hanno
campi `pub` e viste `ndarray`: nessun tipo numpy.

**C'è un modulo pubblico, `api`**, che riesporta ciò che OMBRA chiama e ciò che serve al
passo. Gli altri moduli restano privati. A monte dichiarano che un cambio a ciò che `api`
riesporta sarà trattato come incompatibile e scritto nelle note di release. E
`rust/Cargo.toml` dichiara `crate-type = ["cdylib", "rlib"]`.

**Il quarto punto non è in programma.** `pyo3` resta nell'albero delle dipendenze:
`calculate_svf_inner` restituisce ancora `PyResult`, le tre struct di scalari sono
`#[pyclass]`, e `pyo3` è compilato con `extension-module`. A monte lo scrivono nella
risposta: un binario che si lega al crate non trova i simboli di Python.

**Quindi si continua a vendorare, e la scelta si rimotiva, come questo ADR chiedeva.**
[ADR 0002](0002-vendoring-del-motore.md) contava tre ostacoli: il tipo di crate, i moduli
privati, `extension-module`. I primi due sono caduti. Il terzo resta e basta da solo: con
`extension-module` un binario non si lega, e senza la feature si legherebbe a `libpython`,
mentre il binario singolo senza dipendenze native è un vincolo di progetto. Mario ha scritto
a monte, il 10 novembre, che il quarto punto non lo chiede. Dopo ADR 0002 è nata una seconda
ragione: [ADR 0004](0004-psi-per-cella.md) ha deciso che nella copia `psi` cambierà
significato, e una dipendenza dal crate quella modifica non la porterebbe.

**Cambia come si prende il passo.** `estrai.py` lo prenderà da una funzione con un nome, come
già fa per le ombre e per lo sky view factor. Il ripiego descritto sopra, il taglio di
`compute_timestep` per marcatori, non serve più e non si costruisce.

**L'estrazione del passo non è ancora scritta**, e la catena resta tronca: OMBRA non calcola
ancora la temperatura media radiante. Fra i file che `compute_timestep_rust` usa,
`pipeline_scalars.rs` porta gli attributi di Python sulle struct di scalari e sui loro
`impl`: 3 `#[pyclass]`, 3 `#[pymethods]`, 3 `#[new]`, 34 `#[pyo3(get, set)]`. Sono ciò che
l'estrazione dovrà togliere. Quali file entrano nella copia, e con quali regole, si decide
nella [mappa della catena radiativa](https://github.com/maeurong/OMBRA/issues/29).

**Per il passo, a monte chiedono almeno la 0.1.0b100**, per due ragioni. Una riguarda chi
chiama: in `HumanScalars.height` va l'altezza del baricentro per la postura, 1,1 m in piedi e
0,75 m seduti, non la statura. Lo strato Python di monte passava 1,75 m fino alla 0.1.0b99.
L'altra è una correzione al Rust: con `gvf_cache` l'onda corta riflessa dal suolo e l'onda
lunga che il suolo emette erano troppo basse vicino al bordo del raster, entro il raggio di
ricerca, che in piedi è di 22 m. Nel caso di riferimento la Griglia è di 50 × 50 celle a 1 m,
e quel margine lascia fuori solo le 36 celle centrali, su 2500: è un conto geometrico, non una
misura. Il pin è alla 0.1.0b101: cosa è stato misurato nel passaggio sta in
`vendor/solweig/PROVENIENZA.toml`, alla voce `a_quale_presa`.
