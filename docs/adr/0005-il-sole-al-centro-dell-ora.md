# Il sole di un'Ora si calcola al suo centro

Un record orario di un file EPW non descrive un istante: l'ora `H` copre l'intervallo che finisce
in `H`, e le sue radiazioni sono integrate sui minuti che precedono `H`. A monte, in
[`UMEP-dev/solweig`](https://github.com/UMEP-dev/solweig) al commit letto dalla ricerca, il sole
di quel record si calcola al centro dell'intervallo. Alla 0.1.0 OMBRA lo calcola alle ore intere
contate dall'inizio del Periodo: con l'abbinamento fissato qui sotto, quell'istante è l'inizio
dell'intervallo. A Bastia Umbra, il 21 giugno, fra il sole della 0.1.0 all'inizio e quello di
monte al centro passano fino a 5,39° di altezza e 19,96° di azimut. Quasi tutto lo scarto viene dall'istante: nei due giorni misurati i
due algoritmi solari differiscono al più di 0,47°. La misura è in
[`docs/ricerca/02-sole-meta-passo.md`](../ricerca/02-sole-meta-passo.md), la decisione è stata
presa nel ticket [Istante del sole per un record orario](https://github.com/maeurong/OMBRA/issues/35).

Si decide: **il sole di un'[Ora](../../CONTEXT.md) si calcola al suo centro.** L'Ora `k` di un
Periodo, contata da zero, va da `k` a `k + 1` ore dopo la mezzanotte del primo giorno, nell'ora
del fuso che il file meteo dichiara, senza ora legale. Le corrisponde il record EPW numero
`k + 1` contato dal primo record di quel giorno, e il suo sole è quello di `k + 0:30`. Vale per tutto
ciò che una Corsa calcola in quell'Ora: le ombre, le ore di sole, le serie dei Punti di
osservazione, la verifica dell'ombra nel Giornale, e la radiazione quando la catena radiativa
entrerà.

**Dalla 0.2.0 il binario calcola il sole al centro dell'Ora e rifiuta un file meteo che non è
orario.** Alla 0.1.0 il sole è all'inizio dell'Ora e il campo minuto non viene letto. Né l'una
né l'altra abbinano un record meteo a un'Ora: l'abbinamento, e ciò che qui sotto è scritto al
futuro, arriveranno con la catena radiativa.

## Perché non le alternative

**L'inizio dell'Ora, com'è alla 0.1.0**, non cambierebbe nessun numero già prodotto. Ma terrebbe il
sole mezz'ora indietro rispetto alla radiazione del record che gli sarà abbinato. E la parità con
il codice a monte, che la [mappa della catena radiativa](https://github.com/maeurong/OMBRA/issues/29)
ha fissato come verifica, si otterrebbe solo spostando il sole dentro il test di parità: si
proverebbe una cosa diversa da quella che il binario fa.

**La fine dell'Ora**, cioè l'ora scritta nel file, è la più facile da spiegare a chi apre l'EPW.
Per simmetria sbaglierebbe quanto l'inizio, con il segno opposto; non è stata misurata.

**Il centro per la sola catena radiativa**, lasciando le ombre dove sono, salverebbe i numeri già
prodotti e metterebbe due soli nella stessa Ora: il Giornale verificherebbe un'ombra che non è
quella entrata nella Tmrt.

## Cosa comporta

**`dectime` e `jday` si calcoleranno sull'etichetta, come a monte.** Quando lo strato Python di
monte sarà riscritto in Rust, citando la riga d'origine a ogni formula come la mappa ha
stabilito, l'ora del giorno che pilota la temperatura del suolo e il giorno dell'anno staranno su
`H` e non su `H − 0:30`. A monte convivono due istanti nella stessa Ora, e OMBRA li seguirà: è
un'incoerenza ereditata e dichiarata, perché uniformarli romperebbe la parità sul suolo.

**Le ombre diurne cambiano fra la 0.1.0 e la 0.2.0.** L'Impronta prende la versione del binario e
non la convenzione del sole. Il commit che sposta il sole porta quindi la versione a 0.2.0:
senza, la stessa Impronta nominerebbe due risultati diversi. Fare della convenzione un
ingrediente dell'Impronta è stato scartato: sarebbe un parametro con un solo valore possibile,
che non distinguerebbe mai due Corse. Le Corse già calcolate restano vere per la versione che il
loro Giornale dichiara.

**Dalla 0.2.0 le ore di sole sono campionate dentro l'Ora che dichiarano.** Sommare un'ombra per
Ora è un'integrazione. Il sole a `k` ore rappresenta l'intervallo da `k − 0:30` a `k + 0:30`, non
quello da `k` a `k + 1`; il sole al centro rappresenta l'Ora, senza una chiamata in più al
Motore.

**Un'Ora a cavallo dell'alba o del tramonto potrà avere il sole sotto l'orizzonte e radiazione
nel record.** Il centro cade di notte, la media oraria no. Come a monte si tratta quell'Ora non
è stato ancora letto: è annotato nella mappa fra le cose da verificare.

**Dalla 0.2.0 il file meteo deve essere orario.** Alla 0.1.0 OMBRA non legge il campo minuto, e
un file sub-orario viene letto senza errori. Dalla 0.2.0 un file con un record per ora e il
minuto a 0 o a 60 è orario: il lettore prende l'ora dal campo ora, e il Giornale annota i valori
che il campo minuto assume nel file. Più record per la stessa ora, o un altro minuto, sono un
errore detto prima che il Motore sia chiamato. Un file a cui mancano delle ore non è rilevato.
OMBRA accetta così i file con minuto 60, che il dizionario EPW dichiara validi e che a monte, al
commit letto dalla ricerca, vengono scartati; per confrontarli il test di parità darà a monte una
copia con il minuto a 0.
