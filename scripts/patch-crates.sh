# source this file

update_domichain_dependencies() {
  declare project_root="$1"
  declare domichain_ver="$2"
  declare tomls=()
  while IFS='' read -r line; do tomls+=("$line"); done < <(find "$project_root" -name Cargo.toml)

  sed -i -e "s#\(domichain-program = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-program-test = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-sdk = \"\).*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-sdk = { version = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-client = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-client = { version = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-clap-utils = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-clap-utils = { version = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-account-decoder = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-account-decoder = { version = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-faucet = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-faucet = { version = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-zk-token-sdk = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
  sed -i -e "s#\(domichain-zk-token-sdk = { version = \"\)[^\"]*\(\"\)#\1=$domichain_ver\2#g" "${tomls[@]}" || return $?
}

patch_crates_io_domichain() {
  declare Cargo_toml="$1"
  declare domichain_dir="$2"
  cat >> "$Cargo_toml" <<EOF
[patch.crates-io]
domichain-account-decoder = { path = "$domichain_dir/account-decoder" }
domichain-clap-utils = { path = "$domichain_dir/clap-utils" }
domichain-client = { path = "$domichain_dir/client" }
domichain-program = { path = "$domichain_dir/sdk/program" }
domichain-program-test = { path = "$domichain_dir/program-test" }
domichain-sdk = { path = "$domichain_dir/sdk" }
domichain-faucet = { path = "$domichain_dir/faucet" }
domichain-zk-token-sdk = { path = "$domichain_dir/zk-token-sdk" }
EOF
}
