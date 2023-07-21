use {
    domichain_program_runtime::invoke_context::ProcessInstructionWithContext,
    domichain_sdk::{
        bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable,
        wasm_loader, wasm_loader_deprecated, wasm_loader_upgradeable,
        feature_set, pubkey::Pubkey,
    },
};

/// Transitions of built-in programs at epoch bondaries when features are activated.
pub struct BuiltinPrototype {
    pub feature_id: Option<Pubkey>,
    pub program_id: Pubkey,
    pub name: &'static str,
    pub entrypoint: ProcessInstructionWithContext,
}

impl std::fmt::Debug for BuiltinPrototype {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut builder = f.debug_struct("BuiltinPrototype");
        builder.field("program_id", &self.program_id);
        builder.field("name", &self.name);
        builder.field("feature_id", &self.feature_id);
        builder.finish()
    }
}

#[cfg(RUSTC_WITH_SPECIALIZATION)]
impl domichain_frozen_abi::abi_example::AbiExample for BuiltinPrototype {
    fn example() -> Self {
        // BuiltinPrototype isn't serializable by definition.
        domichain_program_runtime::declare_process_instruction!(entrypoint, 0, |_invoke_context| {
            // Do nothing
            Ok(())
        });
        Self {
            feature_id: None,
            program_id: Pubkey::default(),
            name: "",
            entrypoint,
        }
    }
}

pub static BUILTINS: &[BuiltinPrototype] = &[
    BuiltinPrototype {
        feature_id: None,
        program_id: domichain_system_program::id(),
        name: "system_program",
        entrypoint: domichain_system_program::system_processor::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: domichain_vote_program::id(),
        name: "vote_program",
        entrypoint: domichain_vote_program::vote_processor::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: domichain_stake_program::id(),
        name: "stake_program",
        entrypoint: domichain_stake_program::stake_instruction::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: domichain_config_program::id(),
        name: "config_program",
        entrypoint: domichain_config_program::config_processor::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: bpf_loader_deprecated::id(),
        name: "domichain_bpf_loader_deprecated_program",
        entrypoint: domichain_bpf_loader_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: bpf_loader::id(),
        name: "domichain_bpf_loader_program",
        entrypoint: domichain_bpf_loader_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: bpf_loader_upgradeable::id(),
        name: "domichain_bpf_loader_upgradeable_program",
        entrypoint: domichain_bpf_loader_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: wasm_loader_deprecated::id(),
        name: "domichain_wasm_loader_deprecated_program",
        entrypoint: domichain_wasm_loader_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: wasm_loader::id(),
        name: "domichain_wasm_loader_program",
        entrypoint: domichain_wasm_loader_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: wasm_loader_upgradeable::id(),
        name: "domichain_wasm_loader_upgradeable_program",
        entrypoint: domichain_wasm_loader_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: domichain_sdk::compute_budget::id(),
        name: "compute_budget_program",
        entrypoint: domichain_compute_budget_program::process_instruction,
    },
    BuiltinPrototype {
        feature_id: None,
        program_id: domichain_address_lookup_table_program::id(),
        name: "address_lookup_table_program",
        entrypoint: domichain_address_lookup_table_program::processor::process_instruction,
    },
    BuiltinPrototype {
        feature_id: Some(feature_set::zk_token_sdk_enabled::id()),
        program_id: domichain_zk_token_sdk::zk_token_proof_program::id(),
        name: "zk_token_proof_program",
        entrypoint: domichain_zk_token_proof_program::process_instruction,
    },
];
