domichain_sdk::declare_builtin!(
    domichain_sdk::wasm_loader::ID,
    domichain_wasm_loader_program_with_jit,
    domichain_wasm_loader_program::process_instruction_jit
);
