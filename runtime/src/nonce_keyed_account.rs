use {
    domichain_program_runtime::{ic_msg, invoke_context::InvokeContext},
    domichain_sdk::{
        feature_set::{self, nonce_must_be_writable},
        instruction::{checked_add, InstructionError},
        nonce::{
            self,
            state::{DurableNonce, Versions},
            State,
        },
        pubkey::Pubkey,
        system_instruction::{nonce_to_instruction_error, NonceError},
        sysvar::rent::Rent,
        transaction_context::{BorrowedAccount, InstructionContext, TransactionContext},
    },
    std::collections::HashSet,
};

fn get_durable_nonce(invoke_context: &InvokeContext) -> (DurableNonce, /*separate_domains:*/ bool) {
    let separate_nonce_from_blockhash = invoke_context
        .feature_set
        .is_active(&feature_set::separate_nonce_from_blockhash::id());
    let durable_nonce =
        DurableNonce::from_blockhash(&invoke_context.blockhash, separate_nonce_from_blockhash);
    (durable_nonce, separate_nonce_from_blockhash)
}

pub fn advance_nonce_account(
    account: &mut BorrowedAccount,
    signers: &HashSet<Pubkey>,
    invoke_context: &InvokeContext,
) -> Result<(), InstructionError> {
    let merge_nonce_error_into_system_error = invoke_context
        .feature_set
        .is_active(&feature_set::merge_nonce_error_into_system_error::id());

    if invoke_context
        .feature_set
        .is_active(&nonce_must_be_writable::id())
        && !account.is_writable()
    {
        ic_msg!(
            invoke_context,
            "Advance nonce account: Account {} must be writeable",
            account.get_key()
        );
        return Err(InstructionError::InvalidArgument);
    }

    let state: Versions = account.get_state()?;
    match state.state() {
        State::Initialized(data) => {
            if !signers.contains(&data.authority) {
                ic_msg!(
                    invoke_context,
                    "Advance nonce account: Account {} must be a signer",
                    data.authority
                );
                return Err(InstructionError::MissingRequiredSignature);
            }
            let (next_durable_nonce, separate_domains) = get_durable_nonce(invoke_context);
            if data.durable_nonce == next_durable_nonce {
                ic_msg!(
                    invoke_context,
                    "Advance nonce account: nonce can only advance once per slot"
                );
                return Err(nonce_to_instruction_error(
                    NonceError::NotExpired,
                    merge_nonce_error_into_system_error,
                ));
            }

            let new_data = nonce::state::Data::new(
                data.authority,
                next_durable_nonce,
                invoke_context.satomis_per_signature,
            );
            account.set_state(&Versions::new(
                State::Initialized(new_data),
                separate_domains,
            ))
        }
        State::Uninitialized => {
            ic_msg!(
                invoke_context,
                "Advance nonce account: Account {} state is invalid",
                account.get_key()
            );
            Err(nonce_to_instruction_error(
                NonceError::BadAccountState,
                merge_nonce_error_into_system_error,
            ))
        }
    }
}

pub fn withdraw_nonce_account(
    from_account_index: usize,
    satomis: u64,
    to_account_index: usize,
    rent: &Rent,
    signers: &HashSet<Pubkey>,
    invoke_context: &InvokeContext,
    transaction_context: &TransactionContext,
    instruction_context: &InstructionContext,
) -> Result<(), InstructionError> {
    let mut from = instruction_context
        .try_borrow_instruction_account(transaction_context, from_account_index)?;
    let merge_nonce_error_into_system_error = invoke_context
        .feature_set
        .is_active(&feature_set::merge_nonce_error_into_system_error::id());

    if invoke_context
        .feature_set
        .is_active(&nonce_must_be_writable::id())
        && !from.is_writable()
    {
        ic_msg!(
            invoke_context,
            "Withdraw nonce account: Account {} must be writeable",
            from.get_key()
        );
        return Err(InstructionError::InvalidArgument);
    }

    let state: Versions = from.get_state()?;
    let signer = match state.state() {
        State::Uninitialized => {
            if satomis > from.get_satomis() {
                ic_msg!(
                    invoke_context,
                    "Withdraw nonce account: insufficient satomis {}, need {}",
                    from.get_satomis(),
                    satomis,
                );
                return Err(InstructionError::InsufficientFunds);
            }
            *from.get_key()
        }
        State::Initialized(ref data) => {
            if satomis == from.get_satomis() {
                let (durable_nonce, separate_domains) = get_durable_nonce(invoke_context);
                if data.durable_nonce == durable_nonce {
                    ic_msg!(
                        invoke_context,
                        "Withdraw nonce account: nonce can only advance once per slot"
                    );
                    return Err(nonce_to_instruction_error(
                        NonceError::NotExpired,
                        merge_nonce_error_into_system_error,
                    ));
                }
                from.set_state(&Versions::new(State::Uninitialized, separate_domains))?;
            } else {
                let min_balance = rent.minimum_balance(from.get_data().len());
                let amount = checked_add(satomis, min_balance)?;
                if amount > from.get_satomis() {
                    ic_msg!(
                        invoke_context,
                        "Withdraw nonce account: insufficient satomis {}, need {}",
                        from.get_satomis(),
                        amount,
                    );
                    return Err(InstructionError::InsufficientFunds);
                }
            }
            data.authority
        }
    };

    if !signers.contains(&signer) {
        ic_msg!(
            invoke_context,
            "Withdraw nonce account: Account {} must sign",
            signer
        );
        return Err(InstructionError::MissingRequiredSignature);
    }

    from.checked_sub_satomis(satomis)
        .map_err(|_| InstructionError::ArithmeticOverflow)?;
    drop(from);
    let mut to = instruction_context
        .try_borrow_instruction_account(transaction_context, to_account_index)?;
    to.checked_add_satomis(satomis)
        .map_err(|_| InstructionError::ArithmeticOverflow)?;

    Ok(())
}

pub fn initialize_nonce_account(
    account: &mut BorrowedAccount,
    nonce_authority: &Pubkey,
    rent: &Rent,
    invoke_context: &InvokeContext,
) -> Result<(), InstructionError> {
    let merge_nonce_error_into_system_error = invoke_context
        .feature_set
        .is_active(&feature_set::merge_nonce_error_into_system_error::id());

    if invoke_context
        .feature_set
        .is_active(&nonce_must_be_writable::id())
        && !account.is_writable()
    {
        ic_msg!(
            invoke_context,
            "Initialize nonce account: Account {} must be writeable",
            account.get_key()
        );
        return Err(InstructionError::InvalidArgument);
    }

    match account.get_state::<Versions>()?.state() {
        State::Uninitialized => {
            let min_balance = rent.minimum_balance(account.get_data().len());
            if account.get_satomis() < min_balance {
                ic_msg!(
                    invoke_context,
                    "Initialize nonce account: insufficient satomis {}, need {}",
                    account.get_satomis(),
                    min_balance
                );
                return Err(InstructionError::InsufficientFunds);
            }
            let (durable_nonce, separate_domains) = get_durable_nonce(invoke_context);
            let data = nonce::state::Data::new(
                *nonce_authority,
                durable_nonce,
                invoke_context.satomis_per_signature,
            );
            let state = State::Initialized(data);
            account.set_state(&Versions::new(state, separate_domains))
        }
        State::Initialized(_) => {
            ic_msg!(
                invoke_context,
                "Initialize nonce account: Account {} state is invalid",
                account.get_key()
            );
            Err(nonce_to_instruction_error(
                NonceError::BadAccountState,
                merge_nonce_error_into_system_error,
            ))
        }
    }
}

pub fn authorize_nonce_account(
    account: &mut BorrowedAccount,
    nonce_authority: &Pubkey,
    signers: &HashSet<Pubkey>,
    invoke_context: &InvokeContext,
) -> Result<(), InstructionError> {
    let merge_nonce_error_into_system_error = invoke_context
        .feature_set
        .is_active(&feature_set::merge_nonce_error_into_system_error::id());

    if invoke_context
        .feature_set
        .is_active(&nonce_must_be_writable::id())
        && !account.is_writable()
    {
        ic_msg!(
            invoke_context,
            "Authorize nonce account: Account {} must be writeable",
            account.get_key()
        );
        return Err(InstructionError::InvalidArgument);
    }

    let state: Versions = account.get_state()?;
    let separate_domains = state.separate_domains();
    match state.state() {
        State::Initialized(data) => {
            if !signers.contains(&data.authority) {
                ic_msg!(
                    invoke_context,
                    "Authorize nonce account: Account {} must sign",
                    data.authority
                );
                return Err(InstructionError::MissingRequiredSignature);
            }
            let new_data = nonce::state::Data::new(
                *nonce_authority,
                data.durable_nonce,
                data.get_satomis_per_signature(),
            );
            account.set_state(&Versions::new(
                State::Initialized(new_data),
                separate_domains,
            ))
        }
        State::Uninitialized => {
            ic_msg!(
                invoke_context,
                "Authorize nonce account: Account {} state is invalid",
                account.get_key()
            );
            Err(nonce_to_instruction_error(
                NonceError::BadAccountState,
                merge_nonce_error_into_system_error,
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        assert_matches::assert_matches,
        domichain_program_runtime::invoke_context::InvokeContext,
        domichain_sdk::{
            account::AccountSharedData,
            hash::{hash, Hash},
            nonce::{self, State},
            nonce_account::{create_account, verify_nonce_account},
            system_instruction::SystemError,
            system_program,
            transaction_context::{InstructionAccount, TransactionContext},
        },
    };

    pub const NONCE_ACCOUNT_INDEX: usize = 0;
    pub const WITHDRAW_TO_ACCOUNT_INDEX: usize = 1;

    macro_rules! push_instruction_context {
        ($invoke_context:expr, $transaction_context:ident, $instruction_context:ident, $instruction_accounts:ident) => {
            $invoke_context
                .push(&$instruction_accounts, &[2], &[])
                .unwrap();
            let $transaction_context = &$invoke_context.transaction_context;
            let $instruction_context = $transaction_context
                .get_current_instruction_context()
                .unwrap();
        };
    }

    macro_rules! prepare_mockup {
        ($invoke_context:ident, $instruction_accounts:ident, $rent:ident) => {
            let $rent = Rent {
                satomis_per_byte_year: 42,
                ..Rent::default()
            };
            let from_satomis = $rent.minimum_balance(State::size()) + 42;
            let accounts = vec![
                (
                    Pubkey::new_unique(),
                    create_account(from_satomis, /*separate_domains:*/ true).into_inner(),
                ),
                (
                    Pubkey::new_unique(),
                    create_account(42, /*separate_domains:*/ true).into_inner(),
                ),
                (system_program::id(), AccountSharedData::default()),
            ];
            let $instruction_accounts = vec![
                InstructionAccount {
                    index_in_transaction: 0,
                    index_in_caller: 0,
                    index_in_callee: 0,
                    is_signer: true,
                    is_writable: true,
                },
                InstructionAccount {
                    index_in_transaction: 1,
                    index_in_caller: 1,
                    index_in_callee: 1,
                    is_signer: false,
                    is_writable: true,
                },
            ];
            let mut transaction_context = TransactionContext::new(accounts, 1, 2);
            let mut $invoke_context = InvokeContext::new_mock(&mut transaction_context, &[]);
        };
    }

    macro_rules! set_invoke_context_blockhash {
        ($invoke_context:expr, $seed:expr) => {
            $invoke_context.blockhash = hash(&bincode::serialize(&$seed).unwrap());
            $invoke_context.satomis_per_signature = ($seed as u64).saturating_mul(100);
        };
    }

    #[test]
    fn default_is_uninitialized() {
        assert_eq!(State::default(), State::Uninitialized)
    }

    #[test]
    fn expected_behavior() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let data = nonce::state::Data {
            authority: *nonce_account.get_key(),
            ..nonce::state::Data::default()
        };
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        let versions = nonce_account.get_state::<Versions>().unwrap();
        // New is in Uninitialzed state
        assert_eq!(versions.state(), &State::Uninitialized);
        set_invoke_context_blockhash!(invoke_context, 95);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        let data = nonce::state::Data::new(
            data.authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        // First nonce instruction drives state from Uninitialized to Initialized
        assert_eq!(versions.state(), &State::Initialized(data.clone()));
        set_invoke_context_blockhash!(invoke_context, 63);
        advance_nonce_account(&mut nonce_account, &signers, &invoke_context).unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        let data = nonce::state::Data::new(
            data.authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        // Second nonce instruction consumes and replaces stored nonce
        assert_eq!(versions.state(), &State::Initialized(data.clone()));
        set_invoke_context_blockhash!(invoke_context, 31);
        advance_nonce_account(&mut nonce_account, &signers, &invoke_context).unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        let data = nonce::state::Data::new(
            data.authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        // Third nonce instruction for fun and profit
        assert_eq!(versions.state(), &State::Initialized(data));

        set_invoke_context_blockhash!(invoke_context, 0);
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let withdraw_satomis = nonce_account.get_satomis();
        let expect_nonce_satomis = nonce_account.get_satomis() - withdraw_satomis;
        let expect_to_satomis = to_account.get_satomis() + withdraw_satomis;
        drop(nonce_account);
        drop(to_account);
        withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        )
        .unwrap();
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        // Empties Account balance
        assert_eq!(nonce_account.get_satomis(), expect_nonce_satomis);
        // Account balance goes to `to`
        assert_eq!(to_account.get_satomis(), expect_to_satomis);
        let versions = nonce_account.get_state::<Versions>().unwrap();
        // Empty balance deinitializes data
        assert_eq!(versions.state(), &State::Uninitialized);
    }

    #[test]
    fn nonce_inx_initialized_account_not_signer_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        set_invoke_context_blockhash!(invoke_context, 31);
        let authority = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authority, &rent, &invoke_context).unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        let data = nonce::state::Data::new(
            authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        assert_eq!(versions.state(), &State::Initialized(data));
        // Nonce account did not sign
        let signers = HashSet::new();
        set_invoke_context_blockhash!(invoke_context, 0);
        let result = advance_nonce_account(&mut nonce_account, &signers, &invoke_context);
        assert_eq!(result, Err(InstructionError::MissingRequiredSignature));
    }

    #[test]
    fn nonce_inx_too_early_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 63);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let result = advance_nonce_account(&mut nonce_account, &signers, &invoke_context);
        assert_eq!(result, Err(SystemError::NonceBlockhashNotExpired.into()));
    }

    #[test]
    fn nonce_inx_uninitialized_account_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 63);
        let result = advance_nonce_account(&mut nonce_account, &signers, &invoke_context);
        assert_eq!(result, Err(InstructionError::InvalidAccountData));
    }

    #[test]
    fn nonce_inx_independent_nonce_authority_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let nonce_authority = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX + 1)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 63);
        let authorized = *nonce_authority.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let mut signers = HashSet::new();
        signers.insert(authorized);
        set_invoke_context_blockhash!(invoke_context, 31);
        let result = advance_nonce_account(&mut nonce_account, &signers, &invoke_context);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn nonce_inx_no_nonce_authority_sig_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let nonce_authority = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX + 1)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 63);
        let authorized = *nonce_authority.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let result = advance_nonce_account(&mut nonce_account, &signers, &invoke_context);
        assert_eq!(result, Err(InstructionError::MissingRequiredSignature));
    }

    #[test]
    fn withdraw_inx_unintialized_acc_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 0);
        let withdraw_satomis = nonce_account.get_satomis();
        let expect_from_satomis = nonce_account.get_satomis() - withdraw_satomis;
        let expect_to_satomis = to_account.get_satomis() + withdraw_satomis;
        drop(nonce_account);
        drop(to_account);
        withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        )
        .unwrap();
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        assert_eq!(nonce_account.get_satomis(), expect_from_satomis);
        assert_eq!(to_account.get_satomis(), expect_to_satomis);
    }

    #[test]
    fn withdraw_inx_unintialized_acc_unsigned_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        let signers = HashSet::new();
        set_invoke_context_blockhash!(invoke_context, 0);
        let withdraw_satomis = nonce_account.get_satomis();
        drop(nonce_account);
        drop(to_account);
        let result = withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        );
        assert_eq!(result, Err(InstructionError::MissingRequiredSignature));
    }

    #[test]
    fn withdraw_inx_unintialized_acc_insuff_funds_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 0);
        let withdraw_satomis = nonce_account.get_satomis() + 1;
        drop(nonce_account);
        let result = withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        );
        assert_eq!(result, Err(InstructionError::InsufficientFunds));
    }

    #[test]
    fn withdraw_inx_uninitialized_acc_two_withdraws_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 0);
        let withdraw_satomis = nonce_account.get_satomis() / 2;
        let from_expect_satomis = nonce_account.get_satomis() - withdraw_satomis;
        let to_expect_satomis = to_account.get_satomis() + withdraw_satomis;
        drop(nonce_account);
        drop(to_account);
        withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        )
        .unwrap();
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        assert_eq!(nonce_account.get_satomis(), from_expect_satomis);
        assert_eq!(to_account.get_satomis(), to_expect_satomis);
        let withdraw_satomis = nonce_account.get_satomis();
        let from_expect_satomis = nonce_account.get_satomis() - withdraw_satomis;
        let to_expect_satomis = to_account.get_satomis() + withdraw_satomis;
        drop(nonce_account);
        drop(to_account);
        withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        )
        .unwrap();
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        assert_eq!(nonce_account.get_satomis(), from_expect_satomis);
        assert_eq!(to_account.get_satomis(), to_expect_satomis);
    }

    #[test]
    fn withdraw_inx_initialized_acc_two_withdraws_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 31);
        let authority = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authority, &rent, &invoke_context).unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        let data = nonce::state::Data::new(
            authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        assert_eq!(versions.state(), &State::Initialized(data.clone()));
        let withdraw_satomis = 42;
        let from_expect_satomis = nonce_account.get_satomis() - withdraw_satomis;
        let to_expect_satomis = to_account.get_satomis() + withdraw_satomis;
        drop(nonce_account);
        drop(to_account);
        withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        )
        .unwrap();
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        let data = nonce::state::Data::new(
            data.authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        assert_eq!(versions.state(), &State::Initialized(data));
        assert_eq!(nonce_account.get_satomis(), from_expect_satomis);
        assert_eq!(to_account.get_satomis(), to_expect_satomis);
        set_invoke_context_blockhash!(invoke_context, 0);
        let withdraw_satomis = nonce_account.get_satomis();
        let from_expect_satomis = nonce_account.get_satomis() - withdraw_satomis;
        let to_expect_satomis = to_account.get_satomis() + withdraw_satomis;
        drop(nonce_account);
        drop(to_account);
        withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        )
        .unwrap();
        let nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        assert_eq!(nonce_account.get_satomis(), from_expect_satomis);
        assert_eq!(to_account.get_satomis(), to_expect_satomis);
    }

    #[test]
    fn withdraw_inx_initialized_acc_nonce_too_early_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let to_account = instruction_context
            .try_borrow_instruction_account(transaction_context, WITHDRAW_TO_ACCOUNT_INDEX)
            .unwrap();
        set_invoke_context_blockhash!(invoke_context, 0);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        let withdraw_satomis = nonce_account.get_satomis();
        drop(nonce_account);
        drop(to_account);
        let result = withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        );
        assert_eq!(result, Err(SystemError::NonceBlockhashNotExpired.into()));
    }

    #[test]
    fn withdraw_inx_initialized_acc_insuff_funds_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        set_invoke_context_blockhash!(invoke_context, 95);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        set_invoke_context_blockhash!(invoke_context, 63);
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        let withdraw_satomis = nonce_account.get_satomis() + 1;
        drop(nonce_account);
        let result = withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        );
        assert_eq!(result, Err(InstructionError::InsufficientFunds));
    }

    #[test]
    fn withdraw_inx_initialized_acc_insuff_rent_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        set_invoke_context_blockhash!(invoke_context, 95);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        set_invoke_context_blockhash!(invoke_context, 63);
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        let withdraw_satomis = 42 + 1;
        drop(nonce_account);
        let result = withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        );
        assert_eq!(result, Err(InstructionError::InsufficientFunds));
    }

    #[test]
    fn withdraw_inx_overflow() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        set_invoke_context_blockhash!(invoke_context, 95);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        set_invoke_context_blockhash!(invoke_context, 63);
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        let withdraw_satomis = u64::MAX - 54;
        drop(nonce_account);
        let result = withdraw_nonce_account(
            NONCE_ACCOUNT_INDEX,
            withdraw_satomis,
            WITHDRAW_TO_ACCOUNT_INDEX,
            &rent,
            &signers,
            &invoke_context,
            transaction_context,
            instruction_context,
        );
        assert_eq!(result, Err(InstructionError::InsufficientFunds));
    }

    #[test]
    fn initialize_inx_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Uninitialized);
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 0);
        let authorized = *nonce_account.get_key();
        let result =
            initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context);
        let data = nonce::state::Data::new(
            authorized,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        assert_eq!(result, Ok(()));
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Initialized(data));
    }

    #[test]
    fn initialize_inx_initialized_account_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        set_invoke_context_blockhash!(invoke_context, 31);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        set_invoke_context_blockhash!(invoke_context, 0);
        let result =
            initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context);
        assert_eq!(result, Err(InstructionError::InvalidAccountData));
    }

    #[test]
    fn initialize_inx_uninitialized_acc_insuff_funds_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        nonce_account.checked_sub_satomis(42 * 2).unwrap();
        set_invoke_context_blockhash!(invoke_context, 63);
        let authorized = *nonce_account.get_key();
        let result =
            initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context);
        assert_eq!(result, Err(InstructionError::InsufficientFunds));
    }

    #[test]
    fn authorize_inx_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 31);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let authority = Pubkey::default();
        let data = nonce::state::Data::new(
            authority,
            get_durable_nonce(&invoke_context).0,
            invoke_context.satomis_per_signature,
        );
        authorize_nonce_account(&mut nonce_account, &authority, &signers, &invoke_context).unwrap();
        let versions = nonce_account.get_state::<Versions>().unwrap();
        assert_eq!(versions.state(), &State::Initialized(data));
    }

    #[test]
    fn authorize_inx_uninitialized_state_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        let result = authorize_nonce_account(
            &mut nonce_account,
            &Pubkey::default(),
            &signers,
            &invoke_context,
        );
        assert_eq!(result, Err(InstructionError::InvalidAccountData));
    }

    #[test]
    fn authorize_inx_bad_authority_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(*nonce_account.get_key());
        set_invoke_context_blockhash!(invoke_context, 31);
        let authorized = Pubkey::default();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        let result =
            authorize_nonce_account(&mut nonce_account, &authorized, &signers, &invoke_context);
        assert_eq!(result, Err(InstructionError::MissingRequiredSignature));
    }

    #[test]
    fn verify_nonce_ok() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(nonce_account.get_key());
        let versions: Versions = nonce_account.get_state().unwrap();
        // New is in Uninitialzed state
        assert_eq!(versions.state(), &State::Uninitialized);
        set_invoke_context_blockhash!(invoke_context, 0);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(&mut nonce_account, &authorized, &rent, &invoke_context).unwrap();
        drop(nonce_account);
        assert_matches!(
            verify_nonce_account(
                &transaction_context
                    .get_account_at_index(NONCE_ACCOUNT_INDEX)
                    .unwrap()
                    .borrow(),
                get_durable_nonce(&invoke_context).0.as_hash(),
                true, // separate_domins
            ),
            Some(_)
        );
    }

    #[test]
    fn verify_nonce_bad_acc_state_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            _instruction_context,
            instruction_accounts
        );
        assert_eq!(
            verify_nonce_account(
                &transaction_context
                    .get_account_at_index(NONCE_ACCOUNT_INDEX)
                    .unwrap()
                    .borrow(),
                &Hash::default(),
                true, // separate_domins
            ),
            None
        );
    }

    #[test]
    fn verify_nonce_bad_query_hash_fail() {
        prepare_mockup!(invoke_context, instruction_accounts, rent);
        push_instruction_context!(
            invoke_context,
            transaction_context,
            instruction_context,
            instruction_accounts
        );
        let mut nonce_account = instruction_context
            .try_borrow_instruction_account(transaction_context, NONCE_ACCOUNT_INDEX)
            .unwrap();
        let mut signers = HashSet::new();
        signers.insert(nonce_account.get_key());
        let versions: Versions = nonce_account.get_state().unwrap();
        // New is in Uninitialzed state
        assert_eq!(versions.state(), &State::Uninitialized);
        set_invoke_context_blockhash!(invoke_context, 0);
        let authorized = *nonce_account.get_key();
        initialize_nonce_account(
            &mut nonce_account,
            &authorized,
            &Rent::free(),
            &invoke_context,
        )
        .unwrap();
        set_invoke_context_blockhash!(invoke_context, 1);
        drop(nonce_account);
        assert_eq!(
            verify_nonce_account(
                &transaction_context
                    .get_account_at_index(NONCE_ACCOUNT_INDEX)
                    .unwrap()
                    .borrow(),
                get_durable_nonce(&invoke_context).0.as_hash(),
                true, // separate_domins
            ),
            None
        );
    }
}
