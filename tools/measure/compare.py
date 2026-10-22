"""Compare two settlement measurements, settlement by settlement.

    python3 tools/measure/compare.py BEFORE_DIR AFTER_DIR

Pairs the settlements both runs finished, prints the totals and a paired t
for births, deaths, hunger deaths, survivors and camp moves - smooth maps,
scattered maps and all - and then the counter lines (KEEPX, SHELX, CAREX,
GAPX, FISHX) and death causes of the second run.
"""
import collections
import glob
import math
import os
import re
import sys


def read(d):
    out = {}
    for f in glob.glob(os.path.join(d, "*_s*.txt")):
        r = dict(births=0, deaths=0, alive=0, moves=0, hungerdeaths=0, done=False,
                 counted=collections.Counter(), causes=collections.Counter())
        for line in open(f, errors="replace"):
            if line.startswith("BD D0720"):
                m = re.search(r"births (\d+) deaths (\d+) alive (\d+)", line)
                r["births"], r["deaths"], r["alive"] = map(int, m.groups())
                r["done"] = True
            elif line.startswith("MOVECAMP"):
                r["moves"] += 1
            elif line.startswith("DEATH"):
                cause = line.split("cause=", 1)[1].strip()
                r["causes"][cause] += 1
                if cause == "hunger":
                    r["hungerdeaths"] += 1
            elif line[:5] in ("KEEPX", "SHELX", "CAREX", "GAPX ", "FISHX"):
                r["counted"][line.strip()] += 1
        out[os.path.basename(f)[:-4]] = r
    return out


a, b = read(sys.argv[1]), read(sys.argv[2])
keys = sorted(k for k in a if k in b and a[k]["done"] and b[k]["done"])
print(len(keys), "paired")
for terrain in ("sm", "sc", ""):
    ks = [k for k in keys if k.startswith(terrain)]
    if len(ks) < 2:
        continue
    for field in ("births", "deaths", "hungerdeaths", "alive", "moves"):
        before = sum(a[k][field] for k in ks)
        after = sum(b[k][field] for k in ks)
        d = [b[k][field] - a[k][field] for k in ks]
        n = len(d)
        mean = sum(d) / n
        sd = math.sqrt(sum((x - mean) ** 2 for x in d) / (n - 1)) if n > 1 else 0
        t = mean / (sd / math.sqrt(n)) if sd else 0
        print(f"{terrain or 'all':3} {field:13} {before:6} -> {after:6}  mean {mean:+.2f} t {t:+.2f}")

counted = collections.Counter()
worlds = collections.Counter()
causes = collections.Counter()
for k in keys:
    counted.update(b[k]["counted"])
    causes.update(b[k]["causes"])
    for line in b[k]["counted"]:
        worlds[line] += 1
print("deaths by cause", dict(causes))
for line, n in counted.most_common():
    print(f"  {n:7} {line}  ({worlds[line]} worlds)")
