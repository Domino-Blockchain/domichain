#!/bin/bash

set -o errexit

cd ~/domichain

# screen -d -m -S faucet bash -c './multinode-demo/faucet.sh'

export RUST_LOG="INFO,domichain_metrics::metrics=WARN"
./multinode-demo/bootstrap-validator.sh \
  --gossip-host $NODE_IP_ADDR \
  --enable-rpc-transaction-history \
  --allow-private-addr \
  --limit-ledger-size 50000000 \
   > ~/stdout.txt 2> ~/stderr.txt
