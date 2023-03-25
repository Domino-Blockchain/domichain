#!/bin/bash
# Start new validator

# Exit on any error
set -o errexit
# Print executed commands
set -o verbose

#if [ -z "$1" ]
#  then
#    echo "No argument supplied: you must supply private IP address or bootstrap node"
#    exit 1
#fi
#export NODE_IP_ADDR="$1" # Get form main node: hostname -I | cut -d' ' -f1
#export URL="http://$NODE_IP_ADDR:8899/"

cd ~/domichain
rm -rf ~/domichain/config

screen -d -m -S sys-tuner bash -c 'sudo $(command -v domichain-sys-tuner) --user $(whoami)'

slots_per_epoch=
if [ -n "$1" ]
  then
    slots_per_epoch=" --slots-per-epoch $1"
fi

airdrop_amount=600
if [ -n "$2" ]
  then
    airdrop_amount=$2
fi

WAIT_TIMEOUT=200
if [ -n "$3" ]
  then
    WAIT_TIMEOUT=$3
fi
export WAIT_TIMEOUT

./multinode-demo/setup.sh $slots_per_epoch
FAUCET_PUBKEY=$(domichain-keygen pubkey ~/domichain/config/faucet.json)
export FAUCET_PUBKEY
domichain airdrop $airdrop_amount --url "$URL" "$FAUCET_PUBKEY"

export RUST_LOG="INFO,domichain_metrics::metrics=WARN"
screen -d -m -S validator bash -c "./multinode-demo/validator-x.sh \
  --label test1 \
  --entrypoint $NODE_IP_ADDR:8001 \
  --rpc-faucet-address $NODE_IP_ADDR:9900 \
  --allow-private-addr \
  > ~/stdout.txt 2> ~/stderr.txt"

screen -d -m -S watch bash -c "watch \
  \"domichain gossip --url $URL && \
    domichain validators --url $URL && \
    domichain stake-history --url $URL\" "

# WARNING: it will override keypair file
domichain-keygen new --force --no-passphrase -o ~/validator-stake-keypair.json
PUBKEY=$(domichain-keygen pubkey ~/validator-stake-keypair.json)
export PUBKEY

domichain airdrop 600 --url "$URL" "$PUBKEY"

function wait_for() {
    timeout=WAIT_TIMEOUT
    shift 1
    until [ $timeout -le 0 ] || ("$@" &> /dev/null); do
        echo waiting for "$@"
        sleep 1
        timeout=$(( timeout - 1 ))
    done
    if [ $timeout -le 0 ]; then
        return 1
    fi
}

function is_root_equal() {
  highest_root=$(domichain validators --url $URL --output json | jq '.validators | map(.rootSlot) | max')

  vote_pubkey=$(domichain-keygen pubkey ~/domichain/config/validator-test1/vote-account.json)
  our_root=$(domichain validators --url $URL --output json | jq ".validators[] | select(.voteAccountPubkey == \"${vote_pubkey}\") | .rootSlot")
  if [ "${highest_root}" = "${our_root}" ]; then
    return 0;
  else
    return 1;
  fi
}

echo "Waiting for sync slots with bootstrap validator"
wait_for 180 is_root_equal

./multinode-demo/delegate-stake.sh \
  --url "http://$NODE_IP_ADDR:8899/" \
  --label test1 \
  --keypair ~/validator-stake-keypair.json
