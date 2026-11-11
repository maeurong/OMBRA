# UMEP-dev/umep-solweig#14 — Ground `Tstart` has the opposite sign when no land-cover grid is used

URL: https://github.com/UMEP-dev/umep-solweig/issues/14  
Autore: songololo · aperta: 2026-11-11T11:56:17Z · stato: open · commenti: 0 · ultima modifica: 2026-11-11T12:01:23Z

Catturata con `gh api repos/UMEP-dev/umep-solweig/issues/14` il 2026-11-11.

---

**Repository:** UMEP-dev/umep-solweig, commit `ff5bc5c` (`main`).

## What happens

`Solweig_run.py` sets the ground `Tstart` in two places, and they disagree in sign.

With a land-cover grid, `Tgmaps_v1.py` (l.19) uses the parameter as it is:

```python
Tstart[Tstart == i] = solweig_parameters["Tstart"]["Value"][...]
```

Without a land-cover grid, `Solweig_run.py` (l.483) subtracts it:

```python
Tstart = Knight - param["Tstart"]["Value"]["Cobble_stone_2014a"]
```

`Knight` is a grid of zeros, so this is the negated parameter. The default parameter file has
`Tstart.Cobble_stone_2014a = -3.41`. A cobblestone pixel therefore gets `Tstart = -3.41` with a
land-cover grid and `Tstart = +3.41` without one. The wall value on the same branch (l.491) is
not negated.

`Tstart` enters the ground temperature amplitude in `Solweig_2025a_calc_forprocessing.py` (l.298):

```python
Tgamp = TgK * altmax + Tstart  # Fixed 2021
```

The two paths differ by 6.82 K in `Tgamp` for the same material.

## Size of the effect

One timestep, 40 x 40 px at 1 m, one 12 m block, no vegetation, isotropic sky, 15 July 13:00 at
57.7 N, Ta 25 degC, global radiation 700 W/m2. Running `Solweig_2025a_calc` with `Tstart = +3.41`
(the no-land-cover value) against `Tstart = -3.41`, everything else equal:

| Output | Max difference | Mean difference |
| ------ | -------------- | --------------- |
| Lup    | 39.3 W/m2      | 35.5 W/m2       |
| Tmrt   | 2.4 degC       | 2.2 degC        |

The same site gives a Tmrt about 2 degC higher at midday when the land-cover grid is left out,
even if every pixel of that grid would have been cobblestone.

## Likely cause

The commented line above l.298 is the older form, `(TgK * altmax - Tstart) + Tstart`. The
subtraction in `Solweig_run.py` may date from when the sign convention of the parameter was the
opposite, and was not updated with the 2021 fix.

## Suggested fix

Use the parameter as it is on the no-land-cover branch, as `Tgmaps_v1` does:

```python
Tstart = Knight + param["Tstart"]["Value"]["Cobble_stone_2014a"]
```

If `+3.41` is the intended value, then `Tgmaps_v1` and the parameter file need the change
instead. Either way the two paths should agree.

## Context

Found by an end-to-end comparison of the Rust port (UMEP-dev/solweig) against
`Solweig_2025a_calc`, with every argument derived as `Solweig_run.py` derives it. The port uses
`-3.41` on both paths and keeps that until this is settled here.


