# La geometria delle chiome verso il cielo verrà da una passata sulle Chiome riunite

Il passo fuso di [`UMEP-dev/solweig`](https://github.com/UMEP-dev/solweig) accetta **una sola**
geometria delle chiome per la volta celeste: una matrice d'ombra della vegetazione, una delle
chiome davanti agli edifici, un gruppo di sky view factor della vegetazione. D'inverno OMBRA ha
due [Strati di chioma](../../CONTEXT.md). Alla 0.2.0 fa una passata dello sky view factor per
Strato e somma le due detrazioni: conta due volte la volta che entrambi gli Strati coprono, e la
nota del campo nel Giornale lo dichiara.

L'[ADR 0004](0004-psi-per-cella.md) aveva stabilito che una cella riceve una sola trasmissività
verso il cielo. Lasciava aperte due cose: come due Strati si compongono in una geometria, e come
si calcola quell'unica trasmissività. Le chiude il ticket
[Come comporre più Strati di chioma nel cielo anisotropo](https://github.com/maeurong/OMBRA/issues/33).
Le letture di monte e le misure citate qui sotto stanno in
[`docs/ricerca/03-chiome-riunite.md`](../ricerca/03-chiome-riunite.md).

Qui «nucleo» è il codice vendorato del Motore, in `vendor/solweig/`.

Si decide:

- **La geometria unica verrà da una passata in più, sulle [Chiome riunite](../../CONTEXT.md).**
  Da quella passata usciranno le tre matrici d'ombra, cioè edifici, vegetazione e chiome davanti
  agli edifici, e i dieci sky view factor della vegetazione. Li produrrà il nucleo, e OMBRA li
  prenderà come sono. Dalla 0.3.0 il binario fa la passata quando gli Strati sono due, e dei
  campi della vegetazione ne tiene uno solo: la volta che le Chiome riunite tolgono.
- **«Chiome riunite» entra nel vocabolario.** È un raster di sola geometria: a differenza di uno
  Strato di chioma, non ha una trasmissività.
- **Le passate per Strato restano, e serviranno a una cosa sola: la
  [Trasmissività verso il cielo](../../CONTEXT.md).** Dalla 0.3.0 il binario legge dal nucleo tre
  campi: la volta tolta dal primo Strato, dal secondo, dalle Chiome riunite. La loro differenza
  dice quanta volta copre solo l'uno, solo l'altro, entrambi, e quindi la detrazione: dove coprono
  entrambi passa il prodotto delle due trasmissività. Alla 0.3.0 nessun raster porta la
  Trasmissività verso il cielo: la detrazione entra solo nello sky view factor con le chiome.
- **Dalla 0.3.0 lo sky view factor con le chiome è quello senza chiome meno quella detrazione.**
  La Trasmissività verso il cielo sarà il numero che, messo nella formula di monte
  `svf − (1 − svf_veg) · (1 − psi)` sul campo delle Chiome riunite, dà la stessa detrazione. Il
  Giornale riporta lo sky view factor che il Motore riceverà.
- **Dove due Strati hanno una chioma sulla stessa cella, dalla 0.3.0 le Chiome riunite tengono
  la cima più alta e la zona-tronco più bassa.**
- **La passata sulle Chiome riunite e lo sky view factor che ne esce sono nel binario dalla
  0.3.0**, prima della catena radiativa. Le matrici e i direzionali troveranno un consumatore
  quando il passo fuso sarà cucito, e la 0.3.0 non li conserva. Alla 0.2.0 il binario somma le
  detrazioni per Strato.

## Perché non le alternative

**Comporre le matrici bit a bit, Strato per Strato**, risparmia la passata in più. Sul caso di
riferimento d'inverno dà le stesse matrici della passata riunita: zero differenze su 382.500
coppie di cella e patch, per ciascuna delle due matrici della vegetazione e in ciascuno dei due
Scenari. Per quella delle chiome davanti agli edifici la prova è debole, perché lì un solo Strato
la riempie. Ma il nucleo scrive matrice e campo dalla stessa ombra per patch, e una matrice
composta da noi vuole i suoi campi. Ricalcolarli fuori da `vendor/` vuol dire riscrivere i pesi
per patch, la correzione dell'ultimo anello e i troncamenti del nucleo. Comporli in aritmetica
mette matrice e campo in contraddizione dove due Strati coprono la stessa patch.

**La media delle trasmissività pesata su quanta volta ogni Strato toglie** era la definizione
scritta in `CONTEXT.md`. Tratta la volta coperta da entrambi gli Strati come quella coperta da uno
solo. Lo sky view factor con le chiome che ne esce si scosta dal conto fatto patch per patch sui
bit del nucleo, in valore assoluto, in media di 0,0013 nello stato di fatto e di 0,0043 negli
interventi, al più di 0,010 e di 0,069. Con la differenza fra i tre campi, che è la strada scelta,
lo scarto massimo sulla stessa grandezza resta sotto 10⁻⁶ in entrambi gli Scenari.

**La somma delle detrazioni per Strato**, com'è alla 0.2.0 e come la
[risoluzione sulle due trasmissività](https://github.com/maeurong/OMBRA/issues/18) la conservava,
lascerebbe intatti i numeri pubblicati. Sulla stessa grandezza si scosta in media di 0,0039 nello
stato di fatto e di 0,0143 negli interventi, al più di 0,033 e di 0,188: è il doppio conteggio.

**Tenere, sulla cella condivisa, la chioma più alta con la sua zona-tronco** è la regola che vale
dentro uno Strato. Con questa regola, su una Griglia di prova con quindici celle condivise, la
passata riunita lascia libere 1980 coppie di cella e patch che uno dei due Strati copre, in 1018
celle su 1681: la geometria unica vedrebbe cielo dove c'è una chioma. Con la zona-tronco più bassa
la matrice d'ombra della vegetazione coincide con la composizione bit a bit.

## Cosa comporta

**I valori invernali dello sky view factor con le chiome cambiano fra la 0.2.0 e la 0.3.0.**
Sul caso di riferimento la media passa da 0,7064 a 0,7104 nello stato di fatto e da 0,5934 a
0,6077 negli interventi. Il commit che cambia il calcolo porta la versione a 0.3.0, come
l'[ADR 0005](0005-il-sole-al-centro-dell-ora.md) ha fatto per la 0.2.0, e le Corse già calcolate
restano vere per la versione che il loro Giornale dichiara.

**Con uno Strato solo il risultato resta identico fra la 0.2.0 e la 0.3.0.** Le Chiome riunite
sono quello Strato e la sua passata è la passata riunita; la Trasmissività verso il cielo sarà
la sua trasmissività. Con due Strati, dalla 0.3.0 le passate sono tre. Sul caso di riferimento,
alla 0.2.0 una Corsa spende da 32 a 63 ms nello sky view factor, con una e con due passate; alla
0.3.0 la terza ne aggiunge da 33 a 42 alla Corsa invernale (`vendor/solweig/PROVENIENZA.toml`).
Al tetto di progetto resta da cronometrare.

**L'approssimazione che resterà è quella dell'ADR 0004, liberata dal doppio conteggio.** La
geometria sarà quella del nucleo, e la quota di volta che le chiome lasciano passare sarà esatta
rispetto ai suoi bit. Una cella riceverà comunque **una sola** Trasmissività verso il cielo per
tutte le patch coperte, mentre nel cielo anisotropo le patch hanno luminosità diverse e
nell'isotropo le quattro direzioni hanno coperture diverse. Quanto pesa resta da misurare: la
copia vendorata è priva del modello di Perez. Sul caso di riferimento d'inverno più del 99 per
cento delle celle, in entrambi gli Scenari, vede entrambi gli Strati in qualche patch.

**Le misure valgono per lo sky view factor ricavato direttamente dalla differenza**, che è ciò
che la 0.3.0 calcola. La Trasmissività verso il cielo si otterrà dividendo la detrazione per la
volta che le Chiome riunite tolgono. Rimetterla nella formula di monte in `f32` resta da misurare.

**La differenza fra tre campi vale per due Strati**, che sono quanti la Derivazione ne produce.
Dalla 0.3.0, con più di due il binario si ferma con un errore invece di calcolare.

**Dalla 0.3.0 la cima più alta con la zona-tronco più bassa copre anche il vuoto fra due chiome
che stanno una sopra l'altra.** Resta da misurare: il caso di riferimento è privo di celle condivise, e nella
Griglia di prova le due chiome si toccavano.

**Uno Scenario senza Alberi fa la sola passata senza chioma.** Il nucleo restituisce le due
matrici della vegetazione tutte a uno e i dieci campi della vegetazione a zero, come li ha
allocati. A monte è Python a mettere quei campi a uno
(`pysrc/solweig/models/precomputed.py:143-152` al commit `c31c575`), e OMBRA farà lo stesso.

**Due voci di `CONTEXT.md` sono entrate con questo ADR, la terza con il codice.** «Chiome
riunite» e la nuova «Trasmissività verso il cielo» precedono il codice, perché il piano e il
codice si scrivessero con quelle parole. La voce «Sky view factor» dice che cosa il Giornale
riporta: è cambiata con il commit della 0.3.0, e fino ad allora descriveva la somma per Strato.
