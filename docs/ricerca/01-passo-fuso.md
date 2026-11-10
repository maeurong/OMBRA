# 01 — Quattro fatti del passo fuso di monte

Ticket: maeurong/OMBRA#30 (parte di #29). Dossier d'ingresso: `research/catena-radiativa.md`
§4 (righe 187-200), §5 (righe 204-234), §8 (riga 286).

**Fonte unica: codice di monte al commit pinnato.** `UMEP-dev/solweig` HEAD
`02246ab71a3a8b127d740dde9640449ee9d558ff`, clone in `/tmp/monte` (`git -C /tmp/monte rev-parse HEAD`).
Ogni `percorso:riga` sotto è relativo a quel clone e a quel commit. Nessuna pagina web: niente
`fonti/`, il commit è il riferimento immutabile.

Tag: `[V]` letto nel sorgente · `[M]` misurato con comando · `[INF]` inferenza · `[NON TROVATO]`.

Lavoro fatto da: OMBRA branch `research/passo-fuso`, HEAD `fdd9e13`.

## Premesse del dossier corrette

Tre affermazioni del dossier e del brief non reggono alla lettura. Non cambiano le domande, ne
cambiano le risposte.

1. **`pipeline.rs:1149` non è un ritorno notturno.** È il `return TimestepResultRaw` del ramo
   dello schema di suolo 2026a, aperto da `if use_scheme {` a `rust/src/pipeline.rs:796` e chiuso
   a `:1172`. `use_scheme = sch.is_some()` (`:647`), cioè `ground_scheme` presente. Nella chiusura
   (`:709`-`:1953`) non c'è nessun altro `return` che dipenda dal sole: `grep -n "return\b"
   rust/src/pipeline.rs` trova solo `:1149` e il `return;` di un pixel non valido a `:1320`. `[V]`
2. **Tre degli otto siti di `psi` stanno fuori dal percorso che OMBRA esegue.** `:826`, `:862`,
   `:1090` sono dentro il ramo 2026a (`:796`-`:1172`), che il dossier §3 lascia spento
   (`ground_scheme = None`). `:1571` è dentro `#[cfg(feature = "gpu")]` (`:1549`), e il manifesto
   vendorato non dichiara la feature (`vendor/solweig/Cargo.toml:22-25` del repo OMBRA). `[V]`
3. **`:826` e `:1383` non sono letture fisiche ma legature** (`let psi = weather.psi;`): il
   valore entra nella fisica solo dove `psi` viene passato. `[V]`

---

## 1. `psi` a valle: raggio o cielo, e se le firme reggono un raster

`weather.psi` è un `f32` (`rust/src/pipeline_scalars.rs:43`). Lo calcola Python per passo,
`psi = compute_transmissivity(doy, physics, conifer)` (`pysrc/solweig/computation.py:277`), e lo
passa a `WeatherScalars(psi=float(psi))` (`:351`). `[V]`

| Sito | Dove va `psi` | Raggio o cielo | Firma oggi | Percorso OMBRA |
|---|---|---|---|---|
| `pipeline.rs:744` | `bldg_sh - &((1.0 - veg_sh) * (1.0 - weather.psi))`, espressione in linea | **raggio**: `veg_sh` è l'ombra delle chiome sul raggio del sole, da `calculate_shadows_rust(weather.sun_azimuth, weather.sun_altitude, …)` (`:718-738`) | nessuna funzione; `weather.psi: f32` | attivo |
| `pipeline.rs:862` | `compute_ani_lum_from_packed(shmat_a, vegshmat_a, lv.column(2), psi, valid_v)` | cielo (patch della volta) | `psi: f32` (`pipeline_radiation.rs:378`) | spento (2026a) |
| `pipeline.rs:1090` | `kside_veg_isotropic_pure(…, psi, …)` | cielo | `psi: f32` (`vegetation.rs:363`) | spento (2026a) |
| `pipeline.rs:1571` | `ctx.dispatch_begin(…, psi, rad_tot)` | cielo: `diffsh = sh - (1 - vsh) * (1 - params.psi)` (`gpu/anisotropic_sky.wgsl:205-206`) | `psi: f32` (`gpu/aniso_gpu.rs:263`), campo di un uniform (`anisotropic_sky.wgsl:42`) | spento (GPU) |
| `pipeline.rs:1699` | `compute_ani_lum_from_packed(shmat_a, vegshmat_a, lv_col2, psi, valid_v)` | cielo | `psi: f32` (`pipeline_radiation.rs:378`) | attivo, cielo anisotropo CPU |
| `pipeline.rs:1803` | `kside_veg_isotropic_pure(…, psi, …)` | cielo | `psi: f32` (`vegetation.rs:363`) | attivo, cielo isotropo |

Tutti `[V]`, per lettura.

**Dove `psi` entra nelle funzioni a valle.** `[V]`

- `compute_ani_lum_from_packed` (`rust/src/pipeline_radiation.rs:374-429`): `psi` compare solo in
  `let diff = sh - (1.0 - vsh) * (1.0 - psi);`, dentro il ciclo per pixel e per patch (`:402`
  nel ramo contiguo, `:421` nel ripiego non contiguo). Il pixel `(r, c)` è già noto in entrambi i
  punti (`:388-389`, `:409-413`).
- `kside_veg_isotropic_pure` (`rust/src/vegetation.rs:348-479`): `psi` compare solo in
  `svfviktbuveg{E,S,W,N}[(r, c)] = vwall + vveg * (1.0 - psi);` (`:413`, `:415`, `:417`, `:419`),
  dentro il doppio ciclo `for r … for c` (`:405-421`). È il peso della parte vegetale delle SVF
  direzionali sulla diffusa: cielo. La componente diretta della stessa funzione (`:423-460`) usa
  `shadow`, che il `psi` del raggio l'ha già dentro da `:744`. `[V]`
- `anisotropic_sky_pure` (`rust/src/sky.rs:236-264`) **non riceve `psi`**: tratta le chiome come
  bit d'ombra (`:330-333`, `:505-507`). `[V]`
- Fuori dal Rust c'è un nono punto, già raster: `svfbuveg = svf - (1 - svf_veg) * (1 - psi)`, con
  clip e cast a `float32` (`pysrc/solweig/components/svf_resolution.py:141-142`), applicato a
  `computation.py:286` ed entrato nel Rust come `svfbuveg_v` (`pipeline.rs:568`, `:585`). È
  cielo. Il dossier lo ha già come `SkyViewFactor::con_le_chiome`. `[V]`

**Conteggio rispetto ad ADR 0004.** Usi fisici nel Rust: 1 raggio (`:744`) e 5 cielo (`:862`,
`:1090`, `:1571`, `:1699`, `:1803`), più `svfbuveg` in Python. Con le due legature si arriva a 8
righe, che è il conteggio del dossier. Sul percorso che OMBRA esegue (classico, CPU) i punti
toccati sono 1 raggio (`:744`) più 1 cielo per modo (`:1699` anisotropo, `:1803` isotropo), più
`svfbuveg`. Il «sette per il cielo» dell'ADR vale solo se si contano anche i siti spenti e le
legature. `[V]` sul conteggio, `[INF]` sulla lettura che ne dà l'ADR.

**Cosa cambierebbe con un raster.** `[INF]` dalla lettura

- `:744`: `(1.0 - weather.psi)` diventa un raster `1 - psi_raggio`, moltiplicato elemento per
  elemento. Nessuna firma cambia, perché è un'espressione in linea. Il raster va indicizzato sulla
  cella che riceve l'ombra, non sulla chioma che la proietta: è la «trasmissività lungo il
  raggio» per ora che ADR 0004 già prevede.
- `compute_ani_lum_from_packed`: `psi: f32` diventa `psi: ArrayView2<f32>`, e si legge
  `psi[[r, c]]` alle righe `:402` e `:421`. Due righe di corpo.
- `kside_veg_isotropic_pure`: `psi: f32` diventa `psi: ArrayView2<f32>`, e si legge `psi[(r, c)]`
  alle righe `:413-419`. Quattro righe di corpo.
- `dispatch_begin` (GPU): `psi` sta in un uniform, quindi un raster vorrebbe un buffer di storage e
  un cambio di shader. Il percorso è spento in OMBRA e resta fuori.

La fisica non cambia: in tutti i siti CPU `psi` entra già per cella, dentro il ciclo sul pixel, e
la formula per cella resta la stessa.

**Byte per byte con un raster costante.** `[M]` sulle formule isolate. Ho riscritto i tre kernel
(`:744`, `vegetation.rs:413`, `pipeline_radiation.rs:402`) in Rust puro, una volta con `psi`
scalare e una con `psi` letto da un `Vec` costante, su 10⁶ celle casuali e per
`psi ∈ {0.03, 0.5, 0.1234567}`. Il confronto è fatto sui bit (`to_bits`), anche sulla somma
accumulata di `ani_lum`.

```
rustc 1.98.0 (88d9e12ae 2026-08-18) (Homebrew)
rustc -O …/scratchpad/psi/main.rs && …/scratchpad/psi/t
psi=0.03: 744 ok, kside ok, ani_lum ok (bit-identici su 1000000 celle)
```

Lo stesso esito si ripete per gli altri due valori. Script:
`/private/tmp/claude-501/-Users-mario-GitHub/89c63284-d20f-45d5-ace4-5f46359bb090/scratchpad/psi/main.rs`.
Questo **non** sostituisce il test di parità chiesto da ADR 0004 sul passo intero: prova solo che
le stesse operazioni IEEE, nello stesso ordine, danno gli stessi bit. Rust non contrae `a*b+c` in
FMA senza richiesta esplicita. `[INF]`

---

## 2. `svf_aveg*` ↔ `svf_veg_blocks_bldg_sh*`

**È la stessa grandezza con due nomi.** `[V]`, catena letta per intero:

1. `calculate_svf_inner` (`rust/src/skyview.rs:330`) riempie `SvfIntermediate` (`:256-275`), che
   ha i campi `svf_veg_blocks_bldg_sh{,_n,_e,_s,_w}` (`:267-271`). Li accumula **nella stessa
   passata** che produce `svf` e `svf_veg`, patch per patch, da
   `shadow_result.veg_blocks_bldg_sh` di `compute_svf_shadows` (`:639-655`), con gli stessi pesi
   `pw.weight_*` (`:608`) usati per `svf` (`:610-622`) e `svf_veg` (`:624-637`). Poi li tronca a 1
   (`:739-743`) e li mette a NaN dove il DSM è NaN (`:765-779`). Anche il percorso GPU scrive in
   quegli stessi campi, da `svf.svf_aveg*` (`:527-541`).
2. Verso Python, `svf_intermediate_to_py` li esporta come `SvfResult.svf_veg_blocks_bldg_sh{,_north,…}`
   (`skyview.rs:837-841`).
3. `SvfArrays.from_rust_result` li rinomina: `svf_aveg=np.array(svf_result.svf_veg_blocks_bldg_sh)
   if use_veg else ones.copy()`, e lo stesso per `_north/_east/_south/_west`
   (`pysrc/solweig/models/precomputed.py:148-152`). Lo stesso rinomino sta nel percorso a
   piastrelle (`models/surface_svf_tiled.py:389-393`). Chiamanti: `models/surface_compute.py:347`,
   `models/surface.py:1405`.
4. `resolve_svf` copia `svf_data.svf_aveg*` nel bundle senza trasformarli
   (`components/svf_resolution.py:50-56`, ramo `surface.svf`; `:75-81`, ramo `precomputed.svf`).
5. `calculate_core_fused` li passa in undicesima-quindicesima posizione a `pipeline.SvfBundle(…)`
   (`computation.py:522-526`), che li riceve come `svf_aveg{,_n,_e,_s,_w}`
   (`rust/src/pipeline_bundles.rs:64-68`). L'ordine N, E, S, W combacia.
6. Nel passo fuso li usano `compute_ldown(…, svf_aveg_v, …)` (`pipeline.rs:1781`) e `lside_veg_pure`
   (`:1825-1828`).

**Da quale chiamata escono.** Da una sola: la stessa invocazione di `calculate_svf_inner` produce
`svf*`, `svf_veg*` e `svf_veg_blocks_bldg_sh*`. I tre punti d'ingresso (`skyview.rs:964`, `:1036`,
`:1079`) chiamano tutti la stessa funzione e restituiscono tutti i campi. `[V]`

**Che cosa misura.** Lo spiega l'elaborazione dell'ombra `veg_blocks_bldg_sh`
(`rust/src/shadowing.rs:572-585`), che replica `vbshvegsh` di UMEP: binarizza, sottrae `veg_sh`,
inverte. È la frazione di cielo che le chiome non oscurano, contando anche le chiome che
stanno nell'ombra degli edifici. `[V]` sul codice, `[INF]` sull'interpretazione.

**Per OMBRA.** Con `use_veg` falso i campi diventano `ones` (`precomputed.py:148-152`). Nel
`skyview.rs` vendorato i campi ci sono (`grep -c svf_veg_blocks_bldg_sh
vendor/solweig/src/skyview.rs` → 42), ma oggi `src/motore.rs` legge solo `svf` e `svf_veg`
(dossier §5). `[M]` sul conteggio. La composizione per più Strati del dossier §6 tocca anche
questa grandezza, ed è una questione della Fase 3. `[INF]`

---

## 3. `direct_rad`: normale (DNI) o orizzontale

**È irradianza diretta normale (DNI).** `[V]` su tre prove indipendenti:

1. **Da EPW passa così com'è.** `Weather.from_epw` costruisce
   `measured_direct_rad=float(row["dni"])` e `measured_diffuse_rad=float(row["dhi"])`
   (`pysrc/solweig/models/weather.py:459-460`). `Weather.compute` li copia senza conversione:
   `self.direct_rad = self.measured_direct_rad` (`:217-220`). Lo stesso succede con `kdir` dal
   file forcing UMEP (`:592`, `:608-609`).
2. **Quando la calcola, la divide per sin(altitudine).** `diffusefraction` (Reindl 1990) restituisce
   `radI = (radG - radD) / sin_alfa` (`pysrc/solweig/physics/diffusefraction.py:37-38`), cioè
   la componente normale. La chiamata sta a `weather.py:256-258`. Anche `rad_g0` la
   riproietta: `rad_g0 = rad_i0 * np.sin(alt) + rad_d0` (`computation.py:330`).
3. **Il Rust la moltiplica per sin(altitudine) sul piano orizzontale.** `WeatherScalars.direct_rad
   = float(weather.direct_rad)` (`computation.py:343`), poi `rad_i = weather.direct_rad`
   (`pipeline.rs:1380`) e `*kd = rad_i * shadow[[r, c]] * sin_alt + drad + …`
   (`pipeline_radiation.rs:174`), con `sin_alt = sin(sun_altitude)` (`pipeline.rs:1379`). Per la
   persona in piedi usa invece `cos(altitudine)`: `KsideI = shadow * radI * cos(altitude)`
   (`vegetation.rs:427`). Perez usa `(Dh + I)/Dh` (`perez.rs:265`), che è la definizione con `I`
   normale.

**Conseguenza per OMBRA.** L'EPW dà già DNI e DHI, quindi `direct_rad = DNI` e
`diffuse_rad = DHI` senza conversione, esattamente come fa monte. `[INF]` dalla lettura

**Caveat.** Con i valori misurati, `direct_rad` **non** viene azzerato di notte: resta quello
che dice il file (`weather.py:217-220`). L'azzeramento a `0.0` scatta solo nel ramo calcolato
(`:259-262`). Nel ramo misurato `clearness_index` vale `1.0` di notte (`:239`) e si calcola con
`clearnessindex_2013b` di giorno (`:222-233`). `[V]`

---

## 4. Il passo di notte

**Non c'è un ritorno anticipato.** Vedi le premesse corrette, punto 1. Quando il sole è sotto
l'orizzonte, la chiusura esegue l'intero percorso classico (ombre → suolo → GVF → ritardo termico
→ radiazione → Tmrt) e chiude sul `TimestepResultRaw` finale (`pipeline.rs:1933-1951`). Python
chiama `compute_timestep` per ogni passo, giorno o notte (`computation.py:606`), e non salta le
ore notturne. `[V]`

**Che cosa vale di notte**, con `is_daytime = sun_altitude > 0` (`computation.py:352`, identico
a `weather.py:270-271`):

- **Ombre: tutto al sole.** `calculate_shadows_rust` con `altitude_deg < 0.0` restituisce
  `bldg_sh`, `veg_sh`, `veg_blocks_bldg_sh` tutti a 1 senza tracciare (`shadowing.rs:212-226`).
  A `:744` viene quindi `shadow = 1 - 0·(1-psi) = 1`: di notte il `psi` del raggio non conta. `[V]`
  Per altitudini fra 0° e 3° (`min_sun_altitude = 3.0`, `pipeline.rs:736`) il comportamento non
  l'ho letto. `[NON TROVATO]`
- **Suolo:** `tg = 0` e `tg_wall = 0` (`pipeline.rs:780-789`), quindi `tg_temp = 0·shadow + ta =
  ta` (`:1354`). `[V]`
- **Radiazione:** `direct_rad = 0` e `diffuse_rad = global_rad` nel ramo calcolato
  (`weather.py:259-262`), `clearness_index = 1.0` (`:265`, `:239`). Kdown e Kside si riducono ai
  termini di diffusa e riflessa, che valgono zero se `global_rad` è zero. Nel cielo anisotropo
  `lum_chi` si azzera (`pipeline.rs:1520-1535`). Ldown, Lup e Tmrt si calcolano per intero.
  `[V]` sul codice, `[INF]` sul valore zero, che dipende dal file meteo.
- **Campi restituiti:** `tmrt`, poi `shadow`/`kdown`/`kup`/`ldown`/`lup` secondo `output_mask`
  (`:1934-1939`). Seguono `timeadd`, `tgmap1*` e `tgout1`, che vengono da
  `ts_wave_delay_batch_pure` (`:1940-1946`), e `tg`/`rn`/`rn_past`/`g` a `None` (`:1948-1951`).
  `[V]`

**Lo stato che il chiamante aggiorna** (`computation.py:637-667`). `[V]`

1. Copia sempre dal Rust `state.timeadd = result.timeadd` e i sei raster `tgmap1`, `tgmap1_e/s/w/n`
   e `tgout1` (`:637-650`).
2. Poi sovrascrive in base al giorno (`:653-657`): di giorno `firstdaytime = 0.0`; di notte
   `firstdaytime = 1.0` e `timeadd = 0.0`, **scartando** il `timeadd` appena restituito dal Rust.
3. Solo con lo schema 2026a attivo porta avanti `tg`, `rn`, `rn_past`, `g` e `shadow_past`
   (`:661-667`). In OMBRA no.
4. `timestep_dec` non lo tocca: arriva dallo stato (`computation.py:575`).

**Che cosa ne segue nel ritardo termico.** `[INF]` dalla lettura di `ground.rs:134-226`. Con
`firstdaytime == 1`, `tgmap1*` e `tgout1` vengono riassegnati agli ingressi correnti
(`ground.rs:159-166`). Con `timeadd = 0`, sotto la soglia di 59/1440 si prende il ramo `else`
(`:201-220`): `lerp(lup, tgmap1 = lup) = lup`. Dal secondo passo notturno in poi il ritardo è
quindi l'identità. Il primo passo notturno lo calcola ancora con `firstdaytime = 0`, arrivato
dall'ultimo passo di giorno. Il primo passo di giorno riparte con `firstdaytime = 1`. È la logica
di stato che il dossier §5 chiede di coprire con un test su più passi: va portata riga per riga
da `computation.py:653-657`, non ricavata.

---

## Cosa resta incerto

- La parità byte per byte di `svfbuveg` calcolato in Rust f32 contro numpy
  (`svf_resolution.py:141-142`): numpy riceve `psi` come `float` Python, a 64 bit, e la
  promozione dipende dalla versione di numpy e dal dtype di `svf`. Non verificato. `[NON TROVATO]`
- Le ombre fra 0° e 3° di altitudine (`min_sun_altitude = 3.0`). Non lette. `[NON TROVATO]`
- Il test di ADR 0004 sul passo intero, con raster costante e confronto byte per byte contro
  monte. Qui sono provate solo le formule isolate. `[M]` parziale
