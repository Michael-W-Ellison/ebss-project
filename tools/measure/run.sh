#!/bin/bash
# run.sh BIN OUT [SMOOTH_SEEDS] [SCATTERED_SEEDS]: forty settlements, two
# years each, three at a time, one file per settlement in OUT. Settlements
# already finished in OUT are skipped, so a run cut short can be resumed.
# Seeds default to smooth 1-24 and scattered 1-16.
BIN=$1; OUT=$2
SM=${3:-"$(seq -s ' ' 1 24)"}; SC=${4:-"$(seq -s ' ' 1 16)"}
REPO=$(cd "$(dirname "$0")/../.." && pwd)
export BIN OUT REPO
mkdir -p "$OUT"
one() { t=$1; s=$2; if [ "$t" = sm ]; then E="SMOOTH=1"; else E="SCATTERED=1"; fi
  grep -q '^BD D0720' "$OUT/${t}_s$s.txt" 2>/dev/null && return
  cd "$REPO"
  env $E SEEDS=$s DAYS=720 "$BIN" --ignored --nocapture zz_discovery_watch 2>&1 \
    | grep -a "^START\|^DISC\|^BD\|^DEATH\|^MOVECAMP\|^FISHX\|^KEEPX\|^SHELX\|^CAREX\|^GAPX\|^LEATHX\|^HERDX" > "$OUT/${t}_s$s.txt"; }
export -f one
(for s in $SM; do echo sm $s; done; for s in $SC; do echo sc $s; done) | xargs -P 3 -L 1 bash -c 'one $0 $1'
echo ALLDONE > "$OUT/done"
