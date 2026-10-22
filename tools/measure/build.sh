#!/bin/bash
# build.sh REV NAME: a release test binary of git revision REV with the
# measurement counters, at $WORK/NAME. $WORK defaults to /tmp/measure.
# The measurement harness is taken from the working tree for revisions that
# predate it being committed (#352).
set -e
REV=$1; NAME=$2
WORK=${WORK:-/tmp/measure}
REPO=$(cd "$(dirname "$0")/../.." && pwd)
TREE=$WORK/tree_$NAME
mkdir -p "$WORK"
rm -rf "$TREE"; mkdir -p "$TREE"
git -C "$REPO" archive "$REV" | tar xf - -C "$TREE"
if [ ! -f "$TREE/src/analytics/tests/measurement_watch.rs" ]; then
  cp "$REPO/src/analytics/tests/measurement_watch.rs" "$TREE/src/analytics/tests/"
  echo "pub mod measurement_watch;" >> "$TREE/src/analytics/tests/mod.rs"
fi
python3 "$REPO/tools/measure/probe.py" "$TREE"
(cd "$TREE" && CARGO_TARGET_DIR=$WORK/target_$NAME nice cargo test --release --lib --no-run -j 3 > "$WORK/build_$NAME.txt" 2>&1)
B=$(ls -t $WORK/target_$NAME/release/deps/ebss-* | grep -v '\.d$' | head -1)
cp "$B" "$WORK/$NAME"
echo "built $WORK/$NAME"
