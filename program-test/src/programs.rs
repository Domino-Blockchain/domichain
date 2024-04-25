use domichain_sdk::{
    account::{Account, AccountSharedData},
    bpf_loader_upgradeable::UpgradeableLoaderState,
    pubkey::Pubkey,
    rent::Rent,
};

mod spl_token {
    domichain_sdk::declare_id!("TokenAAGbeQq5tGW2r5RoR3oauzN2EkNFiHNPw9q34s");
}
mod spl_token_btci {
    domichain_sdk::declare_id!("BTCi9FUjBVY3BSaqjzfhEPKVExuvarj8Gtfn4rJ5soLC");
}
mod spl_token_2022 {
    domichain_sdk::declare_id!("BvVePGKKwuGb6QVJHG6LvCrULB7QBgjocqnYxYHUkNEd");
}
mod spl_memo_1_0 {
    domichain_sdk::declare_id!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");
}
mod spl_memo_3_0 {
    domichain_sdk::declare_id!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
}
mod spl_associated_token_account {
    domichain_sdk::declare_id!("Dt8fRCpjeV6JDemhPmtcTKijgKdPxXHn9Wo9cXY5agtG");
}

static SPL_PROGRAMS: &[(Pubkey, Pubkey, &[u8])] = &[
    (
        spl_token::ID,
        domichain_sdk::wasm_loader::ID,
        include_bytes!("programs/spl_token-4.0.0.wasm"),
    ),
    (
        spl_token_btci::ID,
        domichain_sdk::wasm_loader::ID,
        include_bytes!("programs/spl_token-btci-4.0.0.wasm"),
    ),
    (
        spl_token_2022::ID,
        domichain_sdk::wasm_loader_upgradeable::ID,
        include_bytes!("programs/spl_token_2022-0.7.0.wasm"),
    ),
    (
        spl_memo_1_0::ID,
        domichain_sdk::bpf_loader::ID,
        include_bytes!("programs/spl_memo-1.0.0.so"),
    ),
    (
        spl_memo_3_0::ID,
        domichain_sdk::bpf_loader::ID,
        include_bytes!("programs/spl_memo-3.0.0.so"),
    ),
    (
        spl_associated_token_account::ID,
        domichain_sdk::bpf_loader::ID,
        include_bytes!("programs/spl_associated_token_account-1.1.1.so"),
    ),
];

pub fn spl_programs(rent: &Rent) -> Vec<(Pubkey, AccountSharedData)> {
    SPL_PROGRAMS
        .iter()
        .flat_map(|(program_id, loader_id, elf)| {
            let mut accounts = vec![];
            let data = if *loader_id == domichain_sdk::bpf_loader_upgradeable::ID {
                let (programdata_address, _) =
                    Pubkey::find_program_address(&[program_id.as_ref()], loader_id);
                let mut program_data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
                    slot: 0,
                    upgrade_authority_address: Some(Pubkey::default()),
                })
                .unwrap();
                program_data.extend_from_slice(elf);
                accounts.push((
                    programdata_address,
                    AccountSharedData::from(Account {
                        satomis: rent.minimum_balance(program_data.len()).max(1),
                        data: program_data,
                        owner: *loader_id,
                        executable: false,
                        rent_epoch: 0,
                    }),
                ));
                bincode::serialize(&UpgradeableLoaderState::Program {
                    programdata_address,
                })
                .unwrap()
            } else {
                elf.to_vec()
            };
            accounts.push((
                *program_id,
                AccountSharedData::from(Account {
                    satomis: rent.minimum_balance(data.len()).max(1),
                    data,
                    owner: *loader_id,
                    executable: true,
                    rent_epoch: 0,
                }),
            ));
            accounts.into_iter()
        })
        .collect()
}
