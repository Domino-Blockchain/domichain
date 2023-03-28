domichain_sdk::declare_builtin!(
    domichain_sdk::bpf_loader::ID,
    domichain_bpf_loader_program_with_jit,
    domichain_bpf_loader_program::process_instruction_jit
);
