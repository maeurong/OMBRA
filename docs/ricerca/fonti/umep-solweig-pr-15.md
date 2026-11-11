# UMEP-dev/umep-solweig PR #15 — Tgmaps_v1: loop over unique classes and mask on the class grid

URL: https://github.com/UMEP-dev/umep-solweig/pull/15  
Autore: songololo · aperta: 2026-11-11T12:00:57Z · stato: open · head: f16fc20ddb96709458db303d941d858c3f3f9ed4 · ultima modifica: 2026-11-11T12:00:57Z

Catturata con `gh api repos/UMEP-dev/umep-solweig/pulls/15` il 2026-11-11.

---

**Branch:** `songololo:fix/tgmaps-class-mapping` → `UMEP-dev:main`

## Problem

In `Tgmaps_v1.py` the unique class list is overwritten on the next line:

```python
id = np.unique(lc_grid)
id = lc_grid[lc_grid <= 7].astype(int)   # the class of every pixel, in raster order
...
for i in id:
    Tstart[Tstart == i] = ...
    TgK[TgK == i] = ...
```

The loop then replaces values in place, in grids that started as copies of the class grid. Two
things follow.

**Water gets the cobblestone `TgK` and `Tstart` when a water pixel comes before a class-0
pixel.** Water has `Ts_deg = 0.0` and `Tstart = 0.0`. Once a water pixel is processed its cells
hold `0.0`, and when the loop later reaches class 0, `TgK == 0` matches them.

| Class grid | `TgK` | `Tstart` |
| ---------- | ----- | -------- |
| `[0, 7, 5]` | `[0.37, 0.0, 0.21]` | `[-3.41, 0.0, -3.38]` |
| `[7, 0, 5]` | `[0.37, 0.37, 0.21]` | `[-3.41, -3.41, -3.38]` |

The same land cover gives different water parameters depending on where the water lies in the
raster.

**Run time grows with the square of the pixel count.** The loop body compares five full grids
and runs once per pixel: 0.13 s at 100 x 100, 1.8 s at 200 x 200, about 27 s at 400 x 400.

## Change

- Loop over the unique classes at or below 7.
- Take each mask from the class grid, not from the grid being filled.
- Same change in `Tgmaps_v1_torch.py`.

Results are unchanged for any grid where the water case above does not occur.

## Test

`tests/test_tgmaps.py`: each class gets its own parameters in three pixel orders, wall classes
(100 and above) map to roofs, and a 400 x 400 grid finishes in under a second. Three of the five
cases fail on `main`.

I ran the numpy variant only. The torch variant has the same edit but I have no torch
environment to run it in.

## Not changed

A class from 8 to 99 still keeps its class ID as the parameter value (class 9 gets albedo 9),
and classes 3 and 4 still raise `KeyError`, because they have no entry in `Names`. Should an
unknown class raise an error that names it, or fall back to cobblestone with a warning? I can
add either here.


