#RESTART=1 # Update the below block before uncommenting this line
#if [[ -n "$RESTART" ]]; then
#        WAIT_FOR_SUPERMAJORITY=53180900
#        EXPECTED_BANK_HASH=Fi4p8z3AkfsuGXZzQ4TD28N8QDNSWC7ccqAqTs2GPdPu
#fi
EXPECTED_SHRED_VERSION=4175
EXPECTED_GENESIS_HASH=Bk35uYWXgt6rN1rYzY7L7AqrrNwPhbg3MVde46YhMUCS
TRUSTED_VALIDATOR_PUBKEYS=(7hoUyAJm3dG7XpFt4yB1f7AHZvaQygyHcb8hD66vP2Ai 9MyuD1XotiuYWtDHFJnXkPFKxdSL47uMJxYFjBDpFbQo)
# export DOMI_METRICS_CONFIG=host=https://metrics.domichain.io:8086,db=mainnet-beta,u=mainnet-beta_write,p=password
#Replace the below with a full path that includes both Domichain's binary and generic system binaries
#Do not enter PATH=$PATH if you're planning to run the script as systemctl
PATH=/home/domi/bin:$PATH
#MINIMUM_MINUTES_BETWEEN_ARCHIVE=720
RPC_URL=https://api.testnet.domichain.io
ENTRYPOINT_HOST=103.106.59.69
ENTRYPOINT_PORT=8001
ENTRYPOINT=103.106.59.69:8001
# ENTRYPOINTS=(
# )
export RUST_BACKTRACE=1
export LimitNOFILE=1000000
export GOOGLE_APPLICATION_CREDENTIALS=<path_to_your_google_cloud_credentials>
ENABLE_BPF_JIT=0
ENABLE_CPI_AND_LOG_STORAGE=1
