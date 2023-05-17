#[allow(deprecated)]
use domichain_sdk::keyed_account::{create_keyed_accounts_unified, KeyedAccount};
use {
    crate::{
        accounts_data_meter::AccountsDataMeter,
        compute_budget::ComputeBudget,
        ic_logger_msg, ic_msg,
        log_collector::LogCollector,
        pre_account::PreAccount,
        stable_log,
        sysvar_cache::SysvarCache,
        timings::{ExecuteDetailsTimings, ExecuteTimings},
    },
    domichain_measure::measure::Measure,
    domichain_sdk::{
        account::{AccountSharedData, ReadableAccount},
        wasm_loader_upgradeable,
        feature_set::{
            cap_accounts_data_len, record_instruction_in_transaction_context_push,
            tx_wide_compute_cap, FeatureSet,
        },
        hash::Hash,
        instruction::{AccountMeta, Instruction, InstructionError},
        native_loader,
        pubkey::Pubkey,
        rent::Rent,
        saturating_add_assign,
        transaction_context::{
            InstructionAccount, InstructionContext, TransactionAccount, TransactionContext,
        },
    },
    std::{
        alloc::Layout,
        borrow::Cow,
        cell::RefCell,
        collections::HashMap,
        fmt::{self, Debug},
        rc::Rc,
        sync::Arc,
    },
};

pub type ProcessInstructionWithContext =
    fn(usize, &mut InvokeContext) -> Result<(), InstructionError>;

#[derive(Clone)]
pub struct BuiltinProgram {
    pub program_id: Pubkey,
    pub process_instruction: ProcessInstructionWithContext,
}

impl std::fmt::Debug for BuiltinProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // These are just type aliases for work around of Debug-ing above pointers
        type ErasedProcessInstructionWithContext =
            fn(usize, &'static mut InvokeContext<'static>) -> Result<(), InstructionError>;

        // rustc doesn't compile due to bug without this work around
        // https://github.com/rust-lang/rust/issues/50280
        // https://users.rust-lang.org/t/display-function-pointer/17073/2
        let erased_instruction: ErasedProcessInstructionWithContext = self.process_instruction;
        write!(f, "{}: {:p}", self.program_id, erased_instruction)
    }
}

/// Program executor
pub trait Executor: Debug + Send + Sync {
    /// Execute the program
    fn execute(
        &self,
        first_instruction_account: usize,
        invoke_context: &mut InvokeContext,
    ) -> Result<(), InstructionError>;
}

pub type Executors = HashMap<Pubkey, TransactionExecutor>;

#[repr(u8)]
#[derive(PartialEq, Debug)]
enum TransactionExecutorStatus {
    /// Executor was already in the cache, no update needed
    Cached,
    /// Executor was missing from the cache, but not updated
    Missing,
    /// Executor is for an updated program
    Updated,
}

/// Tracks whether a given executor is "dirty" and needs to updated in the
/// executors cache
#[derive(Debug)]
pub struct TransactionExecutor {
    executor: Arc<dyn Executor>,
    status: TransactionExecutorStatus,
}

impl TransactionExecutor {
    /// Wraps an executor and tracks that it doesn't need to be updated in the
    /// executors cache.
    pub fn new_cached(executor: Arc<dyn Executor>) -> Self {
        Self {
            executor,
            status: TransactionExecutorStatus::Cached,
        }
    }

    /// Wraps an executor and tracks that it needs to be updated in the
    /// executors cache.
    pub fn new_miss(executor: Arc<dyn Executor>) -> Self {
        Self {
            executor,
            status: TransactionExecutorStatus::Missing,
        }
    }

    /// Wraps an executor and tracks that it needs to be updated in the
    /// executors cache only if the transaction succeeded.
    pub fn new_updated(executor: Arc<dyn Executor>) -> Self {
        Self {
            executor,
            status: TransactionExecutorStatus::Updated,
        }
    }

    pub fn is_missing(&self) -> bool {
        self.status == TransactionExecutorStatus::Missing
    }

    pub fn is_updated(&self) -> bool {
        self.status == TransactionExecutorStatus::Updated
    }

    pub fn get(&self) -> Arc<dyn Executor> {
        self.executor.clone()
    }
}

/// Compute meter
pub struct ComputeMeter {
    remaining: u64,
}
impl ComputeMeter {
    /// Consume compute units
    pub fn consume(&mut self, amount: u64) -> Result<(), InstructionError> {
        let exceeded = self.remaining < amount;
        self.remaining = self.remaining.saturating_sub(amount);
        if exceeded {
            return Err(InstructionError::ComputationalBudgetExceeded);
        }
        Ok(())
    }
    /// Get the number of remaining compute units
    pub fn get_remaining(&self) -> u64 {
        self.remaining
    }
    /// Set compute units
    ///
    /// Only use for tests and benchmarks
    pub fn mock_set_remaining(&mut self, remaining: u64) {
        self.remaining = remaining;
    }
    /// Construct a new one with the given remaining units
    pub fn new_ref(remaining: u64) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { remaining }))
    }
}

/// Based loosely on the unstable std::alloc::Alloc trait
pub trait Alloc {
    fn alloc(&mut self, layout: Layout) -> Result<u64, AllocErr>;
    fn dealloc(&mut self, addr: u64, layout: Layout);
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AllocErr;

impl fmt::Display for AllocErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Error: Memory allocation failed")
    }
}

#[deprecated(
    since = "1.11.0",
    note = "Please use InstructionContext instead of StackFrame"
)]
#[allow(deprecated)]
pub struct StackFrame<'a> {
    pub number_of_program_accounts: usize,
    pub keyed_accounts: Vec<KeyedAccount<'a>>,
    pub keyed_accounts_range: std::ops::Range<usize>,
}

#[allow(deprecated)]
impl<'a> StackFrame<'a> {
    pub fn new(number_of_program_accounts: usize, keyed_accounts: Vec<KeyedAccount<'a>>) -> Self {
        let keyed_accounts_range = std::ops::Range {
            start: 0,
            end: keyed_accounts.len(),
        };
        Self {
            number_of_program_accounts,
            keyed_accounts,
            keyed_accounts_range,
        }
    }

    pub fn program_id(&self) -> Option<&Pubkey> {
        self.keyed_accounts
            .get(self.number_of_program_accounts.saturating_sub(1))
            .map(|keyed_account| keyed_account.unsigned_key())
    }
}

struct SyscallContext {
    check_aligned: bool,
    check_size: bool,
    orig_account_lengths: Vec<usize>,
    allocator: Rc<RefCell<dyn Alloc>>,
}

pub struct InvokeContext<'a> {
    pub transaction_context: &'a mut TransactionContext,
    #[allow(deprecated)]
    invoke_stack: Vec<StackFrame<'a>>,
    rent: Rent,
    pre_accounts: Vec<PreAccount>,
    builtin_programs: &'a [BuiltinProgram],
    pub sysvar_cache: Cow<'a, SysvarCache>,
    log_collector: Option<Rc<RefCell<LogCollector>>>,
    compute_budget: ComputeBudget,
    current_compute_budget: ComputeBudget,
    compute_meter: Rc<RefCell<ComputeMeter>>,
    accounts_data_meter: AccountsDataMeter,
    executors: Rc<RefCell<Executors>>,
    pub feature_set: Arc<FeatureSet>,
    pub timings: ExecuteDetailsTimings,
    pub blockhash: Hash,
    pub lamports_per_signature: u64,
    syscall_context: Vec<Option<SyscallContext>>,
}

impl<'a> InvokeContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transaction_context: &'a mut TransactionContext,
        rent: Rent,
        builtin_programs: &'a [BuiltinProgram],
        sysvar_cache: Cow<'a, SysvarCache>,
        log_collector: Option<Rc<RefCell<LogCollector>>>,
        compute_budget: ComputeBudget,
        executors: Rc<RefCell<Executors>>,
        feature_set: Arc<FeatureSet>,
        blockhash: Hash,
        lamports_per_signature: u64,
        initial_accounts_data_len: u64,
    ) -> Self {
        Self {
            transaction_context,
            invoke_stack: Vec::with_capacity(compute_budget.max_invoke_depth),
            rent,
            pre_accounts: Vec::new(),
            builtin_programs,
            sysvar_cache,
            log_collector,
            current_compute_budget: compute_budget,
            compute_budget,
            compute_meter: ComputeMeter::new_ref(compute_budget.compute_unit_limit),
            accounts_data_meter: AccountsDataMeter::new(initial_accounts_data_len),
            executors,
            feature_set,
            timings: ExecuteDetailsTimings::default(),
            blockhash,
            lamports_per_signature,
            syscall_context: Vec::new(),
        }
    }

    pub fn new_mock(
        transaction_context: &'a mut TransactionContext,
        builtin_programs: &'a [BuiltinProgram],
    ) -> Self {
        let mut sysvar_cache = SysvarCache::default();
        sysvar_cache.fill_missing_entries(|pubkey| {
            (0..transaction_context.get_number_of_accounts()).find_map(|index| {
                if transaction_context
                    .get_key_of_account_at_index(index)
                    .unwrap()
                    == pubkey
                {
                    Some(
                        transaction_context
                            .get_account_at_index(index)
                            .unwrap()
                            .borrow()
                            .clone(),
                    )
                } else {
                    None
                }
            })
        });
        Self::new(
            transaction_context,
            Rent::default(),
            builtin_programs,
            Cow::Owned(sysvar_cache),
            Some(LogCollector::new_ref()),
            ComputeBudget::default(),
            Rc::new(RefCell::new(Executors::default())),
            Arc::new(FeatureSet::all_enabled()),
            Hash::default(),
            0,
            0,
        )
    }

    /// Push a stack frame onto the invocation stack
    pub fn push(
        &mut self,
        instruction_accounts: &[InstructionAccount],
        program_indices: &[usize],
        instruction_data: &[u8],
    ) -> Result<(), InstructionError> {
        if !self
            .feature_set
            .is_active(&record_instruction_in_transaction_context_push::id())
            && self
                .transaction_context
                .get_instruction_context_stack_height()
                > self.compute_budget.max_invoke_depth
        {
            return Err(InstructionError::CallDepth);
        }

        let program_id = self.transaction_context.get_key_of_account_at_index(
            *program_indices
                .last()
                .ok_or(InstructionError::UnsupportedProgramId)?,
        )?;
        if self
            .transaction_context
            .get_instruction_context_stack_height()
            == 0
        {
            self.current_compute_budget = self.compute_budget;

            if !self.feature_set.is_active(&tx_wide_compute_cap::id()) {
                self.compute_meter =
                    ComputeMeter::new_ref(self.current_compute_budget.compute_unit_limit);
            }

            self.pre_accounts = Vec::with_capacity(instruction_accounts.len());

            for (instruction_account_index, instruction_account) in
                instruction_accounts.iter().enumerate()
            {
                if instruction_account_index != instruction_account.index_in_callee {
                    continue; // Skip duplicate account
                }
                if instruction_account.index_in_transaction
                    >= self.transaction_context.get_number_of_accounts()
                {
                    return Err(InstructionError::MissingAccount);
                }
                let account = self
                    .transaction_context
                    .get_account_at_index(instruction_account.index_in_transaction)?
                    .borrow()
                    .clone();
                self.pre_accounts.push(PreAccount::new(
                    self.transaction_context
                        .get_key_of_account_at_index(instruction_account.index_in_transaction)?,
                    account,
                ));
            }
        } else {
            let contains = (0..self
                .transaction_context
                .get_instruction_context_stack_height())
                .any(|level| {
                    self.transaction_context
                        .get_instruction_context_at(level)
                        .and_then(|instruction_context| {
                            instruction_context
                                .try_borrow_last_program_account(self.transaction_context)
                        })
                        .map(|program_account| program_account.get_key() == program_id)
                        .unwrap_or(false)
                });
            let is_last = self
                .transaction_context
                .get_current_instruction_context()
                .and_then(|instruction_context| {
                    instruction_context.try_borrow_last_program_account(self.transaction_context)
                })
                .map(|program_account| program_account.get_key() == program_id)
                .unwrap_or(false);
            if contains && !is_last {
                // Reentrancy not allowed unless caller is calling itself
                return Err(InstructionError::ReentrancyNotAllowed);
            }
        }

        // Create the KeyedAccounts that will be passed to the program
        #[allow(deprecated)]
        let keyed_accounts = program_indices
            .iter()
            .map(|account_index| {
                Ok((
                    false,
                    false,
                    self.transaction_context
                        .get_key_of_account_at_index(*account_index)?,
                    self.transaction_context
                        .get_account_at_index(*account_index)?,
                ))
            })
            .chain(instruction_accounts.iter().map(|instruction_account| {
                Ok((
                    instruction_account.is_signer,
                    instruction_account.is_writable,
                    self.transaction_context
                        .get_key_of_account_at_index(instruction_account.index_in_transaction)?,
                    self.transaction_context
                        .get_account_at_index(instruction_account.index_in_transaction)?,
                ))
            }))
            .collect::<Result<Vec<_>, InstructionError>>()?;

        // Unsafe will be removed together with the keyed_accounts
        #[allow(deprecated)]
        self.invoke_stack.push(StackFrame::new(
            program_indices.len(),
            create_keyed_accounts_unified(unsafe {
                std::mem::transmute(keyed_accounts.as_slice())
            }),
        ));
        self.syscall_context.push(None);
        self.transaction_context.push(
            program_indices,
            instruction_accounts,
            instruction_data,
            self.feature_set
                .is_active(&record_instruction_in_transaction_context_push::id()),
        )
    }

    /// Pop a stack frame from the invocation stack
    pub fn pop(&mut self) -> Result<(), InstructionError> {
        self.syscall_context.pop();
        self.invoke_stack.pop();
        self.transaction_context.pop()
    }

    /// Current height of the invocation stack, top level instructions are height
    /// `domichain_sdk::instruction::TRANSACTION_LEVEL_STACK_HEIGHT`
    pub fn get_stack_height(&self) -> usize {
        self.transaction_context
            .get_instruction_context_stack_height()
    }

    /// Verify the results of an instruction
    ///
    /// Note: `instruction_accounts` must be the same as passed to `InvokeContext::push()`,
    /// so that they match the order of `pre_accounts`.
    fn verify(
        &mut self,
        instruction_accounts: &[InstructionAccount],
        program_indices: &[usize],
    ) -> Result<(), InstructionError> {
        let cap_accounts_data_len = self.feature_set.is_active(&cap_accounts_data_len::id());
        let instruction_context = self
            .transaction_context
            .get_current_instruction_context()
            .map_err(|_| InstructionError::CallDepth)?;
        let program_id = instruction_context
            .get_last_program_key(self.transaction_context)
            .map_err(|_| InstructionError::CallDepth)?;

        // Verify all executable accounts have zero outstanding refs
        for account_index in program_indices.iter() {
            self.transaction_context
                .get_account_at_index(*account_index)
                ?
                .try_borrow_mut()
                .map_err(|_| InstructionError::AccountBorrowOutstanding)?;
        }

        // Verify the per-account instruction results
        let (mut pre_sum, mut post_sum) = (0_u128, 0_u128);
        let mut pre_account_index = 0;
        for (instruction_account_index, instruction_account) in
            instruction_accounts.iter().enumerate()
        {
            if instruction_account_index != instruction_account.index_in_callee {
                continue; // Skip duplicate account
            }
            {
                // Verify account has no outstanding references
                let _ = self
                    .transaction_context
                    .get_account_at_index(instruction_account.index_in_transaction)?
                    .try_borrow_mut()
                    .map_err(|_| InstructionError::AccountBorrowOutstanding)?;
            }
            let pre_account = &self
                .pre_accounts
                .get(pre_account_index)
                .ok_or(InstructionError::NotEnoughAccountKeys)?;
            pre_account_index = pre_account_index.saturating_add(1);
            let account = self
                .transaction_context
                .get_account_at_index(instruction_account.index_in_transaction)?
                .borrow();
            pre_account
                .verify(
                    program_id,
                    instruction_account.is_writable,
                    &self.rent,
                    &account,
                    &mut self.timings,
                    true,
                )
                .map_err(|err| {
                    ic_logger_msg!(
                        self.log_collector,
                        "failed to verify account {}: {}",
                        pre_account.key(),
                        err
                    );
                    err
                })?;
            pre_sum = pre_sum
                .checked_add(u128::from(pre_account.lamports()))
                .ok_or(InstructionError::UnbalancedInstruction)?;
            post_sum = post_sum
                .checked_add(u128::from(account.lamports()))
                .ok_or(InstructionError::UnbalancedInstruction)?;

            let pre_data_len = pre_account.data().len() as i64;
            let post_data_len = account.data().len() as i64;
            let data_len_delta = post_data_len.saturating_sub(pre_data_len);
            if cap_accounts_data_len {
                self.accounts_data_meter.adjust_delta(data_len_delta)?;
            } else {
                self.accounts_data_meter
                    .adjust_delta_unchecked(data_len_delta);
            }
        }

        // Verify that the total sum of all the lamports did not change
        if pre_sum != post_sum {
            return Err(InstructionError::UnbalancedInstruction);
        }
        Ok(())
    }

    /// Verify and update PreAccount state based on program execution
    ///
    /// Note: `instruction_accounts` must be the same as passed to `InvokeContext::push()`,
    /// so that they match the order of `pre_accounts`.
    fn verify_and_update(
        &mut self,
        instruction_accounts: &[InstructionAccount],
        before_instruction_context_push: bool,
    ) -> Result<(), InstructionError> {
        let cap_accounts_data_len = self.feature_set.is_active(&cap_accounts_data_len::id());
        let transaction_context = &self.transaction_context;
        let instruction_context = transaction_context.get_current_instruction_context()?;
        let program_id = instruction_context
            .get_last_program_key(transaction_context)
            .map_err(|_| InstructionError::CallDepth)?;

        // Verify the per-account instruction results
        let (mut pre_sum, mut post_sum) = (0_u128, 0_u128);
        for (instruction_account_index, instruction_account) in
            instruction_accounts.iter().enumerate()
        {
            if instruction_account_index != instruction_account.index_in_callee {
                continue; // Skip duplicate account
            }
            if instruction_account.index_in_transaction
                < transaction_context.get_number_of_accounts()
            {
                let key = transaction_context
                    .get_key_of_account_at_index(instruction_account.index_in_transaction)?;
                let account = transaction_context
                    .get_account_at_index(instruction_account.index_in_transaction)?;
                let is_writable = if before_instruction_context_push {
                    instruction_context
                        .is_instruction_account_writable(instruction_account.index_in_caller)?
                } else {
                    instruction_account.is_writable
                };
                // Find the matching PreAccount
                for pre_account in self.pre_accounts.iter_mut() {
                    if key == pre_account.key() {
                        {
                            // Verify account has no outstanding references
                            let _ = account
                                .try_borrow_mut()
                                .map_err(|_| InstructionError::AccountBorrowOutstanding)?;
                        }
                        let account = account.borrow();
                        pre_account
                            .verify(
                                program_id,
                                is_writable,
                                &self.rent,
                                &account,
                                &mut self.timings,
                                false,
                            )
                            .map_err(|err| {
                                ic_logger_msg!(
                                    self.log_collector,
                                    "failed to verify account {}: {}",
                                    key,
                                    err
                                );
                                err
                            })?;
                        pre_sum = pre_sum
                            .checked_add(u128::from(pre_account.lamports()))
                            .ok_or(InstructionError::UnbalancedInstruction)?;
                        post_sum = post_sum
                            .checked_add(u128::from(account.lamports()))
                            .ok_or(InstructionError::UnbalancedInstruction)?;
                        if is_writable && !pre_account.executable() {
                            pre_account.update(account.clone());
                        }

                        let pre_data_len = pre_account.data().len() as i64;
                        let post_data_len = account.data().len() as i64;
                        let data_len_delta = post_data_len.saturating_sub(pre_data_len);
                        if cap_accounts_data_len {
                            self.accounts_data_meter.adjust_delta(data_len_delta)?;
                        } else {
                            self.accounts_data_meter
                                .adjust_delta_unchecked(data_len_delta);
                        }

                        break;
                    }
                }
            }
        }

        // Verify that the total sum of all the lamports did not change
        if pre_sum != post_sum {
            return Err(InstructionError::UnbalancedInstruction);
        }
        Ok(())
    }

    /// Entrypoint for a cross-program invocation from a builtin program
    pub fn native_invoke(
        &mut self,
        instruction: Instruction,
        signers: &[Pubkey],
    ) -> Result<(), InstructionError> {
        let (instruction_accounts, program_indices) =
            self.prepare_instruction(&instruction, signers)?;
        let mut prev_account_sizes = Vec::with_capacity(instruction_accounts.len());
        for instruction_account in instruction_accounts.iter() {
            let account_length = self
                .transaction_context
                .get_account_at_index(instruction_account.index_in_transaction)?
                .borrow()
                .data()
                .len();
            prev_account_sizes.push((instruction_account.index_in_transaction, account_length));
        }

        let mut compute_units_consumed = 0;
        self.process_instruction(
            &instruction.data,
            &instruction_accounts,
            &program_indices,
            &mut compute_units_consumed,
            &mut ExecuteTimings::default(),
        )?;

        Ok(())
    }

    /// Helper to prepare for process_instruction()
    #[allow(clippy::type_complexity)]
    pub fn prepare_instruction(
        &mut self,
        instruction: &Instruction,
        signers: &[Pubkey],
    ) -> Result<(Vec<InstructionAccount>, Vec<usize>), InstructionError> {
        // Finds the index of each account in the instruction by its pubkey.
        // Then normalizes / unifies the privileges of duplicate accounts.
        // Note: This is an O(n^2) algorithm,
        // but performed on a very small slice and requires no heap allocations.
        let instruction_context = self.transaction_context.get_current_instruction_context()?;
        let mut deduplicated_instruction_accounts: Vec<InstructionAccount> = Vec::new();
        let mut duplicate_indicies = Vec::with_capacity(instruction.accounts.len());
        for (instruction_account_index, account_meta) in instruction.accounts.iter().enumerate() {
            let index_in_transaction = self
                .transaction_context
                .find_index_of_account(&account_meta.pubkey)
                .ok_or_else(|| {
                    ic_msg!(
                        self,
                        "Instruction references an unknown account {}",
                        account_meta.pubkey,
                    );
                    InstructionError::MissingAccount
                })?;
            if let Some(duplicate_index) =
                deduplicated_instruction_accounts
                    .iter()
                    .position(|instruction_account| {
                        instruction_account.index_in_transaction == index_in_transaction
                    })
            {
                duplicate_indicies.push(duplicate_index);
                let instruction_account = deduplicated_instruction_accounts
                    .get_mut(duplicate_index)
                    .ok_or(InstructionError::NotEnoughAccountKeys)?;
                instruction_account.is_signer |= account_meta.is_signer;
                instruction_account.is_writable |= account_meta.is_writable;
            } else {
                let index_in_caller = instruction_context
                    .find_index_of_instruction_account(
                        self.transaction_context,
                        &account_meta.pubkey,
                    )
                    .ok_or_else(|| {
                        ic_msg!(
                            self,
                            "Instruction references an unknown account {}",
                            account_meta.pubkey,
                        );
                        InstructionError::MissingAccount
                    })?;
                duplicate_indicies.push(deduplicated_instruction_accounts.len());
                deduplicated_instruction_accounts.push(InstructionAccount {
                    index_in_transaction,
                    index_in_caller,
                    index_in_callee: instruction_account_index,
                    is_signer: account_meta.is_signer,
                    is_writable: account_meta.is_writable,
                });
            }
        }
        for instruction_account in deduplicated_instruction_accounts.iter() {
            let borrowed_account = instruction_context.try_borrow_instruction_account(
                self.transaction_context,
                instruction_account.index_in_caller,
            )?;

            // Readonly in caller cannot become writable in callee
            if instruction_account.is_writable && !borrowed_account.is_writable() {
                ic_msg!(
                    self,
                    "{}'s writable privilege escalated",
                    borrowed_account.get_key(),
                );
                return Err(InstructionError::PrivilegeEscalation);
            }

            // To be signed in the callee,
            // it must be either signed in the caller or by the program
            if instruction_account.is_signer
                && !(borrowed_account.is_signer() || signers.contains(borrowed_account.get_key()))
            {
                ic_msg!(
                    self,
                    "{}'s signer privilege escalated",
                    borrowed_account.get_key()
                );
                return Err(InstructionError::PrivilegeEscalation);
            }
        }
        let instruction_accounts = duplicate_indicies
            .into_iter()
            .map(|duplicate_index| {
                Ok(deduplicated_instruction_accounts
                    .get(duplicate_index)
                    .ok_or(InstructionError::NotEnoughAccountKeys)?
                    .clone())
            })
            .collect::<Result<Vec<InstructionAccount>, InstructionError>>()?;

        // Find and validate executables / program accounts
        let callee_program_id = instruction.program_id;
        let program_account_index = instruction_context
            .find_index_of_instruction_account(self.transaction_context, &callee_program_id)
            .ok_or_else(|| {
                ic_msg!(self, "Unknown program {}", callee_program_id);
                InstructionError::MissingAccount
            })?;
        let borrowed_program_account = instruction_context
            .try_borrow_instruction_account(self.transaction_context, program_account_index)?;
        if !borrowed_program_account.is_executable() {
            ic_msg!(self, "Account {} is not executable", callee_program_id);
            return Err(InstructionError::AccountNotExecutable);
        }
        let mut program_indices = vec![];
        if borrowed_program_account.get_owner() == &wasm_loader_upgradeable::id() {
            if let wasm_loader_upgradeable::UpgradeableLoaderState::Program {
                programdata_address,
            } = borrowed_program_account.get_state()?
            {
                if let Some(programdata_account_index) = self
                    .transaction_context
                    .find_index_of_program_account(&programdata_address)
                {
                    program_indices.push(programdata_account_index);
                } else {
                    ic_msg!(
                        self,
                        "Unknown upgradeable programdata account {}",
                        programdata_address,
                    );
                    return Err(InstructionError::MissingAccount);
                }
            } else {
                ic_msg!(
                    self,
                    "Invalid upgradeable program account {}",
                    callee_program_id,
                );
                return Err(InstructionError::MissingAccount);
            }
        }
        program_indices.push(borrowed_program_account.get_index_in_transaction());

        Ok((instruction_accounts, program_indices))
    }

    /// Processes an instruction and returns how many compute units were used
    pub fn process_instruction(
        &mut self,
        instruction_data: &[u8],
        instruction_accounts: &[InstructionAccount],
        program_indices: &[usize],
        compute_units_consumed: &mut u64,
        timings: &mut ExecuteTimings,
    ) -> Result<(), InstructionError> {
        *compute_units_consumed = 0;
        let program_id = program_indices
            .last()
            .map(|index| {
                self.transaction_context
                    .get_key_of_account_at_index(*index)
                    .map(|pubkey| *pubkey)
            })
            .unwrap_or_else(|| Ok(native_loader::id()))
            ?;

        let nesting_level = self
            .transaction_context
            .get_instruction_context_stack_height();
        let is_top_level_instruction = nesting_level == 0;
        if !is_top_level_instruction {
            // Verify the calling program hasn't misbehaved
            let mut verify_caller_time = Measure::start("verify_caller_time");
            let verify_caller_result = self.verify_and_update(instruction_accounts, true);
            verify_caller_time.stop();
            saturating_add_assign!(
                timings
                    .execute_accessories
                    .process_instructions
                    .verify_caller_us,
                verify_caller_time.as_us()
            );
            verify_caller_result
                ?;

            if !self
                .feature_set
                .is_active(&record_instruction_in_transaction_context_push::id())
            {
                self.transaction_context
                    .record_instruction(InstructionContext::new(
                        nesting_level,
                        program_indices,
                        instruction_accounts,
                        instruction_data,
                    ));
            }
        }

        let result = self
            .push(instruction_accounts, program_indices, instruction_data)

            .and_then(|_| {
                let mut process_executable_chain_time =
                    Measure::start("process_executable_chain_time");
                self.transaction_context
                    .set_return_data(program_id, Vec::new())
                    ?;
                let pre_remaining_units = self.compute_meter.borrow().get_remaining();
                let execution_result = self
                    .process_executable_chain()
                    ;
                let post_remaining_units = self.compute_meter.borrow().get_remaining();
                *compute_units_consumed = pre_remaining_units.saturating_sub(post_remaining_units);
                process_executable_chain_time.stop();

                // Verify the called program has not misbehaved
                let mut verify_callee_time = Measure::start("verify_callee_time");
                let result = execution_result.and_then(|_| {
                    if is_top_level_instruction {
                        self
                            .verify(instruction_accounts, program_indices)

                    } else {
                        self
                            .verify_and_update(instruction_accounts, false)

                    }
                });
                verify_callee_time.stop();

                saturating_add_assign!(
                    timings
                        .execute_accessories
                        .process_instructions
                        .process_executable_chain_us,
                    process_executable_chain_time.as_us()
                );
                saturating_add_assign!(
                    timings
                        .execute_accessories
                        .process_instructions
                        .verify_callee_us,
                    verify_callee_time.as_us()
                );

                result
            })
            ;

        // Pop the invoke_stack to restore previous state
        let _ = self.pop();
        result
    }

    /// Calls the instruction's program entrypoint method
    fn process_executable_chain(&mut self) -> Result<(), InstructionError> {
        let instruction_context = self.transaction_context
            .get_current_instruction_context()
            ?;

        let (first_instruction_account, builtin_id) = {
            let borrowed_root_account = instruction_context
                .try_borrow_program_account(self.transaction_context, 0)
                .map_err(|_| InstructionError::UnsupportedProgramId)
                ?;
            let owner_id = borrowed_root_account.get_owner();
            if domichain_sdk::native_loader::check_id(owner_id) {
                (1, *borrowed_root_account.get_key())
            } else {
                (0, *owner_id)
            }
        };

        for entry in self.builtin_programs {
            if entry.program_id == builtin_id {
                let program_id =
                    *instruction_context.get_last_program_key(self.transaction_context)
                        ?;
                if builtin_id == program_id {
                    let logger = self.get_log_collector();
                    stable_log::program_invoke(&logger, &program_id, self.get_stack_height());

                    return (entry.process_instruction)(first_instruction_account, self)
                        .map(|()| {
                            stable_log::program_success(&logger, &program_id);
                        })
                        .map_err(|err| {
                            stable_log::program_failure(&logger, &program_id, &err);
                            err
                        });
                } else {
                    return (entry.process_instruction)(first_instruction_account, self);
                }
            }
        }

        Err(InstructionError::UnsupportedProgramId)
    }

    #[deprecated(
        since = "1.11.0",
        note = "Please use BorrowedAccount instead of KeyedAccount"
    )]
    #[allow(deprecated)]
    /// Get the list of keyed accounts including the chain of program accounts
    pub fn get_keyed_accounts(&self) -> Result<&[KeyedAccount], InstructionError> {
        self.invoke_stack
            .last()
            .and_then(|frame| frame.keyed_accounts.get(frame.keyed_accounts_range.clone()))
            .ok_or(InstructionError::CallDepth)
    }

    /// Get this invocation's LogCollector
    pub fn get_log_collector(&self) -> Option<Rc<RefCell<LogCollector>>> {
        self.log_collector.clone()
    }

    /// Get this invocation's ComputeMeter
    pub fn get_compute_meter(&self) -> Rc<RefCell<ComputeMeter>> {
        self.compute_meter.clone()
    }

    /// Get this invocation's AccountsDataMeter
    pub fn get_accounts_data_meter(&self) -> &AccountsDataMeter {
        &self.accounts_data_meter
    }

    /// Cache an executor that wasn't found in the cache
    pub fn add_executor(&self, pubkey: &Pubkey, executor: Arc<dyn Executor>) {
        self.executors
            .borrow_mut()
            .insert(*pubkey, TransactionExecutor::new_miss(executor));
    }

    /// Cache an executor that has changed
    pub fn update_executor(&self, pubkey: &Pubkey, executor: Arc<dyn Executor>) {
        self.executors
            .borrow_mut()
            .insert(*pubkey, TransactionExecutor::new_updated(executor));
    }

    /// Get the completed loader work that can be re-used across execution
    pub fn get_executor(&self, pubkey: &Pubkey) -> Option<Arc<dyn Executor>> {
        self.executors
            .borrow()
            .get(pubkey)
            .map(|tx_executor| tx_executor.executor.clone())
    }

    /// Get this invocation's compute budget
    pub fn get_compute_budget(&self) -> &ComputeBudget {
        &self.current_compute_budget
    }

    /// Get cached sysvars
    pub fn get_sysvar_cache(&self) -> &SysvarCache {
        &self.sysvar_cache
    }

    // Get pubkey of account at index
    pub fn get_key_of_account_at_index(
        &self,
        index_in_transaction: usize,
    ) -> Result<&Pubkey, InstructionError> {
        self.transaction_context
            .get_key_of_account_at_index(index_in_transaction)
    }

    // Set this instruction syscall context
    pub fn set_syscall_context(
        &mut self,
        check_aligned: bool,
        check_size: bool,
        orig_account_lengths: Vec<usize>,
        allocator: Rc<RefCell<dyn Alloc>>,
    ) -> Result<(), InstructionError> {
        *self
            .syscall_context
            .last_mut()
            .ok_or(InstructionError::CallDepth)? = Some(SyscallContext {
            check_aligned,
            check_size,
            orig_account_lengths,
            allocator,
        });
        Ok(())
    }

    // Should alignment be enforced during user pointer translation
    pub fn get_check_aligned(&self) -> bool {
        self.syscall_context
            .last()
            .and_then(|context| context.as_ref())
            .map(|context| context.check_aligned)
            .unwrap_or(true)
    }

    // Set should type size be checked during user pointer translation
    pub fn get_check_size(&self) -> bool {
        self.syscall_context
            .last()
            .and_then(|context| context.as_ref())
            .map(|context| context.check_size)
            .unwrap_or(true)
    }

    /// Get the original account lengths
    pub fn get_orig_account_lengths(&self) -> Result<&[usize], InstructionError> {
        self.syscall_context
            .last()
            .and_then(|context| context.as_ref())
            .map(|context| context.orig_account_lengths.as_slice())
            .ok_or(InstructionError::CallDepth)
    }

    // Get this instruction's memory allocator
    pub fn get_allocator(&self) -> Result<Rc<RefCell<dyn Alloc>>, InstructionError> {
        self.syscall_context
            .last()
            .and_then(|context| context.as_ref())
            .map(|context| context.allocator.clone())
            .ok_or(InstructionError::CallDepth)
    }
}

pub struct MockInvokeContextPreparation {
    pub transaction_accounts: Vec<TransactionAccount>,
    pub instruction_accounts: Vec<InstructionAccount>,
}

pub fn prepare_mock_invoke_context(
    transaction_accounts: Vec<TransactionAccount>,
    instruction_account_metas: Vec<AccountMeta>,
    _program_indices: &[usize],
) -> MockInvokeContextPreparation {
    let mut instruction_accounts: Vec<InstructionAccount> =
        Vec::with_capacity(instruction_account_metas.len());
    for (instruction_account_index, account_meta) in instruction_account_metas.iter().enumerate() {
        let index_in_transaction = transaction_accounts
            .iter()
            .position(|(key, _account)| *key == account_meta.pubkey)
            .unwrap_or(transaction_accounts.len());
        let index_in_callee = instruction_accounts
            .get(0..instruction_account_index)
            .unwrap()
            .iter()
            .position(|instruction_account| {
                instruction_account.index_in_transaction == index_in_transaction
            })
            .unwrap_or(instruction_account_index);
        instruction_accounts.push(InstructionAccount {
            index_in_transaction,
            index_in_caller: index_in_transaction,
            index_in_callee,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        });
    }
    MockInvokeContextPreparation {
        transaction_accounts,
        instruction_accounts,
    }
}

pub fn with_mock_invoke_context<R, F: FnMut(&mut InvokeContext) -> R>(
    loader_id: Pubkey,
    account_size: usize,
    mut callback: F,
) -> R {
    let program_indices = vec![0, 1];
    let transaction_accounts = vec![
        (
            loader_id,
            AccountSharedData::new(0, 0, &domichain_sdk::native_loader::id()),
        ),
        (
            Pubkey::new_unique(),
            AccountSharedData::new(1, 0, &loader_id),
        ),
        (
            Pubkey::new_unique(),
            AccountSharedData::new(2, account_size, &Pubkey::new_unique()),
        ),
    ];
    let instruction_accounts = vec![AccountMeta {
        pubkey: transaction_accounts.get(2).unwrap().0,
        is_signer: false,
        is_writable: false,
    }];
    let preparation =
        prepare_mock_invoke_context(transaction_accounts, instruction_accounts, &program_indices);
    let mut transaction_context = TransactionContext::new(
        preparation.transaction_accounts,
        ComputeBudget::default().max_invoke_depth.saturating_add(1),
        1,
    );
    let mut invoke_context = InvokeContext::new_mock(&mut transaction_context, &[]);
    invoke_context
        .push(&preparation.instruction_accounts, &program_indices, &[])
        .unwrap();
    callback(&mut invoke_context)
}

pub fn mock_process_instruction(
    loader_id: &Pubkey,
    mut program_indices: Vec<usize>,
    instruction_data: &[u8],
    transaction_accounts: Vec<TransactionAccount>,
    instruction_accounts: Vec<AccountMeta>,
    sysvar_cache_override: Option<&SysvarCache>,
    feature_set_override: Option<Arc<FeatureSet>>,
    expected_result: Result<(), InstructionError>,
    process_instruction: ProcessInstructionWithContext,
) -> Vec<AccountSharedData> {
    program_indices.insert(0, transaction_accounts.len());
    let mut preparation =
        prepare_mock_invoke_context(transaction_accounts, instruction_accounts, &program_indices);
    let processor_account = AccountSharedData::new(0, 0, &domichain_sdk::native_loader::id());
    preparation
        .transaction_accounts
        .push((*loader_id, processor_account));
    let mut transaction_context = TransactionContext::new(
        preparation.transaction_accounts,
        ComputeBudget::default().max_invoke_depth.saturating_add(1),
        1,
    );
    let mut invoke_context = InvokeContext::new_mock(&mut transaction_context, &[]);
    if let Some(sysvar_cache) = sysvar_cache_override {
        invoke_context.sysvar_cache = Cow::Borrowed(sysvar_cache);
    }
    if let Some(feature_set) = feature_set_override {
        invoke_context.feature_set = feature_set;
    }
    let result = invoke_context
        .push(
            &preparation.instruction_accounts,
            &program_indices,
            instruction_data,
        )
        .and_then(|_| process_instruction(1, &mut invoke_context));
    invoke_context.pop().unwrap();
    assert_eq!(result, expected_result);
    let mut transaction_accounts = transaction_context.deconstruct_without_keys().unwrap();
    transaction_accounts.pop();
    transaction_accounts
}


