"""Measure whether solweig's thermal state forgets the previous days.

Research script for docs/ricerca/04-stato-termico.md (OMBRA #21). Not part of
OMBRA. Runs against the solweig 0.1.0b101 wheel (tag v0.1.0b101 = commit
41840b74cd7c003f88dbde0c30fff736cb51bd5d).

Run A: days 1-3 from the initial state. Run B: days 2-3 from the initial state.
Compares, at every common timestamp, Tmrt, Ldown, Lup and the carried state
(tgmap1, tgout1, timeadd, firstdaytime) bit for bit.

Usage: python -I 04-stato-termico-misura.py
"""

import json
import sys
import tempfile
from datetime import datetime, timedelta

import numpy as np
import solweig
import solweig.api as sapi
from solweig import Location, SurfaceData, Weather
from solweig.physics import sun_position as sp
from solweig.physics.clearnessindex_2013b import clearnessindex_2013b

OUTS = {"tmrt", "shadow", "kdown", "kup", "ldown", "lup"}


def surface():
    dsm = np.zeros((40, 40), dtype=np.float32)
    dsm[14:24, 14:24] = 12.0  # one block, 12 m
    return SurfaceData.prepare(dsm=dsm, pixel_size=1.0)


def synth_weather(loc, day0, n_days, sky):
    """Hourly records stamped at the END of the hour (solweig convention).

    sky[d] multiplies the clear-sky global irradiance of day d (1.0 = clear).
    """
    out = []
    for k in range(1, 24 * n_days + 1):
        t = day0 + timedelta(hours=k)
        mid = t - timedelta(minutes=30)
        d = (k - 1) // 24
        sun = sp.sun_position(
            {"year": mid.year, "month": mid.month, "day": mid.day, "hour": mid.hour,
             "min": mid.minute, "sec": 0, "UTC": loc.utc_offset},
            loc.to_sun_position_dict(),
        )
        zen = float(np.asarray(sun["zenith"]).flat[0])
        h = mid.hour + mid.minute / 60.0
        ta = 24.5 - 6.5 * np.cos(2 * np.pi * (h - 3.0) / 24.0)  # min 18 at 03, max 31 at 15
        if zen < 90.0:
            i0 = clearnessindex_2013b(np.radians(zen), mid.timetuple().tm_yday, ta, 0.5, 500.0,
                                      loc.to_sun_position_dict(), 1013.25)[0]
            g = max(0.0, float(i0) * sky[d])
        else:
            g = 0.0
        out.append(Weather(datetime=t, ta=float(ta), rh=50.0, global_rad=g, ws=1.0))
    return out


def run(surf, loc, series, workdir):
    rec = {}
    orig = sapi._calculate_single

    def spy(*args, **kwargs):
        kwargs["_requested_outputs"] = OUTS
        r = orig(*args, **kwargs)
        w = kwargs["weather"]
        s = kwargs["state"]
        rec[w.datetime] = {
            "tmrt": np.array(r.tmrt, copy=True),
            "ldown": np.array(r.ldown, copy=True),
            "lup": np.array(r.lup, copy=True),
            "tgmap1": np.array(s.tgmap1, copy=True),
            "tgout1": np.array(s.tgout1, copy=True),
            "timeadd": float(s.timeadd),
            "timestep_dec": float(s.timestep_dec),
            "firstdaytime": float(s.firstdaytime),
            "ci": float(w.clearness_index),
            "alt": float(w.sun_altitude),
        }
        return r

    sapi._calculate_single = spy
    try:
        solweig.calculate(surf, series, loc, output_dir=workdir)
    finally:
        sapi._calculate_single = orig
    return rec


def compare(a, b, label):
    rows = []
    for t in sorted(b):
        ra, rb = a[t], b[t]
        row = {"t": t.isoformat(), "alt": round(ra["alt"], 2),
               "ci_A": round(ra["ci"], 4), "ci_B": round(rb["ci"], 4)}
        for f in ("tmrt", "ldown", "lup", "tgmap1", "tgout1"):
            x, y = ra[f], rb[f]
            same = bool(np.array_equal(x, y, equal_nan=True))
            d = np.abs(x.astype(np.float64) - y.astype(np.float64))
            row[f] = "=" if same else f"{np.nanmax(d):.3g}"
        row["timeadd"] = "=" if ra["timeadd"] == rb["timeadd"] else f"{ra['timeadd']:.5f}/{rb['timeadd']:.5f}"
        row["fdt"] = "=" if ra["firstdaytime"] == rb["firstdaytime"] else f"{ra['firstdaytime']}/{rb['firstdaytime']}"
        rows.append(row)
    n_diff = sum(1 for r in rows if any(r[f] != "=" for f in ("tmrt", "ldown", "lup", "tgmap1", "tgout1", "timeadd", "fdt")))
    print(f"\n=== {label}: {len(rows)} common steps, {n_diff} with any difference")
    print("t                    alt     ciA    ciB   tmrt     ldown    lup      tgmap1   tgout1   timeadd fdt")
    for r in rows:
        if any(r[f] != "=" for f in ("tmrt", "ldown", "lup", "tgmap1", "tgout1", "timeadd", "fdt")):
            print(f"{r['t']:20} {r['alt']:6.2f} {r['ci_A']:6.3f} {r['ci_B']:6.3f} {r['tmrt']:8} {r['ldown']:8} "
                  f"{r['lup']:8} {r['tgmap1']:8} {r['tgout1']:8} {r['timeadd']:7} {r['fdt']}")
    return {"label": label, "common_steps": len(rows), "steps_with_difference": n_diff, "rows": rows}


def case(surf, loc, day0, sky, label):
    with tempfile.TemporaryDirectory() as wa, tempfile.TemporaryDirectory() as wb:
        full = synth_weather(loc, day0, 3, sky)
        a = run(surf, loc, full, wa)
        b = run(surf, loc, [w for w in full if w.datetime > day0 + timedelta(days=1)], wb)
    # rebuild B from fresh Weather objects too: _precompute_weather mutates them
    with tempfile.TemporaryDirectory() as wb2:
        fresh = synth_weather(loc, day0, 3, sky)[24:]
        b2 = run(surf, loc, fresh, wb2)
    same_b = all(np.array_equal(b[t]["tmrt"], b2[t]["tmrt"], equal_nan=True) for t in b)
    print(f"[{label}] B with reused vs fresh Weather objects identical: {same_b}")
    return compare(a, b2, label)


def case_gaps(surf, loc, day0):
    """Q6: a hole in the series, and a first gap unlike the others."""
    import logging

    msgs = []

    class H(logging.Handler):
        def emit(self, r):
            msgs.append(r.getMessage())

    logging.getLogger("solweig.timeseries").addHandler(H())
    full = synth_weather(loc, day0, 1, [1.0])
    series = {
        "hole 11:00-15:00 (records 10:00 then 16:00)": [w for w in full if not 10 < w.datetime.hour < 16],
        "first gap 2 h, then hourly": [full[0]] + full[2:],
    }
    out = {}
    for label, ser in series.items():
        msgs.clear()
        with tempfile.TemporaryDirectory() as wd:
            rec = run(surf, loc, ser, wd)
        steps = [(t.strftime("%H:%M"), round(r["alt"], 1), r["firstdaytime"], round(r["timeadd"] * 1440, 2),
                  round(r["timestep_dec"] * 1440, 2)) for t, r in sorted(rec.items())]
        warn = [m for m in msgs if "evenly" in m or "apart" in m]
        print(f"\n=== gaps: {label}: {len(ser)} records, warnings: {warn}")
        print("(end-stamp, sun alt, firstdaytime after, timeadd after [min], timestep_dec [min])")
        for st in steps:
            print("  ", st)
        out[label] = {"warnings": warn, "steps": steps}
    return out


def main():
    print("solweig", solweig.__version__, "python", sys.version.split()[0])
    surf = surface()
    bastia = Location(latitude=43.07, longitude=12.62, utc_offset=1)
    svalbard = Location(latitude=78.22, longitude=15.65, utc_offset=1)
    day0 = datetime(2024, 7, 14)
    res = [
        case(surf, bastia, day0, [1.0, 1.0, 1.0], "Bastia, three clear days"),
        case(surf, bastia, day0, [0.35, 1.0, 1.0], "Bastia, day 1 overcast (0.35 x clear), days 2-3 clear"),
        case(surf, svalbard, datetime(2024, 6, 20), [1.0, 1.0, 1.0], "Svalbard 78N, midnight sun, three clear days"),
    ]
    gaps = case_gaps(surf, bastia, day0)
    if len(sys.argv) > 1:
        with open(sys.argv[1], "w") as fh:
            json.dump({"cases": res, "gaps": gaps}, fh, indent=1)


if __name__ == "__main__":
    main()
