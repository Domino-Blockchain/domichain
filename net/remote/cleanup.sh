#!/usr/bin/env bash

set -x
! tmux list-sessions || tmux kill-session
declare sudo=
if sudo true; then
  sudo="sudo -n"
fi

echo "pwd: $(pwd)"
for pid in domichain/*.pid; do
  pgid=$(ps opgid= "$(cat "$pid")" | tr -d '[:space:]')
  if [[ -n $pgid ]]; then
    $sudo kill -- -"$pgid"
  fi
done
if [[ -f domichain/netem.cfg ]]; then
  domichain/scripts/netem.sh delete < domichain/netem.cfg
  rm -f domichain/netem.cfg
fi
domichain/scripts/net-shaper.sh cleanup
for pattern in validator.sh boostrap-leader.sh domichain- remote- iftop validator client node; do
  echo "killing $pattern"
  pkill -f $pattern
done
