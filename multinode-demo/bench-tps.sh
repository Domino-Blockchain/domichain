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
  echo " Run bench-tps "
  echo
  echo "   extra args: additional arguments are passed along to domichain-bench-tps"
  echo
  exit 1
}

args=("$@")
default_arg --entrypoint "127.0.0.1:8001"
default_arg --faucet "127.0.0.1:9900"
default_arg --duration 20
default_arg --tx-count 500
default_arg --thread-batch-sleep-ms 0
default_arg --keypair-multiplier 2

$domichain_bench_tps "${args[@]}"
