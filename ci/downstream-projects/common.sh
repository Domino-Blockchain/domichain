#!/usr/bin/env bash
set -e

source ci/_
source ci/semver_bash/semver.sh
source scripts/patch-crates.sh
source scripts/read-cargo-variable.sh

DOMICHAIN_VER=$(readCargoVariable version Cargo.toml)
export DOMICHAIN_VER
export DOMICHAIN_DIR=$PWD
export CARGO="$DOMICHAIN_DIR"/cargo
export CARGO_BUILD_SBF="$DOMICHAIN_DIR"/cargo-build-sbf
export CARGO_TEST_SBF="$DOMICHAIN_DIR"/cargo-test-sbf

mkdir -p target/downstream-projects
cd target/downstream-projects
