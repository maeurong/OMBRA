# Ricerche

Punto d'ingresso delle ricerche di OMBRA. Una riga per ricerca: il file, la domanda che copre, lo stato, la data. Le ricerche precedenti a questo indice stanno in `research/` e restano lì.

| File | Domanda | Stato | Data |
|---|---|---|---|
| [`../../research/catena-radiativa.md`](../../research/catena-radiativa.md) | Chiusura delle dipendenze del passo fuso di monte, trasformazioni per estrarlo, strato Python da portare (dossier d'ingresso della mappa #29) | fatta; §8 superato da `01-passo-fuso.md` | 2026-10-29 |
| [`01-passo-fuso.md`](01-passo-fuso.md) | Dove va `psi` e se regge un raster, `svf_aveg` contro `svf_veg_blocks_bldg_sh`, `direct_rad` normale o orizzontale, cosa succede di notte (#30) | fatta | 2026-10-29 |
| [`02-sole-meta-passo.md`](02-sole-meta-passo.md) | A quale istante monte, OMBRA ed EPW collocano il sole di un record orario, e lo scarto a Bastia (#31) | fatta | 2026-10-29 |

**Pin e numeri di riga.** Le tre ricerche leggono monte al commit `02246ab71a3a8b127d740dde9640449ee9d558ff`, e i loro `percorso:riga` restano riferiti a quello. Il 2026-11-10 il pin della copia vendorata è passato a `c31c5758236078b6b5f7bc9a931e2fce264bfc88` (v0.1.0b98). Fra i due commit cambiano dodici file sotto `rust/src/`, fra cui `skyview.rs`, `pipeline.rs`, `gvf.rs`, `ground_surface.rs` e `vegetation.rs`, e otto sotto `pysrc/solweig/`: su quei file una riga citata può essere scalata. L'elenco completo lo dà `git diff --stat 02246ab c31c575` in un clone di monte. Le ricerche non si rinumerano: sono il verbale di ciò che è stato letto.
