//! The original and now deprecated Domichain WASM loader.
//!
//! The WASM loader is responsible for loading, finalizing, and executing WASM
//! programs.
//!
//! This loader is deprecated, and it is strongly encouraged to build for and
//! deploy to the latest WASM loader.  For more information see `wasm_loader.rs`
//!
//! The program format may change between loaders, and it is crucial to build
//! your program against the proper entrypoint semantics.  All programs being
//! deployed to this WASM loader must build against the deprecated entrypoint
//! version located in `entrypoint_deprecated.rs`.

crate::declare_id!("WASMLoader111111111111111111111111111111111");
