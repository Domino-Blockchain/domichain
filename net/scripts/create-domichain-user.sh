#!/usr/bin/env bash
set -ex

[[ $(uname) = Linux ]] || exit 1
[[ $USER = root ]] || exit 1

if grep -q domichain /etc/passwd ; then
  echo "User domichain already exists"
else
  adduser domichain --gecos "" --disabled-password --quiet
  adduser domichain sudo
  adduser domichain adm
  echo "domichain ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers
  id domichain

  [[ -r /domichain-scratch/id_ecdsa ]] || exit 1
  [[ -r /domichain-scratch/id_ecdsa.pub ]] || exit 1

  sudo -u domichain bash -c "
    echo 'PATH=\"/home/domichain/.cargo/bin:$PATH\"' > /home/domichain/.profile
    mkdir -p /home/domichain/.ssh/
    cd /home/domichain/.ssh/
    cp /domichain-scratch/id_ecdsa.pub authorized_keys
    umask 377
    cp /domichain-scratch/id_ecdsa id_ecdsa
    echo \"
      Host *
      BatchMode yes
      IdentityFile ~/.ssh/id_ecdsa
      StrictHostKeyChecking no
    \" > config
  "
fi
