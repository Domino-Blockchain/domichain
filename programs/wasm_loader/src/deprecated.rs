domichain_sdk::declare_builtin!(
    domichain_sdk::wasm_loader_deprecated::ID,
    domichain_wasm_loader_deprecated_program,
    domichain_wasm_loader_program::process_instruction,
    deprecated::id
);
