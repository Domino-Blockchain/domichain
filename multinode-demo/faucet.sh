#!/usr/bin/env bash
#
# Starts an instance of domichain-faucet
#
here=$(dirname "$0")

# shellcheck source=multinode-demo/common.sh
source "$here"/common.sh

[[ -f "$DOMICHAIN_CONFIG_DIR"/faucet.json ]] || {
  echo "$DOMICHAIN_CONFIG_DIR/faucet.json not found, create it by running:"
  echo
  echo "  ${here}/setup.sh"
  exit 1
}

set -x
# shellcheck disable=SC2086 # Don't want to double quote $domichain_faucet
exec $domichain_faucet --keypair "$DOMICHAIN_CONFIG_DIR"/faucet.json "$@"
