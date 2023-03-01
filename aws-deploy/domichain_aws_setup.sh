#!/bin/bash
# Setup new node and reboot for apply settings

# Use instance template with AMI ID: ami-05c1e79d94b4b0546
# See: https://aws.amazon.com/releasenotes/aws-deep-learning-gpu-ami-ubuntu-20-04-version-1-x-x/


# Create a new SSH identity file. For example:
# ```
# ssh-keygen -t rsa -N "" -f ~/.ssh/id_rsa
# cat .ssh/id_rsa.pub
# ```
# Put it into: https://github.com/settings/keys and share between instances

# Exit on any error
set -o errexit
# Print executed commands
set -o verbose

if [ -z "$1" ]
  then
    echo "No argument supplied: you must supply identity file for GitHub"
    exit 1
fi

IDENTITY_FILE=$(realpath $1)
export IDENTITY_FILE
chmod 400 "$IDENTITY_FILE"

sudo apt update && sudo apt upgrade -y && sudo apt install -y libsodium-dev libudev-dev libclang-dev htop
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

export GIT_SSH_COMMAND="ssh -i $IDENTITY_FILE"
ssh-keyscan github.com >> ~/.ssh/known_hosts
git clone git@github.com:Domino-Blockchain/domichain.git ~/domichain
cd ~/domichain

if [ -n "$2" ]
  then
    git switch "$2" # Select most recent branch
fi
git pull
cargo build --release

git clone git@github.com:solana-labs/solana-perf-libs.git ~/solana-perf-libs
cd ~/solana-perf-libs
export PATH=/usr/local/cuda/bin:$PATH
make -j$(nproc)
export SOLANA_ROOT=/home/ubuntu/domichain
make DESTDIR=${SOLANA_ROOT:?}/target/perf-libs install

echo "ubuntu soft nofile 500000" | sudo tee -a /etc/security/limits.conf
echo "ubuntu hard nofile 500000" | sudo tee -a /etc/security/limits.conf
echo "fs.file-max = 500000" | sudo tee -a /etc/sysctl.conf
# Apply settings
sudo reboot
