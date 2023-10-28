#!/usr/bin/env bash

here=$(dirname "$0")
# shellcheck source=multinode-demo/common.sh
source "$here"/common.sh

set -e

rm -rf "$DOMICHAIN_CONFIG_DIR"/latest-testnet-snapshot
mkdir -p "$DOMICHAIN_CONFIG_DIR"/latest-testnet-snapshot
(
  cd "$DOMICHAIN_CONFIG_DIR"/latest-testnet-snapshot || exit 1
  set -x
  wget http://api.testnet.domichain.com/genesis.tar.bz2
  wget --trust-server-names http://testnet.domichain.com/snapshot.tar.bz2
)

snapshot=$(ls "$DOMICHAIN_CONFIG_DIR"/latest-testnet-snapshot/snapshot-[0-9]*-*.tar.zst)
if [[ -z $snapshot ]]; then
  echo Error: Unable to find latest snapshot
  exit 1
fi

if [[ ! $snapshot =~ snapshot-([0-9]*)-.*.tar.zst ]]; then
  echo Error: Unable to determine snapshot slot for "$snapshot"
  exit 1
fi

snapshot_slot="${BASH_REMATCH[1]}"

rm -rf "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator
mkdir -p "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator


# Create genesis ledger
if [[ -r $FAUCET_KEYPAIR ]]; then
  cp -f "$FAUCET_KEYPAIR" "$DOMICHAIN_CONFIG_DIR"/faucet.json
else
  $domichain_keygen new --no-passphrase -fso "$DOMICHAIN_CONFIG_DIR"/faucet.json
fi

if [[ -f $BOOTSTRAP_VALIDATOR_IDENTITY_KEYPAIR ]]; then
  cp -f "$BOOTSTRAP_VALIDATOR_IDENTITY_KEYPAIR" "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/identity.json
else
  $domichain_keygen new --no-passphrase -so "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/identity.json
fi

$domichain_keygen new --no-passphrase -so "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/vote-account.json
$domichain_keygen new --no-passphrase -so "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/stake-account.json

$domichain_ledger_tool create-snapshot \
  --ledger "$DOMICHAIN_CONFIG_DIR"/latest-testnet-snapshot \
  --faucet-pubkey "$DOMICHAIN_CONFIG_DIR"/faucet.json \
  --faucet-satomis 500000000000000000 \
  --bootstrap-validator "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/identity.json \
                        "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/vote-account.json \
                        "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator/stake-account.json \
  --hashes-per-tick sleep \
  "$snapshot_slot" "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator

$domichain_ledger_tool modify-genesis \
  --ledger "$DOMICHAIN_CONFIG_DIR"/latest-testnet-snapshot \
  --hashes-per-tick sleep \
  "$DOMICHAIN_CONFIG_DIR"/bootstrap-validator
