#!/usr/bin/env bash
set -ex

[[ $(uname) = Linux ]] || exit 1
[[ $USER = root ]] || exit 1

[[ -d /home/domichain/.ssh ]] || exit 1

if [[ ${#SOLANA_PUBKEYS[@]} -eq 0 ]]; then
  echo "Warning: source domichain-user-authorized_keys.sh first"
fi

# domichain-user-authorized_keys.sh defines the public keys for users that should
# automatically be granted access to ALL testnets
for key in "${SOLANA_PUBKEYS[@]}"; do
  echo "$key" >> /domichain-scratch/authorized_keys
done

sudo -u domichain bash -c "
  cat /domichain-scratch/authorized_keys >> /home/domichain/.ssh/authorized_keys
"
