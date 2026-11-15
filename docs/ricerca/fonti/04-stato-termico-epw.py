"""How often a real EPW leaves solweig a cloudy clearness index for the night.

Research script for docs/ricerca/04-stato-termico.md (OMBRA #21). Not part of
OMBRA. Needs the solweig 0.1.0b101 wheel and an EPW path.

1. Builds the whole year as solweig Weather objects, runs solweig's own
   _precompute_weather (the step that carries the last daytime clearness index
   into the night, timeseries.py:131-143 at 41840b7), and counts the days whose
   first hour inherits a CI below 0.95 (the Ldown cloud-correction threshold).
2. For a few start days, Run A = day before + two days, Run B = the two days,
   and reports the largest Tmrt difference per hour of the common span.

Usage: python -I 04-stato-termico-epw.py <file.epw>
"""

import sys
import tempfile
from datetime import datetime, timedelta

import numpy as np
import solweig
import solweig.api as sapi
from solweig import Location, SurfaceData, Weather
from solweig.timeseries import _precompute_weather

YEAR = 2023  # TMY months come from different years; one non-leap year keeps them in order


def read_epw(path):
    out = []
    with open(path, encoding="latin-1") as fh:
        for i, line in enumerate(fh):
            if i < 8:
                continue
            f = line.strip().split(",")
            month, day, hour = int(f[1]), int(f[2]), int(f[3])
            t = datetime(YEAR, month, day) + timedelta(hours=hour)  # end of the hour
            out.append(dict(datetime=t, ta=float(f[6]), rh=float(f[8]), pressure=float(f[9]) / 100.0,
                            global_rad=float(f[13]), ws=float(f[21])))
    out.sort(key=lambda r: r["datetime"])
    return out


def weather(rows):
    return [Weather(**r) for r in rows]


def run(surf, loc, series, workdir):
    rec = {}
    orig = sapi._calculate_single

    def spy(*args, **kwargs):
        r = orig(*args, **kwargs)
        w = kwargs["weather"]
        rec[w.datetime] = (np.array(r.tmrt, copy=True), float(w.clearness_index), float(w.sun_altitude))
        return r

    sapi._calculate_single = spy
    try:
        solweig.calculate(surf, series, loc, output_dir=workdir)
    finally:
        sapi._calculate_single = orig
    return rec


def main():
    path = sys.argv[1]
    loc = Location.from_epw(path)
    rows = read_epw(path)
    print("solweig", solweig.__version__, "|", loc, "|", len(rows), "records")

    # 1. CI inherited by hour 1 of each day
    ws = weather(rows)
    _precompute_weather(ws, loc)
    first = [w for w in ws if w.datetime.hour == 1]
    night_first = [w for w in first if w.sun_altitude <= 0]
    low = [w for w in night_first if w.clearness_index < 0.95]
    print(f"\n1. days: {len(first)}; hour 1 at night: {len(night_first)}; "
          f"inheriting CI < 0.95: {len(low)} ({100 * len(low) / max(1, len(night_first)):.0f}%)")
    cis = np.array([w.clearness_index for w in night_first])
    print("   inherited CI quantiles 10/25/50/75/90%:", np.round(np.quantile(cis, [0.1, 0.25, 0.5, 0.75, 0.9]), 3))
    by_month = {}
    for w in night_first:
        by_month.setdefault(w.datetime.month, []).append(w.clearness_index < 0.95)
    print("   share per month:", {m: f"{100 * np.mean(v):.0f}%" for m, v in sorted(by_month.items())})

    # 2. A vs B on real days
    dsm = np.zeros((40, 40), dtype=np.float32)
    dsm[14:24, 14:24] = 12.0
    surf = SurfaceData.prepare(dsm=dsm, pixel_size=1.0)
    # four fixed mid-season days, then the inheriting days with the lowest CI (different months)
    fixed = [datetime(YEAR, m, 15) for m in (1, 4, 7, 10)]
    picked, months = [], set()
    for w in sorted(low, key=lambda w: w.clearness_index):
        if w.datetime.month not in months and w.datetime.day > 1:
            picked.append(w.datetime.replace(hour=0))
            months.add(w.datetime.month)
        if len(picked) == 4:
            break
    for start in fixed + sorted(picked):
        a_rows = [r for r in rows if start - timedelta(days=1) < r["datetime"] <= start + timedelta(days=2)]
        b_rows = [r for r in a_rows if r["datetime"] > start]
        with tempfile.TemporaryDirectory() as wa, tempfile.TemporaryDirectory() as wb:
            a = run(surf, loc, weather(a_rows), wa)
            b = run(surf, loc, weather(b_rows), wb)
        diffs = []
        for t in sorted(b):
            d = float(np.nanmax(np.abs(a[t][0].astype(np.float64) - b[t][0])))
            if d > 0:
                diffs.append((t.strftime("%m-%d %H:%M"), round(a[t][2], 1), round(a[t][1], 3), round(b[t][1], 3), round(d, 3)))
        print(f"\n2. B starts {start:%m-%d} 01:00: {len(b)} common hours, {len(diffs)} with Tmrt difference")
        print("   (end-stamp, sun alt, CI in A, CI in B, max |Tmrt A - Tmrt B| K)")
        for x in diffs:
            print("  ", x)


if __name__ == "__main__":
    main()
