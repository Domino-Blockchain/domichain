//! The latest WASM loader native program.
//!
//! The WASM loader is responsible for loading, finalizing, and executing WASM
//! programs. Not all networks may support the latest loader. You can use the
//! command-line tools to check if this version of the loader is supported by
//! requesting the account info for the public key below.
//!
//! The program format may change between loaders, and it is crucial to build
//! your program against the proper entrypoint semantics. All programs being
//! deployed to this WASM loader must build against the latest entrypoint version
//! located in `entrypoint.rs`.
//!
//! Note: Programs built for older loaders must use a matching entrypoint
//! version. An example is [`wasm_loader_deprecated`] which requires
//! [`entrypoint_deprecated`].
//!
//! The `domichain program deploy` CLI command uses the
//! [upgradeable WASM loader][uwasml].
//!
//! [`wasm_loader_deprecated`]: crate::wasm_loader_deprecated
//! [`entrypoint_deprecated`]: mod@crate::entrypoint_deprecated
//! [uwasml]: crate::wasm_loader_upgradeable

crate::declare_id!("WASMLoader211111111111111111111111111111111");
