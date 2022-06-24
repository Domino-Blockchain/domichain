#!/usr/bin/env bash
#
# Builds known downstream projects against local domichain source
#

set -e
cd "$(dirname "$0")"/..
source ci/_
source scripts/patch-crates.sh
source scripts/read-cargo-variable.sh

domichain_ver=$(readCargoVariable version sdk/Cargo.toml)
domichain_dir=$PWD
cargo="$domichain_dir"/cargo
cargo_build_bpf="$domichain_dir"/cargo-build-bpf
cargo_test_bpf="$domichain_dir"/cargo-test-bpf

mkdir -p target/downstream-projects
cd target/downstream-projects

example_helloworld() {
  (
    set -x
    rm -rf example-helloworld
    git clone https://github.com/domichain-labs/example-helloworld.git
    cd example-helloworld

    update_domichain_dependencies src/program-rust "$domichain_ver"
    patch_crates_io_domichain src/program-rust/Cargo.toml "$domichain_dir"
    echo "[workspace]" >> src/program-rust/Cargo.toml

    $cargo_build_bpf \
      --manifest-path src/program-rust/Cargo.toml

    # TODO: Build src/program-c/...
  )
}

spl() {
  (
    # Mind the order!
    PROGRAMS=(
      token/program
      token/program-2022
      token/program-2022-test
      associated-token-account/program
      feature-proposal/program
      governance/addin-mock/program
      governance/program
      memo/program
      name-service/program
      stake-pool/program
      )
    set -x
    rm -rf spl
    git clone https://github.com/domichain-labs/domichain-program-library.git spl
    cd spl

    ./patch.crates-io.sh "$domichain_dir"

    for program in "${PROGRAMS[@]}"; do
      $cargo_test_bpf --manifest-path "$program"/Cargo.toml
    done

    # TODO better: `build.rs` for spl-token-cli doesn't seem to properly build
    # the required programs to run the tests, so instead we run the tests
    # after we know programs have been built
    $cargo build
    $cargo test
  )
}

serum_dex() {
  (
    set -x
    rm -rf serum-dex
    git clone https://github.com/project-serum/serum-dex.git
    cd serum-dex

    update_domichain_dependencies . "$domichain_ver"
    patch_crates_io_domichain Cargo.toml "$domichain_dir"
    patch_crates_io_domichain dex/Cargo.toml "$domichain_dir"
    cat >> dex/Cargo.toml <<EOF
[workspace]
exclude = [
    "crank",
    "permissioned",
]
EOF
    $cargo build

    $cargo_build_bpf \
      --manifest-path dex/Cargo.toml --no-default-features --features program

    $cargo test \
      --manifest-path dex/Cargo.toml --no-default-features --features program
  )
}

_ example_helloworld
_ spl
_ serum_dex
