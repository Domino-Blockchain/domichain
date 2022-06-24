#!/usr/bin/env bash

CLI_USAGE_RELPATH="../cli/usage.md"

SED_OMIT_NONMATCHING=$'\nt\nd'
SED_CMD="s:^#### domichain-(.*):* [\`\\1\`](${CLI_USAGE_RELPATH}#domichain-\\1):${SED_OMIT_NONMATCHING}"

OFFLINE_CMDS=$(grep -E '#### domichain-|--signer ' src/cli/usage.md | grep -B1 -- --signer | sed -Ee "$SED_CMD")

# Omit deprecated
grep -vE '\b(pay)\b' <<<"$OFFLINE_CMDS"
