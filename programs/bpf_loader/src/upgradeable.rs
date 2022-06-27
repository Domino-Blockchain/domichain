domichain_sdk::declare_builtin!(
    domichain_sdk::bpf_loader_upgradeable::ID,
    domichain_bpf_loader_upgradeable_program,
    domichain_bpf_loader_program::process_instruction,
    upgradeable::id
);
