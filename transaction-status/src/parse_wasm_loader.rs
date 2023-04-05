use {
    crate::parse_instruction::{
        check_num_accounts, ParsableProgram, ParseInstructionError, ParsedInstructionEnum,
    },
    bincode::deserialize,
    serde_json::json,
    domichain_sdk::{
        instruction::CompiledInstruction, loader_instruction::LoaderInstruction,
        loader_upgradeable_instruction::UpgradeableLoaderInstruction, message::AccountKeys,
    },
};

pub fn parse_wasm_loader(
    instruction: &CompiledInstruction,
    account_keys: &AccountKeys,
) -> Result<ParsedInstructionEnum, ParseInstructionError> {
    todo!();
    let bpf_loader_instruction: LoaderInstruction = deserialize(&instruction.data)
        .map_err(|_| ParseInstructionError::InstructionNotParsable(ParsableProgram::BpfLoader))?;
    if instruction.accounts.is_empty() || instruction.accounts[0] as usize >= account_keys.len() {
        return Err(ParseInstructionError::InstructionKeyMismatch(
            ParsableProgram::BpfLoader,
        ));
    }
    match bpf_loader_instruction {
        LoaderInstruction::Write { offset, bytes } => {
            check_num_bpf_loader_accounts(&instruction.accounts, 1)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "write".to_string(),
                info: json!({
                    "offset": offset,
                    "bytes": base64::encode(bytes),
                    "account": account_keys[instruction.accounts[0] as usize].to_string(),
                }),
            })
        }
        LoaderInstruction::Finalize => {
            check_num_bpf_loader_accounts(&instruction.accounts, 2)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "finalize".to_string(),
                info: json!({
                    "account": account_keys[instruction.accounts[0] as usize].to_string(),
                }),
            })
        }
    }
}

pub fn parse_wasm_upgradeable_loader(
    instruction: &CompiledInstruction,
    account_keys: &AccountKeys,
) -> Result<ParsedInstructionEnum, ParseInstructionError> {
    todo!();
    let bpf_upgradeable_loader_instruction: UpgradeableLoaderInstruction =
        deserialize(&instruction.data).map_err(|_| {
            ParseInstructionError::InstructionNotParsable(ParsableProgram::BpfUpgradeableLoader)
        })?;
    match instruction.accounts.iter().max() {
        Some(index) if (*index as usize) < account_keys.len() => {}
        _ => {
            // Runtime should prevent this from ever happening
            return Err(ParseInstructionError::InstructionKeyMismatch(
                ParsableProgram::BpfUpgradeableLoader,
            ));
        }
    }
    match bpf_upgradeable_loader_instruction {
        UpgradeableLoaderInstruction::InitializeBuffer => {
            check_num_bpf_upgradeable_loader_accounts(&instruction.accounts, 1)?;
            let mut value = json!({
                "account": account_keys[instruction.accounts[0] as usize].to_string(),
            });
            let map = value.as_object_mut().unwrap();
            if instruction.accounts.len() > 1 {
                map.insert(
                    "authority".to_string(),
                    json!(account_keys[instruction.accounts[1] as usize].to_string()),
                );
            }
            Ok(ParsedInstructionEnum {
                instruction_type: "initializeBuffer".to_string(),
                info: value,
            })
        }
        UpgradeableLoaderInstruction::Write { offset, bytes } => {
            check_num_bpf_upgradeable_loader_accounts(&instruction.accounts, 2)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "write".to_string(),
                info: json!({
                    "offset": offset,
                    "bytes": base64::encode(bytes),
                    "account": account_keys[instruction.accounts[0] as usize].to_string(),
                    "authority": account_keys[instruction.accounts[1] as usize].to_string(),
                }),
            })
        }
        UpgradeableLoaderInstruction::DeployWithMaxDataLen { max_data_len } => {
            check_num_bpf_upgradeable_loader_accounts(&instruction.accounts, 8)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "deployWithMaxDataLen".to_string(),
                info: json!({
                    "maxDataLen": max_data_len,
                    "payerAccount": account_keys[instruction.accounts[0] as usize].to_string(),
                    "programDataAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                    "programAccount": account_keys[instruction.accounts[2] as usize].to_string(),
                    "bufferAccount": account_keys[instruction.accounts[3] as usize].to_string(),
                    "rentSysvar": account_keys[instruction.accounts[4] as usize].to_string(),
                    "clockSysvar": account_keys[instruction.accounts[5] as usize].to_string(),
                    "systemProgram": account_keys[instruction.accounts[6] as usize].to_string(),
                    "authority": account_keys[instruction.accounts[7] as usize].to_string(),
                }),
            })
        }
        UpgradeableLoaderInstruction::Upgrade => {
            check_num_bpf_upgradeable_loader_accounts(&instruction.accounts, 7)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "upgrade".to_string(),
                info: json!({
                    "programDataAccount": account_keys[instruction.accounts[0] as usize].to_string(),
                    "programAccount": account_keys[instruction.accounts[1] as usize].to_string(),
                    "bufferAccount": account_keys[instruction.accounts[2] as usize].to_string(),
                    "spillAccount": account_keys[instruction.accounts[3] as usize].to_string(),
                    "rentSysvar": account_keys[instruction.accounts[4] as usize].to_string(),
                    "clockSysvar": account_keys[instruction.accounts[5] as usize].to_string(),
                    "authority": account_keys[instruction.accounts[6] as usize].to_string(),
                }),
            })
        }
        UpgradeableLoaderInstruction::SetAuthority => {
            check_num_bpf_upgradeable_loader_accounts(&instruction.accounts, 2)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "setAuthority".to_string(),
                info: json!({
                    "account": account_keys[instruction.accounts[0] as usize].to_string(),
                    "authority": account_keys[instruction.accounts[1] as usize].to_string(),
                    "newAuthority": if instruction.accounts.len() > 2 {
                        Some(account_keys[instruction.accounts[2] as usize].to_string())
                    } else {
                        None
                    },
                }),
            })
        }
        UpgradeableLoaderInstruction::Close => {
            check_num_bpf_upgradeable_loader_accounts(&instruction.accounts, 3)?;
            Ok(ParsedInstructionEnum {
                instruction_type: "close".to_string(),
                info: json!({
                    "account": account_keys[instruction.accounts[0] as usize].to_string(),
                    "recipient": account_keys[instruction.accounts[1] as usize].to_string(),
                    "authority": account_keys[instruction.accounts[2] as usize].to_string()
                }),
            })
        }
    }
}

fn check_num_bpf_loader_accounts(accounts: &[u8], num: usize) -> Result<(), ParseInstructionError> {
    check_num_accounts(accounts, num, ParsableProgram::BpfLoader)
}

fn check_num_bpf_upgradeable_loader_accounts(
    accounts: &[u8],
    num: usize,
) -> Result<(), ParseInstructionError> {
    check_num_accounts(accounts, num, ParsableProgram::BpfUpgradeableLoader)
}


