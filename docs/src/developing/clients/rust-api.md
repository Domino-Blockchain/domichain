---
title: Rust API
---

Domichain's Rust crates are [published to crates.io][crates.io] and can be found
[on docs.rs with the "domichain-" prefix][docs.rs].

[crates.io]: https://crates.io/search?q=domichain-
[docs.rs]: https://docs.rs/releases/search?query=domichain-

Some important crates:

- [`domichain-program`] &mdash; Imported by programs running on Domichain, compiled
  to SBF. This crate contains many fundamental data types and is re-exported from
  [`domichain-sdk`], which cannot be imported from a Domichain program.

- [`domichain-sdk`] &mdash; The basic off-chain SDK, it re-exports
  [`domichain-program`] and adds more APIs on top of that. Most Domichain programs
  that do not run on-chain will import this.

- [`domichain-client`] &mdash; For interacting with a Domichain node via the
  [JSON RPC API](/api).

- [`domichain-cli-config`] &mdash; Loading and saving the Domichain CLI configuration
  file.

- [`domichain-clap-utils`] &mdash; Routines for setting up a CLI, using [`clap`],
  as used by the main Domichain CLI. Includes functions for loading all types of
  signers supported by the CLI.

[`domichain-program`]: https://docs.rs/domichain-program
[`domichain-sdk`]: https://docs.rs/domichain-sdk
[`domichain-client`]: https://docs.rs/domichain-client
[`domichain-cli-config`]: https://docs.rs/domichain-cli-config
[`domichain-clap-utils`]: https://docs.rs/domichain-clap-utils
[`clap`]: https://docs.rs/clap
