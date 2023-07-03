#!/bin/bash
# Start bootstrap validator

# Exit on any error
set -o errexit
# Print executed commands
set -o verbose

export NODE_IP_ADDR=127.0.0.1
export URL=http://127.0.0.1:8899

cd ~/domichain
rm -rf ~/domichain/config

screen -d -m -S sys-tuner bash -c 'sudo $(command -v domichain-sys-tuner) --user $(whoami)'

slots_per_epoch=
if [ -n "$1" ]
  then
    slots_per_epoch=" --slots-per-epoch $1"
fi
./multinode-demo/setup.sh $slots_per_epoch
screen -d -m -S faucet bash -c './multinode-demo/faucet.sh'

export RUST_LOG="INFO,domichain_metrics::metrics=WARN"
screen -d -m -S bootstrap-validator bash -c "DOMICHAIN_CUDA=1 ./multinode-demo/bootstrap-validator.sh \
  --gossip-host $NODE_IP_ADDR \
  --enable-rpc-transaction-history \
  --allow-private-addr \
   > ~/stdout.txt 2> ~/stderr.txt"

screen -d -m -S watch bash -c "watch \
  \"domichain gossip --url $URL && \
    domichain validators --url $URL && \
    domichain stake-history --url $URL\" "
