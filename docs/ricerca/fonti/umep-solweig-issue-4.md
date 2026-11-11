# UMEP-dev/umep-solweig#4 — SOLWEIG water branch tests land-cover code 3 (deciduous) instead of 7 (water)

URL: https://github.com/UMEP-dev/umep-solweig/issues/4  
Autore: songololo · aperta: 2026-08-25T14:48:48Z · stato: open · commenti: 1 · ultima modifica: 2026-08-25T14:56:25Z

Catturata con `gh api repos/UMEP-dev/umep-solweig/issues/4` il 2026-11-11.

---

In the classic SOLWEIG path, the special handling for water tests land-cover code 3:

```python
# functions/SOLWEIGpython/sunonsurface_2018a.py
Tg[lc_grid == 3] = Twater - Ta  # Setting water temperature
```

and the nocturnal Lup override is keyed the same way:

```python
# functions/SOLWEIGpython/Solweig_2025a_calc_forprocessing.py
Lup[lc_grid == 3] = SBC * 0.98 * (Twater + 273.15) ** 4  # nocturnal Water temp
```

The same nocturnal override also appears in `Solweig_2021a_calc_forprocessing.py` (line 208) and `Solweig_2022a_calc_forprocessing.py` (line 264). Line numbers are as of the current pip `umep` package; they may differ slightly between copies of these files.

Per the land-cover convention (the Names table in parametersforsolweig.json), water is code 7 and code 3 is deciduous trees (confirmed in UMEP discussion #872).

Consequences as far as I can tell:

- With a standard land-cover grid, the branch never engages and the `Twater` parameter has no effect. Water surface temperature follows air temperature only because the Water class parameters happen to be `Ts_deg = 0` and `Tstart = 0` (zero-amplitude wave). Setting custom water parameters therefore takes full effect, which is how this surfaced (discussion #872).
- If a grid does contain code 3 (deciduous trees), those pixels would be pinned to the water temperature.

The 2026a ground-surface scheme already keys its water handling on `lc_grid == 7`, so presumably the fix here is to change the two tests above from 3 to 7. That would make `Twater` effective again, which is a behaviour change for scenes with water, so flagging rather than PRing directly.

Found while maintaining the experimental Rust-based solweig package, which mirrors this code and will follow whatever is decided here.

Link: https://github.com/UMEP-dev/UMEP/discussions/872



---

**Commento di songololo, 2026-08-25T14:56:25Z:**

The above referred to the 2025a calc. As far as I can tell, the same issue remains with the new 2026a calc: its default path still handles water through `sunonsurface_2018a`, so the `lc_grid == 3` test there is where the fix would apply. One difference: the nocturnal `Lup` override keyed to `lc_grid == 3` (present in the 2021a/2022a/2025a calcs) is no longer in the 2026a calc, so for 2026a it's just the one place.


