#!/usr/bin/env bash

set -e

# rust-analyzer doesn't support hiding noisy test calls in the call hierarchy from tests/benches
# so, here's some wild hack from ryoqun!

if [[ $1 = "doit" ]]; then
  # it's true that we put true just for truely-aligned lines
  # shellcheck disable=SC2046 # our rust files are sanely named with no need to escape
  true &&
    sed -i -e '1i#![cfg(escaped_global_cfg_test_dir)]' $(git ls-files :**/tests/**.rs :^**/build.rs) &&
    sed -i -e 's/#!\[cfg(test)\]/#![cfg(escaped_global_cfg_test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(test)\]/#[cfg(escaped_cfg_test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(not(test))\]/#[cfg(not(escaped_cfg_test))]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[bench\]/#[cfg(escaped_bench)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[test\]/#[cfg(escaped_test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[tokio::test\]/#[cfg(escaped_tokio_test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#!\[feature(test)\]/#![cfg(escaped_feature_test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    echo done
elif [[ $1 = "undoit" ]]; then
  # shellcheck disable=SC2046 # our rust files are sanely named with no need to escape
  true &&
    sed -i -e '/#!\[cfg(escaped_global_cfg_test_dir)\]/d' $(git ls-files :**/tests/**.rs :^**/build.rs) &&
    sed -i -e 's/#!\[cfg(escaped_global_cfg_test)\]/#![cfg(test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(escaped_cfg_test)\]/#[cfg(test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(not(escaped_cfg_test))\]/#[cfg(not(test))]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(escaped_bench)\]/#[bench]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(escaped_test)\]/#[test]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#\[cfg(escaped_tokio_test)\]/#[tokio::test]/g' $(git ls-files :**.rs :^**/build.rs) &&
    sed -i -e 's/#!\[cfg(escaped_feature_test)\]/#![feature(test)]/g' $(git ls-files :**.rs :^**/build.rs) &&
    echo done
else
  echo "usage: $0 [doit|undoit]" > /dev/stderr
  exit 1
fi
