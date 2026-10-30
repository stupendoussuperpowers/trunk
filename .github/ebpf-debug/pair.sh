#!/usr/bin/env bash
# Runs $BUILD under pairing.bt and fails if any open* exit had no enter.
# Usage: pair.sh cold|warm
set -uo pipefail

mode=$1
dir=$(cd "$(dirname "$0")" && pwd)
log="$OUT/pairing-$mode.txt"

eval "$CLEAN" >/dev/null 2>&1 || true
if [ "$mode" = cold ]; then
	sync
	echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null
fi

sudo bpftrace "$dir/pairing.bt" >"$log" 2>&1 &
bt=$!
for _ in $(seq 100); do
	grep -q Attaching "$log" && break
	sleep 0.1
done
sleep 1

bash -c "$BUILD" >/dev/null 2>&1
echo "build exit: $?"

sudo kill -INT "$bt"
wait "$bt"
cat "$log"

if grep -q '^MISSING' "$log"; then
	echo "::error::$mode cache: open* sys_exit without sys_enter reproduced without witness"
	exit 1
fi
