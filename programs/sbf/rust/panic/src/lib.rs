//! Example Rust-based SBF program that panics

#[cfg(all(feature = "custom-panic", target_os = "domichain"))]
#[no_mangle]
fn custom_panic(info: &core::panic::PanicInfo<'_>) {
    // Note: Full panic reporting is included here for testing purposes
    domichain_program::msg!("program custom panic enabled");
    domichain_program::msg!(&format!("{info}"));
}

extern crate domichain_program;
use domichain_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

domichain_program::entrypoint!(process_instruction);
#[allow(clippy::unnecessary_wraps)]
fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(1, 2);
    Ok(())
}
