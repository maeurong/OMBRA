# UMEP-dev/umep-solweig#13 — Solweig_2026a_calc with groundScheme = 0: two lines that differ from 2025a

URL: https://github.com/UMEP-dev/umep-solweig/issues/13  
Autore: songololo · aperta: 2026-11-10T16:27:27Z · stato: open · commenti: 0 · ultima modifica: 2026-11-10T16:27:27Z

Catturata con `gh api repos/UMEP-dev/umep-solweig/issues/13` il 2026-11-11.

---

Two small observations from comparing `Solweig_2026a_calc` (ground scheme off) with `Solweig_2025a_calc`. The first can change Lup when `Tgdiff` is negative; the second affects only the point output of ground temperature. I have not measured either. Checked at `ff5bc5c`.

**1. The negative-temperature clamp acts on a different variable.** In 2025a the clamp is applied to the value that goes into `gvf_2018a`:

```python
Tg = Tg * CI_TgG
if landcover == 1:
    Tg[Tg < 0] = 0
... gvf_2018a(..., Tg, ...)
```

In 2026a the same lines are (L402-413, L475-484):

```python
Tgdiff = Tgdiff * CI_TgG
if landcover == 1:
    Tg[Tg < 0] = 0
... gvf_2018a(..., Tgdiff, ...)
```

`Tg` here is the value carried over from the previous timestep, and `Tgdiff`, which is passed to `gvf_2018a`, is no longer clamped.

**2. The wave delay for the point output takes `Tgdiff` as its stored state.** L518-521:

```python
TgTemp = Tgdiff * shadow + Ta
_, timeadd, Tg = TsWaveDelay_2015a(TgTemp, firstdaytime, timeadd, timestepdec, Tgdiff)
```

The last argument of `TsWaveDelay_2015a` is the state from the previous timestep. 2025a passes and returns a dedicated state (`TgOut1`). Here the current `Tgdiff` is passed instead, and the returned state is stored in `Tg`.

If both are intended, please close this.

