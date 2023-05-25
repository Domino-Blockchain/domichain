domichain_sdk::declare_builtin!(
    domichain_sdk::wasm_loader_upgradeable::ID,
    domichain_wasm_loader_upgradeable_program,
    domichain_wasm_loader_program::process_instruction,
    upgradeable::id
);
