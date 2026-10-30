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

# arm64 has no open(2), so drop its tracepoints (bpftrace refuses to attach a
# probe list containing a missing tracepoint) and pass the arch's syscall
# numbers for open, openat, openat2 to the raw_syscalls filter.
script="$dir/pairing.bt"
case $(uname -m) in
x86_64) ids="2 257 437" ;;
aarch64)
	ids="56 56 437"
	script=$(mktemp)
	grep -vE 'sys_(enter|exit)_open,$' "$dir/pairing.bt" >"$script"
	;;
*) echo "unsupported arch $(uname -m)" >&2; exit 1 ;;
esac

# shellcheck disable=SC2086
sudo bpftrace "$script" $ids >"$log" 2>&1 &
bt=$!
for _ in $(seq 100); do
	grep -q Attaching "$log" && break
	sleep 0.1
done
sleep 1

bash -c "$BUILD" >/dev/null 2>&1
echo "build exit: $?"

# Signal bpftrace itself, not the sudo wrapper in $bt: on Ubuntu 22.04 the
# SIGINT sent to sudo never reached bpftrace and `wait` hung for hours.
sudo pkill -INT -f "bpftrace $script" || true
for _ in $(seq 300); do
	kill -0 "$bt" 2>/dev/null || break
	sleep 0.1
done
if kill -0 "$bt" 2>/dev/null; then
	echo "::warning::bpftrace ignored SIGINT for 30s, killing it (maps not printed)"
	sudo pkill -KILL -f "bpftrace $script" || true
fi
wait "$bt"
cat "$log"

if grep -q '^MISSING' "$log"; then
	echo "::error::$mode cache: open* sys_exit without sys_enter reproduced without witness"
	exit 1
fi
