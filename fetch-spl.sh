#!/usr/bin/env bash
#
# Fetches the latest SPL programs and produces the domichain-genesis command-line
# arguments needed to install them
#

set -e

fetch_program() {
  declare name=$1
  declare version=$2
  declare address=$3
  declare loader=$4

  declare so=spl_$name-$version.so

  genesis_args+=(--bpf-program "$address" "$loader" "$so")

  if [[ -r $so ]]; then
    return
  fi

  if [[ -r ~/.cache/domichain-spl/$so ]]; then
    cp ~/.cache/domichain-spl/"$so" "$so"
  else
    echo "Downloading $name $version"
    so_name="spl_${name//-/_}.so"
    (
      set -x
      curl -L --retry 5 --retry-delay 2 --retry-connrefused \
        -o "$so" \
        "https://github.com/solana-labs/solana-program-library/releases/download/$name-v$version/$so_name"
    )

    mkdir -p ~/.cache/domichain-spl
    cp "$so" ~/.cache/domichain-spl/"$so"
  fi

}

fetch_program_wasm() {
  declare name=$1
  declare version=$2
  declare address=$3
  declare loader=$4

  declare wasm=spl_$name-$version.wasm

  genesis_args+=(--wasm-program "$address" "$loader" "$wasm")

  if [[ -r $wasm ]]; then
    return
  fi

  if [[ -r ~/.cache/domichain-spl/$wasm ]]; then
    cp ~/.cache/domichain-spl/"$wasm" "$wasm"
  else
    echo "Downloading $name $version"
    (
      set -x
      curl -L --retry 5 --retry-delay 2 --retry-connrefused \
        -o "$wasm" \
        "https://github.com/Domino-Blockchain/domichain-program-library/releases/download/v1.16.1-alpha/$wasm"
    )

    mkdir -p ~/.cache/domichain-spl
    cp "$wasm" ~/.cache/domichain-spl/"$wasm"
  fi

}

fetch_program_wasm token 4.0.0 TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA WASMLoader211111111111111111111111111111111
fetch_program_wasm token-2022 0.6.1 TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb WASMLoaderUpgradeab1e1111111111111111111111
fetch_program_wasm token-btci 4.0.0 Token9SPyLZ3C9UDifE6ycx3G7PBR5zpogL5AUgAGxy WASMLoader211111111111111111111111111111111
fetch_program memo  1.0.0 Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo BPFLoader1111111111111111111111111111111111
fetch_program memo  3.0.0 MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr BPFLoader2111111111111111111111111111111111
fetch_program_wasm associated-token-account 1.0.5 ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL WASMLoader211111111111111111111111111111111
fetch_program feature-proposal 1.0.0 Feat1YXHhH6t1juaWF74WLcfv4XoNocjXA6sPWHNgAse BPFLoader2111111111111111111111111111111111
fetch_program_wasm token-metadata 1.13.3 meta3c863KN6CX6HXzfmDHbURDkfJ5HMCwUT5SEqu5C WASMLoader211111111111111111111111111111111
fetch_program_wasm serum-dex 0.5.6 dex62P6skhCTUV1TcVW2bwjTYzjEmudqv7oRs2bniSD WASMLoader211111111111111111111111111111111
fetch_program_wasm token-swap 3.0.0 SwapsVeCiPHMUAtzQWZw7RjsKjgCjhwU55QGu4U1Szw WASMLoader211111111111111111111111111111111

echo "${genesis_args[@]}" > spl-genesis-args.sh

echo
echo "Available SPL programs:"
ls -l spl_*.so
ls -l spl_*.wasm

echo
echo "domichain-genesis command-line arguments (spl-genesis-args.sh):"
cat spl-genesis-args.sh
