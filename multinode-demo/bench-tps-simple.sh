#!/usr/bin/env bash
set -e

here=$(dirname "$0")
# shellcheck source=multinode-demo/common.sh
source "$here"/common.sh

usage() {
  if [[ -n $1 ]]; then
    echo "$*"
    echo
  fi
  echo "usage: $0 [extra args]"
  echo
  echo " Run bench-tps-simple "
  echo
  echo "   extra args: additional arguments are passed along to domichain-bench-tps-simple"
  echo
  exit 1
}

if [[ -z $1 ]]; then # default behavior
  $domichain_bench_tps_simple \
    --entrypoint 127.0.0.1:8001 \
    --faucet 127.0.0.1:9900 \
    --duration 30 \
    --tx_count 1000 \
    --num-lamports-per-account 2000000000 \

else
  $domichain_bench_tps_simple "$@"
fi
