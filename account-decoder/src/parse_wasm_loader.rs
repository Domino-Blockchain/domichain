use {
    crate::{
        parse_account_data::{ParsableAccount, ParseAccountError},
        UiAccountData, UiAccountEncoding,
    },
    bincode::{deserialize, serialized_size},
    domichain_sdk::{wasm_loader_upgradeable::UpgradeableLoaderState, pubkey::Pubkey},
};

pub fn parse_wasm_upgradeable_loader(
    data: &[u8],
) -> Result<BpfUpgradeableLoaderAccountType, ParseAccountError> {
    let account_state: UpgradeableLoaderState = deserialize(data).map_err(|_| {
        ParseAccountError::AccountNotParsable(ParsableAccount::BpfUpgradeableLoader)
    })?;
    let parsed_account = match account_state {
        UpgradeableLoaderState::Uninitialized => BpfUpgradeableLoaderAccountType::Uninitialized,
        UpgradeableLoaderState::Buffer { authority_address } => {
            let offset = if authority_address.is_some() {
                UpgradeableLoaderState::size_of_buffer_metadata()
            } else {
                // This case included for code completeness; in practice, a Buffer account will
                // always have authority_address.is_some()
                UpgradeableLoaderState::size_of_buffer_metadata()
                    - serialized_size(&Pubkey::default()).unwrap() as usize
            };
            BpfUpgradeableLoaderAccountType::Buffer(UiBuffer {
                authority: authority_address.map(|pubkey| pubkey.to_string()),
                data: UiAccountData::Binary(
                    base64::encode(&data[offset as usize..]),
                    UiAccountEncoding::Base64,
                ),
            })
        }
        UpgradeableLoaderState::Program {
            programdata_address,
        } => BpfUpgradeableLoaderAccountType::Program(UiProgram {
            program_data: programdata_address.to_string(),
        }),
        UpgradeableLoaderState::ProgramData {
            slot,
            upgrade_authority_address,
        } => {
            let offset = if upgrade_authority_address.is_some() {
                UpgradeableLoaderState::size_of_programdata_metadata()
            } else {
                UpgradeableLoaderState::size_of_programdata_metadata()
                    - serialized_size(&Pubkey::default()).unwrap() as usize
            };
            BpfUpgradeableLoaderAccountType::ProgramData(UiProgramData {
                slot,
                authority: upgrade_authority_address.map(|pubkey| pubkey.to_string()),
                data: UiAccountData::Binary(
                    base64::encode(&data[offset as usize..]),
                    UiAccountEncoding::Base64,
                ),
            })
        }
    };
    Ok(parsed_account)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type", content = "info")]
pub enum BpfUpgradeableLoaderAccountType {
    Uninitialized,
    Buffer(UiBuffer),
    Program(UiProgram),
    ProgramData(UiProgramData),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiBuffer {
    pub authority: Option<String>,
    pub data: UiAccountData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiProgram {
    pub program_data: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiProgramData {
    pub slot: u64,
    pub authority: Option<String>,
    pub data: UiAccountData,
}


