#!/bin/bash
# Start new validator

# Exit on any error
set -o errexit
# Print executed commands
set -o verbose

if [ -z "$1" ]
  then
    echo "No argument supplied: you must supply private IP address or bootstrap node"
    exit 1
fi
export NODE_IP_ADDR="$1"
export URL="http://$NODE_IP_ADDR:8899/"

cd ~/domichain
export CUDA_HOME=/usr/local/cuda-11.1/
export LD_LIBRARY_PATH=/home/ubuntu/domichain/target/perf-libs${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}
export LD_LIBRARY_PATH=/usr/local/cuda-11.1/lib64${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}

#export RUST_LOG=ERROR
export NDEBUG=1
export DOMICHAIN_CUDA=1

screen -d -m -S validator bash -c './multinode-demo/validator-x.sh --label test1 --entrypoint "$NODE_IP_ADDR:8001" --rpc-faucet-address "$NODE_IP_ADDR:9900" --allow-private-addr > ~/stdout.txt 2> ~/stderr.txt'
# ./multinode-demo/validator-x.sh \
#     --label test1 \
#     --entrypoint "$NODE_IP_ADDR:8001" \
#     --rpc-faucet-address "$NODE_IP_ADDR:9900" \
#     --allow-private-addr

screen -d -m -S watch bash -c 'watch "target/release/domichain gossip --url $URL && target/release/domichain validators --url $URL && target/release/domichain stake-history --url $URL"'
# watch "target/release/domichain gossip --url $URL && target/release/domichain validators --url $URL && target/release/domichain stake-history --url $URL"

# WARNING: it will override keypair file
target/release/domichain-keygen new --force --no-passphrase -o ~/validator-stake-keypair.json
PUBKEY=$(target/release/domichain-keygen pubkey ~/validator-stake-keypair.json)
export PUBKEY

target/release/domichain airdrop 600 --url "$URL" "$PUBKEY"

export RUST_LOG=INFO
echo 'Wait for sync slots with bootstrap validator and run:'
echo './multinode-demo/delegate-stake.sh --url "http://$NODE_IP_ADDR:8899/" --label test --keypair ~/validator-stake-keypair.json'
