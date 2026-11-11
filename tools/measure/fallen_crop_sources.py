"""Sort field crop fallen beyond two walks by where it came from (#426).

    python3 tools/measure/fallen_crop_sources.py RUN_DIR
"""
import sys, glob, re, collections
D = sys.argv[1]
WALK2 = 24
tot = collections.Counter(); cls = collections.Counter(); sub = collections.Counter()
worlds = collections.defaultdict(collections.Counter)
ages = collections.Counter()
for f in sorted(glob.glob(D + "/s[mc]_s*.txt")):
    w = f.split("/")[-1]
    for line in open(f, errors="replace"):
        if not line.startswith("FART"): continue
        t = line.split()
        day = int(t[1][1:]); band = int(t[3]); amt = int(t[5])
        workers = [int(x) for x in t[7].strip("[]").split(",") if x]
        tot[band] += amt
        if band != 2: continue
        kv = dict(zip(t[8::2], t[9::2]))
        if kv.get("how") == "none":
            c = "no origin recorded"
        else:
            bthen = int(kv["breaker_then"]); bnow = int(kv["breaker_now"])
            snow = int(kv["sower_now"]); sthen = int(kv["sower_then"])
            sown = int(kv["sown_day"]); how = int(kv["how"])
            def state(now, then):
                if now in (-2, -3): return "dead"
                if now == -1: return "no hearth now"
                if now > WALK2: return "hearth moved away"
                return "hearth still within two walks"
            if then_leak := (sthen == -1 or sthen > WALK2):
                pass
            if workers:
                alive = [x for x in workers if x >= -1]
                if not alive: c = "sown/worked this season; all workers dead"
                elif all(x == -1 for x in alive): c = "sown/worked this season; workers have no hearth"
                else: c = "sown/worked this season; workers' hearths moved away"
                # how far from the sower's hearth when sown
                sub["worked: sower hearth at sowing " + ("none" if sthen == -1 else "<=24" if sthen <= WALK2 else ">24")] += amt
            else:
                c = "grew back unsown; last sower " + state(snow, sthen)
                sub["unsown: sower hearth at sowing " + ("none" if sthen == -1 else "<=24" if sthen <= WALK2 else ">24")] += amt
                sub["unsown: breaker " + state(bnow, bthen)] += amt
                seasons = (day - sown) // 360
                ages["%d years since last sown" % seasons] += amt
            sub["how " + {"0": "new ground", "1": "ploughed in", "2": "already a field"}[str(how)]] += amt
        cls[c] += amt
        worlds[w][c] += amt
all_ = sum(tot.values())
print("fallen by band:", {b: tot[b] for b in sorted(tot)}, "all", all_)
far = tot[2]
print(f"\nfallen beyond two walks: {far} ({100*far/max(all_,1):.0f}% of all)")
for c, v in cls.most_common(): print(f"  {v:>9} {100*v/far:5.1f}%  {c}")
print()
for c, v in sorted(sub.items()): print(f"  {v:>9} {100*v/far:5.1f}%  {c}")
print()
for c, v in sorted(ages.items()): print(f"  {v:>9} {100*v/far:5.1f}%  unsown, {c}")
