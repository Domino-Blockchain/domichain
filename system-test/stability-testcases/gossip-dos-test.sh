#!/usr/bin/env bash

set -e
cd "$(dirname "$0")"
SOLANA_ROOT="$(cd ../..; pwd)"

logDir="$PWD"/logs
rm -rf "$logDir"
mkdir "$logDir"

domichainInstallDataDir=$PWD/releases
domichainInstallGlobalOpts=(
  --data-dir "$domichainInstallDataDir"
  --config "$domichainInstallDataDir"/config.yml
  --no-modify-path
)

# Install all the domichain versions
bootstrapInstall() {
  declare v=$1
  if [[ ! -h $domichainInstallDataDir/active_release ]]; then
    sh "$SOLANA_ROOT"/install/domichain-install-init.sh "$v" "${domichainInstallGlobalOpts[@]}"
  fi
  export PATH="$domichainInstallDataDir/active_release/bin/:$PATH"
}

bootstrapInstall "edge"
domichain-install-init --version
domichain-install-init edge
domichain-gossip --version
domichain-dos --version

killall domichain-gossip || true
domichain-gossip spy --gossip-port 8001 > "$logDir"/gossip.log 2>&1 &
domichainGossipPid=$!
echo "domichain-gossip pid: $domichainGossipPid"
sleep 5
domichain-dos --mode gossip --data-type random --data-size 1232 &
dosPid=$!
echo "domichain-dos pid: $dosPid"

pass=true

SECONDS=
while ((SECONDS < 600)); do
  if ! kill -0 $domichainGossipPid; then
    echo "domichain-gossip is no longer running after $SECONDS seconds"
    pass=false
    break
  fi
  if ! kill -0 $dosPid; then
    echo "domichain-dos is no longer running after $SECONDS seconds"
    pass=false
    break
  fi
  sleep 1
done

kill $domichainGossipPid || true
kill $dosPid || true
wait || true

$pass && echo Pass
