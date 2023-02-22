#!/bin/bash
# Start bootstrap validator

cd ~/domichain
export CUDA_HOME=/usr/local/cuda-11.1/
export LD_LIBRARY_PATH=/home/ubuntu/domichain/target/perf-libs${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}
export LD_LIBRARY_PATH=/usr/local/cuda-11.1/lib64${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}
export NODE_IP_ADDR=$(hostname --ip-address)

screen -d -m -S sys-tuner bash -c 'sudo $(command -v target/release/domichain-sys-tuner) --user $(whoami)'
# sudo $(command -v target/release/domichain-sys-tuner) --user $(whoami)

export RUST_LOG=ERROR
export NDEBUG=1
export DOMICHAIN_CUDA=1

screen -d -m -S faucet bash -c './multinode-demo/setup.sh && ./multinode-demo/faucet.sh'
# ./multinode-demo/setup.sh && ./multinode-demo/faucet.sh

screen -d -m -S bootstrap-validator bash -c './multinode-demo/bootstrap-validator.sh --gossip-host $NODE_IP_ADDR --enable-rpc-transaction-history'
# ./multinode-demo/bootstrap-validator.sh --gossip-host $NODE_IP_ADDR --enable-rpc-transaction-history