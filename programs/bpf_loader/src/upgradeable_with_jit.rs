domichain_sdk::declare_builtin!(
    domichain_sdk::bpf_loader_upgradeable::ID,
    domichain_bpf_loader_upgradeable_program_with_jit,
    domichain_bpf_loader_program::process_instruction_jit,
    upgradeable_with_jit::id
);
