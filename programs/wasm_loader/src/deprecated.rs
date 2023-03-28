domichain_sdk::declare_builtin!(
    domichain_sdk::bpf_loader_deprecated::ID,
    domichain_bpf_loader_deprecated_program,
    domichain_bpf_loader_program::process_instruction,
    deprecated::id
);
