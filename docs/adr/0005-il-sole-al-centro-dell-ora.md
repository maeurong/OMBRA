# Il sole di un'Ora si calcola al suo centro

Un record orario di un file EPW non descrive un istante: l'ora `H` copre l'intervallo che finisce
in `H`, e le sue radiazioni sono integrate sui minuti che lo precedono. Alla versione 0.1.0 OMBRA
calcola il sole all'inizio di quell'intervallo; [`UMEP-dev/solweig`](https://github.com/UMEP-dev/solweig)
lo calcola al centro. A Bastia Umbra, il 21 giugno, fra le due convenzioni passano fino a 5,39° di
altezza e 19,96° di azimut, e quasi tutto lo scarto è l'istante: i due algoritmi solari
differiscono al più di 0,47°. La misura è in
[`docs/ricerca/02-sole-meta-passo.md`](../ricerca/02-sole-meta-passo.md).

Si decide: **il sole di un'[Ora](../../CONTEXT.md) si calcola al suo centro.** L'Ora `k` di un
Periodo, contata da zero, porta il record EPW con ora `k + 1`, copre l'ora solare locale da `k` a
`k + 1` e ha il sole a `k + 0:30`. Vale per tutto ciò che una Corsa calcola in quell'Ora: le ombre,
le ore di sole, le serie dei Punti di osservazione, la verifica dell'ombra nel Giornale, la
radiazione. Un'Ora ha un solo sole.

## Perché non le alternative

**L'inizio dell'Ora, com'è alla 0.1.0**, non cambia nessun numero già prodotto. Ma tiene il sole
mezz'ora indietro rispetto alla radiazione che lo accompagna. E la parità con monte, che la
[mappa della catena radiativa](https://github.com/maeurong/OMBRA/issues/29) ha fissato come
verifica, si otterrebbe solo spostando il sole dentro il banco di prova: si proverebbe una cosa
diversa da quella che il binario fa.

**La fine dell'Ora**, cioè l'ora scritta nel file, è la più facile da spiegare a chi apre l'EPW.
Sbaglia quanto l'inizio, con il segno opposto.

**Il centro per la sola catena radiativa**, lasciando le ombre dov'erano, salva i numeri già
prodotti e mette due soli nella stessa Ora: il Giornale verificherebbe un'ombra che non è quella
entrata nella Tmrt.

## Cosa comporta

**`dectime` e `jday` restano sull'etichetta, come a monte.** L'ora del giorno che pilota la
temperatura del suolo e il giorno dell'anno si calcolano su `H`, non su `H − 0:30`. Monte tiene
due istanti nello stesso passo, e OMBRA lo segue perché lo strato Python si porta citando la riga
di monte. È un'incoerenza ereditata e dichiarata: uniformare i due istanti romperebbe la parità
sul suolo.

**Ogni numero d'ombra prodotto alla 0.1.0 cambia.** L'Impronta prende la versione del binario e
non la convenzione del sole. Il commit che sposta il sole porta quindi la versione a 0.2.0: senza,
la stessa Impronta nominerebbe due risultati diversi. Le Corse già calcolate restano vere per la
versione che il loro Giornale dichiara.

**Le ore di sole diventano una stima centrata.** Sommare un'ombra per Ora è un'integrazione, e il
sole all'inizio la campiona mezz'ora prima dell'intervallo che dichiara. Con il sole al centro
lo sbilancio sparisce, senza una chiamata in più al Motore.

**Un'Ora a cavallo dell'alba o del tramonto può avere il sole sotto l'orizzonte e radiazione nel
record.** Il centro cade di notte, la media oraria no. Come monte tratta quell'Ora non è stato
ancora letto: è annotato nella mappa fra le cose da verificare.

**Il file meteo deve essere orario.** Un record per ora, con il campo minuto a 0 o a 60: l'ora la
dà il campo ora. Più record per la stessa ora sono un errore detto prima del calcolo, non una
Corsa sbagliata in silenzio. OMBRA accetta così file che il dizionario EPW dichiara validi e che
monte, al commit letto dalla ricerca, scarta; per confrontarli il banco di parità darà a monte
una copia con il minuto a 0.
