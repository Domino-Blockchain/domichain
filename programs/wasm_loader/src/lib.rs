#![deny(clippy::integer_arithmetic)]
#![deny(clippy::indexing_slicing)]
#![feature(result_option_inspect)]

pub mod allocator_bump;
pub mod deprecated;
pub mod serialization;
pub mod syscalls;
pub mod upgradeable;
pub mod upgradeable_with_jit;
pub mod with_jit;

#[macro_use]
extern crate domichain_metrics;

use log::{log_enabled, Level::Trace};
use solana_rbpf::memory_region::MemoryMapping;
use solana_rbpf::vm::SyscallRegistry;
use wasmi::core::Trap;
use wasmi::{Caller, Extern, Func};
use wasmi_wasi::WasiCtx;
use {
    crate::{
        serialization::{deserialize_parameters, serialize_parameters},
        syscalls::{
            SyscallError,
            SyscallAbort,
            SyscallPanic,
            SyscallLog,
            SyscallLogU64,
            SyscallLogBpfComputeUnits,
            SyscallLogPubkey,
            SyscallCreateProgramAddress,
            SyscallTryFindProgramAddress,
            SyscallSha256,
            SyscallKeccak256,
            SyscallSecp256k1Recover,
            SyscallBlake3,
            SyscallZkTokenElgamalOp,
            SyscallZkTokenElgamalOpWithLoHi,
            SyscallZkTokenElgamalOpWithScalar,
            SyscallCurvePointValidation,
            SyscallCurveGroupOps,
            SyscallGetClockSysvar,
            SyscallGetEpochScheduleSysvar,
            SyscallGetFeesSysvar,
            SyscallGetRentSysvar,
            SyscallMemcpy,
            SyscallMemmove,
            SyscallMemcmp,
            SyscallMemset,
            SyscallInvokeSignedC,
            SyscallInvokeSignedRust,
            SyscallAllocFree,
            SyscallSetReturnData,
            SyscallGetReturnData,
            SyscallLogData,
            SyscallGetProcessedSiblingInstruction,
            SyscallGetStackHeight,
        },
    },
    log::error,
    domichain_measure::measure::Measure,
    domichain_program_runtime::{
        ic_logger_msg, ic_msg,
        invoke_context::{ComputeMeter, Executor, InvokeContext},
        log_collector::LogCollector,
        stable_log,
        sysvar_cache::get_sysvar_with_account_check,
    },
    domichain_sdk::{
        wasm_loader, wasm_loader_deprecated,
        wasm_loader_upgradeable::{self, UpgradeableLoaderState},
        entrypoint::HEAP_LENGTH,
        feature_set::{
            disable_deploy_of_alloc_free_syscall,
            disable_deprecated_loader,
            reduce_required_deploy_balance,
            requestable_heap_size,
            blake3_syscall_enabled,
            check_slice_translation_size,
            disable_bpf_deprecated_load_instructions,
            disable_bpf_unresolved_symbols_at_runtime,
            disable_fees_sysvar,
            error_on_syscall_bpf_function_hash_collisions,
            reject_callx_r10,
            zk_token_sdk_enabled,
        },
        instruction::{AccountMeta, InstructionError},
        loader_instruction::LoaderInstruction,
        loader_upgradeable_instruction::UpgradeableLoaderInstruction,
        program_utils::limited_deserialize,
        pubkey::Pubkey,
        saturating_add_assign,
        system_instruction::{self, MAX_PERMITTED_DATA_LENGTH},
        transaction_context::{BorrowedAccount, InstructionContext, TransactionContext},
    },
    solana_rbpf::{
        aligned_memory::AlignedMemory,
        ebpf::{HOST_ALIGN, MM_PROGRAM_START, MM_HEAP_START, MM_INPUT_START},
        error::{EbpfError, UserDefinedError},
        memory_region::MemoryRegion,
        verifier::{RequisiteVerifier, VerifierError},
        vm::{SyscallObject, EbpfVm, InstructionMeter, VerifiedExecutable},
    },
    std::{cell::RefCell, fmt::Debug, rc::Rc, sync::Arc},
    thiserror::Error,
};

domichain_sdk::declare_builtin!(
    domichain_sdk::wasm_loader::ID,
    domichain_wasm_loader_program,
    domichain_wasm_loader_program::process_instruction
);

/// Errors returned by functions the BPF Loader registers with the VM
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BpfError {
    #[error("{0}")]
    VerifierError(#[from] VerifierError),
    #[error("{0}")]
    SyscallError(#[from] SyscallError),
}
impl UserDefinedError for BpfError {}

fn map_ebpf_error(invoke_context: &InvokeContext, e: EbpfError<BpfError>) -> InstructionError {
    ic_msg!(invoke_context, "{}", e);
    InstructionError::InvalidAccountData
}

mod executor_metrics {
    #[derive(Debug, Default)]
    pub struct CreateMetrics {
        pub program_id: String,
        pub load_elf_us: u64,
        pub verify_code_us: u64,
        pub jit_compile_us: u64,
    }

    impl CreateMetrics {
        pub fn submit_datapoint(&self) {
            datapoint_trace!(
                "create_executor_trace",
                ("program_id", self.program_id, String),
                ("load_elf_us", self.load_elf_us, i64),
                ("verify_code_us", self.verify_code_us, i64),
                ("jit_compile_us", self.jit_compile_us, i64),
            );
        }
    }
}

// The BPF loader is special in that it is the only place in the runtime and its built-in programs,
// where data comes not only from instruction account but also program accounts.
// Thus, these two helper methods have to distinguish the mixed sources via index_in_instruction.

fn get_index_in_transaction(
    instruction_context: &InstructionContext,
    index_in_instruction: usize,
) -> Result<usize, InstructionError> {
    if index_in_instruction < instruction_context.get_number_of_program_accounts() {
        instruction_context.get_index_of_program_account_in_transaction(index_in_instruction)
    } else {
        instruction_context.get_index_of_instruction_account_in_transaction(
            index_in_instruction
                .saturating_sub(instruction_context.get_number_of_program_accounts()),
        )
    }
}

fn try_borrow_account<'a>(
    transaction_context: &'a TransactionContext,
    instruction_context: &'a InstructionContext,
    index_in_instruction: usize,
) -> Result<BorrowedAccount<'a>, InstructionError> {
    if index_in_instruction < instruction_context.get_number_of_program_accounts() {
        instruction_context.try_borrow_program_account(transaction_context, index_in_instruction)
    } else {
        instruction_context.try_borrow_instruction_account(
            transaction_context,
            index_in_instruction
                .saturating_sub(instruction_context.get_number_of_program_accounts()),
        )
    }
}

pub fn create_executor(
    programdata_account_index: usize,
    programdata_offset: usize,
    invoke_context: &mut InvokeContext,
    _use_jit: bool,
    reject_deployment_of_broken_elfs: bool,
    disable_deploy_of_alloc_free_syscall: bool,
) -> Result<Arc<WasmExecutor>, InstructionError> {
    let mut register_syscalls_time = Measure::start("register_syscalls_time");
    let register_syscall_result =
        syscalls::register_syscalls(invoke_context, disable_deploy_of_alloc_free_syscall);
    register_syscalls_time.stop();
    invoke_context.timings.create_executor_register_syscalls_us = invoke_context
        .timings
        .create_executor_register_syscalls_us
        .saturating_add(register_syscalls_time.as_us());
    let syscall_registry = register_syscall_result.map_err(|e| {
        ic_msg!(invoke_context, "Failed to register syscalls: {}", e);
        InstructionError::ProgramEnvironmentSetupFailure
    })?;
    let compute_budget = invoke_context.get_compute_budget();
    let config = solana_rbpf::vm::Config {
        max_call_depth: compute_budget.max_call_depth,
        stack_frame_size: compute_budget.stack_frame_size,
        enable_stack_frame_gaps: true,
        instruction_meter_checkpoint_distance: 10000,
        enable_instruction_meter: true,
        enable_instruction_tracing: log_enabled!(Trace),
        enable_symbol_and_section_labels: false,
        disable_unresolved_symbols_at_runtime: invoke_context
            .feature_set
            .is_active(&disable_bpf_unresolved_symbols_at_runtime::id()),
        reject_broken_elfs: reject_deployment_of_broken_elfs,
        noop_instruction_rate: 256,
        sanitize_user_provided_values: true,
        encrypt_environment_registers: true,
        disable_deprecated_load_instructions: reject_deployment_of_broken_elfs
            && invoke_context
                .feature_set
                .is_active(&disable_bpf_deprecated_load_instructions::id()),
        syscall_bpf_function_hash_collision: invoke_context
            .feature_set
            .is_active(&error_on_syscall_bpf_function_hash_collisions::id()),
        reject_callx_r10: invoke_context
            .feature_set
            .is_active(&reject_callx_r10::id()),
        dynamic_stack_frames: false,
        enable_sdiv: false,
        optimize_rodata: false,
        static_syscalls: false,
        enable_elf_vaddr: false,
        // Warning, do not use `Config::default()` so that configuration here is explicit.
    };

    let mut wasmi_config = wasmi::Config::default();
    wasmi_config.consume_fuel(true);
    let engine = wasmi::Engine::new(&wasmi_config);

    let mut create_executor_metrics = executor_metrics::CreateMetrics::default();
    let executable = {
        let transaction_context = &invoke_context.transaction_context;
        let instruction_context = transaction_context.get_current_instruction_context()?;
        let programdata = try_borrow_account(
            transaction_context,
            instruction_context,
            programdata_account_index,
        )?;
        create_executor_metrics.program_id = programdata.get_key().to_string();
        let mut load_elf_time = Measure::start("load_elf_time");

        // TODO: load WASM here
        // TODO: our own Executable, not from solana_rbpf

        let mut data = programdata
                .get_data()
                .get(programdata_offset..)
                .ok_or(InstructionError::AccountDataTooSmall)?;
        let module = wasmi::Module::new(
            &engine,
            &mut data,
        ).expect("Binary should be valid WASM");
        load_elf_time.stop();
        create_executor_metrics.load_elf_us = load_elf_time.as_us();
        invoke_context.timings.create_executor_load_elf_us = invoke_context
            .timings
            .create_executor_load_elf_us
            .saturating_add(create_executor_metrics.load_elf_us);
        Ok(module)
    }
    .map_err(|e| map_ebpf_error(invoke_context, e))?;
    let mut verify_code_time = Measure::start("verify_code_time");
    let verified_executable = executable;
    verify_code_time.stop();
    create_executor_metrics.verify_code_us = verify_code_time.as_us();
    invoke_context.timings.create_executor_verify_code_us = invoke_context
        .timings
        .create_executor_verify_code_us
        .saturating_add(create_executor_metrics.verify_code_us);
    create_executor_metrics.submit_datapoint();
    Ok(Arc::new(WasmExecutor {
        engine,
        verified_executable,
        syscall_registry,
        config,
    }))
}

fn write_program_data(
    program_account_index: usize,
    program_data_offset: usize,
    bytes: &[u8],
    invoke_context: &mut InvokeContext,
) -> Result<(), InstructionError> {
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let mut program = try_borrow_account(
        transaction_context,
        instruction_context,
        program_account_index,
    )?;
    let data = program.get_data_mut()?;
    let write_offset = program_data_offset.saturating_add(bytes.len());
    if data.len() < write_offset {
        ic_msg!(
            invoke_context,
            "Write overflow: {} < {}",
            data.len(),
            write_offset,
        );
        return Err(InstructionError::AccountDataTooSmall);
    }
    data.get_mut(program_data_offset..write_offset)
        .ok_or(InstructionError::AccountDataTooSmall)?
        .copy_from_slice(bytes);
    Ok(())
}

fn check_loader_id(id: &Pubkey) -> bool {
    wasm_loader::check_id(id)
        || wasm_loader_deprecated::check_id(id)
        || wasm_loader_upgradeable::check_id(id)
}

/// Create the BPF virtual machine
pub fn create_vm<'a, 'b>(
    program: &'a VerifiedExecutable<RequisiteVerifier, BpfError, ThisInstructionMeter>,
    parameter_bytes: &mut [u8],
    orig_account_lengths: Vec<usize>,
    invoke_context: &'a mut InvokeContext<'b>,
) -> Result<EbpfVm<'a, RequisiteVerifier, BpfError, ThisInstructionMeter>, EbpfError<BpfError>> {
    let compute_budget = invoke_context.get_compute_budget();
    let heap_size = compute_budget.heap_size.unwrap_or(HEAP_LENGTH);
    if invoke_context
        .feature_set
        .is_active(&requestable_heap_size::id())
    {
        let _ = invoke_context.get_compute_meter().borrow_mut().consume(
            ((heap_size as u64).saturating_div(32_u64.saturating_mul(1024)))
                .saturating_sub(1)
                .saturating_mul(compute_budget.heap_cost),
        );
    }
    let mut heap =
        AlignedMemory::new_with_size(compute_budget.heap_size.unwrap_or(HEAP_LENGTH), HOST_ALIGN);
    let parameter_region = MemoryRegion::new_writable(parameter_bytes, MM_INPUT_START);
    let mut vm = EbpfVm::new(program, heap.as_slice_mut(), vec![parameter_region])?;
    syscalls::bind_syscall_context_objects(&mut vm, invoke_context, heap, orig_account_lengths)?;
    Ok(vm)
}

// Entrypoint
pub fn process_instruction(
    first_instruction_account: usize,
    invoke_context: &mut InvokeContext,
) -> Result<(), InstructionError> {
    process_instruction_common(first_instruction_account, invoke_context, false).inspect_err(|x| { dbg!(x); })
}

pub fn process_instruction_jit(
    first_instruction_account: usize,
    invoke_context: &mut InvokeContext,
) -> Result<(), InstructionError> {
    process_instruction_common(first_instruction_account, invoke_context, true)
}

fn process_instruction_common(
    first_instruction_account: usize,
    invoke_context: &mut InvokeContext,
    use_jit: bool,
) -> Result<(), InstructionError> {
    let log_collector = invoke_context.get_log_collector();
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context().inspect_err(|x| { dbg!(x); })?;
    let program_id = instruction_context.get_last_program_key(transaction_context).inspect_err(|x| { dbg!(x); })?;
    let first_account_key = transaction_context.get_key_of_account_at_index(
        get_index_in_transaction(instruction_context, first_instruction_account).inspect_err(|x| { dbg!(x); })?,
    ).inspect_err(|x| { dbg!(x); })?;
    let second_account_key = get_index_in_transaction(
        instruction_context,
        first_instruction_account.saturating_add(1),
    )
    .and_then(|index_in_transaction| {
        transaction_context.get_key_of_account_at_index(index_in_transaction)
    });

    let program_account_index = if first_account_key == program_id {
        first_instruction_account
    } else if second_account_key
        .map(|key| key == program_id)
        .unwrap_or(false)
    {
        first_instruction_account.saturating_add(1)
    } else {
        let first_account = try_borrow_account(
            transaction_context,
            instruction_context,
            first_instruction_account,
        ).inspect_err(|x| { dbg!(x); })?;
        if first_account.is_executable() {
            ic_logger_msg!(log_collector, "BPF loader is executable");
            return Err(InstructionError::IncorrectProgramId).inspect_err(|x| { dbg!(x); });
        }
        first_instruction_account
    };

    let program = try_borrow_account(
        transaction_context,
        instruction_context,
        program_account_index,
    ).inspect_err(|x| { dbg!(x); })?;
    if program.is_executable() {
        // First instruction account can only be zero if called from CPI, which
        // means stack height better be greater than one
        debug_assert_eq!(
            first_instruction_account == 0,
            invoke_context.get_stack_height() > 1
        );

        if !check_loader_id(program.get_owner()) {
            ic_logger_msg!(
                log_collector,
                "Executable account not owned by the BPF loader"
            );
            return Err(InstructionError::IncorrectProgramId).inspect_err(|x| { dbg!(x); });
        }

        let program_data_offset = if wasm_loader_upgradeable::check_id(program.get_owner()) {
            if let UpgradeableLoaderState::Program {
                programdata_address,
            } = program.get_state()?
            {
                if programdata_address != *first_account_key {
                    ic_logger_msg!(
                        log_collector,
                        "Wrong ProgramData account for this Program account"
                    );
                    return Err(InstructionError::InvalidArgument).inspect_err(|x| { dbg!(x); });
                }
                if !matches!(
                    instruction_context
                        .try_borrow_program_account(transaction_context, first_instruction_account)?
                        .get_state()?,
                    UpgradeableLoaderState::ProgramData {
                        slot: _,
                        upgrade_authority_address: _,
                    }
                ) {
                    ic_logger_msg!(log_collector, "Program has been closed");
                    return Err(InstructionError::InvalidAccountData).inspect_err(|x| { dbg!(x); });
                }
                UpgradeableLoaderState::size_of_programdata_metadata()
            } else {
                ic_logger_msg!(log_collector, "Invalid Program account");
                return Err(InstructionError::InvalidAccountData).inspect_err(|x| { dbg!(x); });
            }
        } else {
            0
        };
        drop(program);

        let mut get_or_create_executor_time = Measure::start("get_or_create_executor_time");
        let executor = match invoke_context.get_executor(program_id) {
            Some(executor) => executor,
            None => {
                let executor = create_executor(
                    first_instruction_account,
                    program_data_offset,
                    invoke_context,
                    use_jit,
                    false, /* reject_deployment_of_broken_elfs */
                    // allow _sol_alloc_free syscall for execution
                    false, /* disable_sol_alloc_free_syscall */
                )?;
                let transaction_context = &invoke_context.transaction_context;
                let instruction_context = transaction_context.get_current_instruction_context().inspect_err(|x| { dbg!(x); })?;
                let program_id = instruction_context.get_last_program_key(transaction_context).inspect_err(|x| { dbg!(x); })?;
                invoke_context.add_executor(program_id, executor.clone());
                executor
            }
        };
        get_or_create_executor_time.stop();
        saturating_add_assign!(
            invoke_context.timings.get_or_create_executor_us,
            get_or_create_executor_time.as_us()
        );

        executor.execute(program_account_index, invoke_context).inspect_err(|x| { dbg!(x); })
    } else {
        drop(program);
        debug_assert_eq!(first_instruction_account, 1);
        let disable_deprecated_loader = invoke_context
            .feature_set
            .is_active(&disable_deprecated_loader::id());
        if wasm_loader_upgradeable::check_id(program_id) {
            process_loader_upgradeable_instruction(
                first_instruction_account,
                invoke_context,
                use_jit,
            ).inspect_err(|x| { dbg!(x); })
        } else if wasm_loader::check_id(program_id)
            || (!disable_deprecated_loader && wasm_loader_deprecated::check_id(program_id))
        {
            process_loader_instruction(first_instruction_account, invoke_context, use_jit).inspect_err(|x| { dbg!(x); })
        } else if disable_deprecated_loader && wasm_loader_deprecated::check_id(program_id) {
            ic_logger_msg!(log_collector, "Deprecated loader is no longer supported");
            Err(InstructionError::UnsupportedProgramId).inspect_err(|x| { dbg!(x); })
        } else {
            ic_logger_msg!(log_collector, "Invalid WASM loader id");
            Err(InstructionError::IncorrectProgramId).inspect_err(|x| { dbg!(x); })
        }
    }
}

fn process_loader_upgradeable_instruction(
    first_instruction_account: usize,
    invoke_context: &mut InvokeContext,
    use_jit: bool,
) -> Result<(), InstructionError> {
    let log_collector = invoke_context.get_log_collector();
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let instruction_data = instruction_context.get_instruction_data();
    let program_id = instruction_context.get_last_program_key(transaction_context)?;

    match limited_deserialize(instruction_data)? {
        UpgradeableLoaderInstruction::InitializeBuffer => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let mut buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;

            if UpgradeableLoaderState::Uninitialized != buffer.get_state()? {
                ic_logger_msg!(log_collector, "Buffer account already initialized");
                return Err(InstructionError::AccountAlreadyInitialized);
            }

            let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?);

            buffer.set_state(&UpgradeableLoaderState::Buffer {
                authority_address: authority_key,
            })?;
        }
        UpgradeableLoaderInstruction::Write { offset, bytes } => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;

            if let UpgradeableLoaderState::Buffer { authority_address } = buffer.get_state()? {
                if authority_address.is_none() {
                    ic_logger_msg!(log_collector, "Buffer is immutable");
                    return Err(InstructionError::Immutable); // TODO better error code
                }
                let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                    instruction_context.get_index_of_instruction_account_in_transaction(1)?,
                )?);
                if authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Incorrect buffer authority provided");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(1)? {
                    ic_logger_msg!(log_collector, "Buffer authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Buffer account");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(buffer);
            write_program_data(
                first_instruction_account,
                UpgradeableLoaderState::size_of_buffer_metadata().saturating_add(offset as usize),
                &bytes,
                invoke_context,
            )?;
        }
        UpgradeableLoaderInstruction::DeployWithMaxDataLen { max_data_len } => {
            instruction_context.check_number_of_instruction_accounts(4)?;
            let payer_key = *transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(0)?,
            )?;
            let programdata_key = *transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?;
            let rent = get_sysvar_with_account_check::rent(invoke_context, instruction_context, 4)?;
            let clock =
                get_sysvar_with_account_check::clock(invoke_context, instruction_context, 5)?;
            instruction_context.check_number_of_instruction_accounts(8)?;
            let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(7)?,
            )?);

            // Verify Program account

            let program =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            if UpgradeableLoaderState::Uninitialized != program.get_state()? {
                ic_logger_msg!(log_collector, "Program account already initialized");
                return Err(InstructionError::AccountAlreadyInitialized);
            }
            if program.get_data().len() < UpgradeableLoaderState::size_of_program() {
                ic_logger_msg!(log_collector, "Program account too small");
                return Err(InstructionError::AccountDataTooSmall);
            }
            if program.get_lamports() < rent.minimum_balance(program.get_data().len()) {
                ic_logger_msg!(log_collector, "Program account not rent-exempt");
                return Err(InstructionError::ExecutableAccountNotRentExempt);
            }
            let new_program_id = *program.get_key();
            drop(program);

            // Verify Buffer account

            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
            if let UpgradeableLoaderState::Buffer { authority_address } = buffer.get_state()? {
                if authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Buffer and upgrade authority don't match");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(7)? {
                    ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Buffer account");
                return Err(InstructionError::InvalidArgument);
            }
            let buffer_key = *buffer.get_key();
            let buffer_lamports = buffer.get_lamports();
            let buffer_data_offset = UpgradeableLoaderState::size_of_buffer_metadata();
            let buffer_data_len = buffer.get_data().len().saturating_sub(buffer_data_offset);
            let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
            let programdata_len = UpgradeableLoaderState::size_of_programdata(max_data_len);
            if buffer.get_data().len() < UpgradeableLoaderState::size_of_buffer_metadata()
                || buffer_data_len == 0
            {
                ic_logger_msg!(log_collector, "Buffer account too small");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(buffer);
            if max_data_len < buffer_data_len {
                ic_logger_msg!(
                    log_collector,
                    "Max data length is too small to hold Buffer data"
                );
                return Err(InstructionError::AccountDataTooSmall);
            }
            if programdata_len > MAX_PERMITTED_DATA_LENGTH as usize {
                ic_logger_msg!(log_collector, "Max data length is too large");
                return Err(InstructionError::InvalidArgument);
            }

            // Create ProgramData account

            let (derived_address, bump_seed) =
                Pubkey::find_program_address(&[new_program_id.as_ref()], program_id);
            if derived_address != programdata_key {
                ic_logger_msg!(log_collector, "ProgramData address is not derived");
                return Err(InstructionError::InvalidArgument);
            }

            let predrain_buffer = invoke_context
                .feature_set
                .is_active(&reduce_required_deploy_balance::id());
            if predrain_buffer {
                // Drain the Buffer account to payer before paying for programdata account
                let mut payer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
                payer.checked_add_lamports(buffer_lamports)?;
                drop(payer);
                let mut buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
                buffer.set_lamports(0)?;
            }

            let mut instruction = system_instruction::create_account(
                &payer_key,
                &programdata_key,
                1.max(rent.minimum_balance(programdata_len)),
                programdata_len as u64,
                program_id,
            );

            // pass an extra account to avoid the overly strict UnbalancedInstruction error
            instruction
                .accounts
                .push(AccountMeta::new(buffer_key, false));

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let caller_program_id =
                instruction_context.get_last_program_key(transaction_context)?;
            let signers = [&[new_program_id.as_ref(), &[bump_seed]]]
                .iter()
                .map(|seeds| Pubkey::create_program_address(*seeds, caller_program_id))
                .collect::<Result<Vec<Pubkey>, domichain_sdk::pubkey::PubkeyError>>()?;
            invoke_context.native_invoke(instruction, signers.as_slice())?;

            // Load and verify the program bits
            let executor = create_executor(
                first_instruction_account.saturating_add(3),
                buffer_data_offset,
                invoke_context,
                use_jit,
                true,
                invoke_context
                    .feature_set
                    .is_active(&disable_deploy_of_alloc_free_syscall::id()),
            )?;
            invoke_context.update_executor(&new_program_id, executor);

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;

            // Update the ProgramData account and record the program bits
            {
                let mut programdata =
                    instruction_context.try_borrow_instruction_account(transaction_context, 1)?;
                programdata.set_state(&UpgradeableLoaderState::ProgramData {
                    slot: clock.slot,
                    upgrade_authority_address: authority_key,
                })?;
                let dst_slice = programdata
                    .get_data_mut()?
                    .get_mut(
                        programdata_data_offset
                            ..programdata_data_offset.saturating_add(buffer_data_len),
                    )
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                let buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
                let src_slice = buffer
                    .get_data()
                    .get(buffer_data_offset..)
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                dst_slice.copy_from_slice(src_slice);
            }

            // Update the Program account
            let mut program =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            program.set_state(&UpgradeableLoaderState::Program {
                programdata_address: programdata_key,
            })?;
            program.set_executable(true)?;
            drop(program);

            if !predrain_buffer {
                // Drain the Buffer account back to the payer
                let mut payer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
                payer.checked_add_lamports(buffer_lamports)?;
                drop(payer);
                let mut buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
                buffer.set_lamports(0)?;
            }

            ic_logger_msg!(log_collector, "Deployed program {:?}", new_program_id);
        }
        UpgradeableLoaderInstruction::Upgrade => {
            instruction_context.check_number_of_instruction_accounts(3)?;
            let programdata_key = *transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(0)?,
            )?;
            let rent = get_sysvar_with_account_check::rent(invoke_context, instruction_context, 4)?;
            let clock =
                get_sysvar_with_account_check::clock(invoke_context, instruction_context, 5)?;
            instruction_context.check_number_of_instruction_accounts(7)?;
            let authority_key = Some(*transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(6)?,
            )?);

            // Verify Program account

            let program =
                instruction_context.try_borrow_instruction_account(transaction_context, 1)?;
            if !program.is_executable() {
                ic_logger_msg!(log_collector, "Program account not executable");
                return Err(InstructionError::AccountNotExecutable);
            }
            if !program.is_writable() {
                ic_logger_msg!(log_collector, "Program account not writeable");
                return Err(InstructionError::InvalidArgument);
            }
            if program.get_owner() != program_id {
                ic_logger_msg!(log_collector, "Program account not owned by loader");
                return Err(InstructionError::IncorrectProgramId);
            }
            if let UpgradeableLoaderState::Program {
                programdata_address,
            } = program.get_state()?
            {
                if programdata_address != programdata_key {
                    ic_logger_msg!(log_collector, "Program and ProgramData account mismatch");
                    return Err(InstructionError::InvalidArgument);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Program account");
                return Err(InstructionError::InvalidAccountData);
            }
            let new_program_id = *program.get_key();
            drop(program);

            // Verify Buffer account

            let buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            if let UpgradeableLoaderState::Buffer { authority_address } = buffer.get_state()? {
                if authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Buffer and upgrade authority don't match");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(6)? {
                    ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid Buffer account");
                return Err(InstructionError::InvalidArgument);
            }
            let buffer_lamports = buffer.get_lamports();
            let buffer_data_offset = UpgradeableLoaderState::size_of_buffer_metadata();
            let buffer_data_len = buffer.get_data().len().saturating_sub(buffer_data_offset);
            if buffer.get_data().len() < UpgradeableLoaderState::size_of_buffer_metadata()
                || buffer_data_len == 0
            {
                ic_logger_msg!(log_collector, "Buffer account too small");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(buffer);

            // Verify ProgramData account

            let programdata =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
            let programdata_balance_required =
                1.max(rent.minimum_balance(programdata.get_data().len()));
            if programdata.get_data().len()
                < UpgradeableLoaderState::size_of_programdata(buffer_data_len)
            {
                ic_logger_msg!(log_collector, "ProgramData account not large enough");
                return Err(InstructionError::AccountDataTooSmall);
            }
            if programdata.get_lamports().saturating_add(buffer_lamports)
                < programdata_balance_required
            {
                ic_logger_msg!(
                    log_collector,
                    "Buffer account balance too low to fund upgrade"
                );
                return Err(InstructionError::InsufficientFunds);
            }
            if let UpgradeableLoaderState::ProgramData {
                slot: _,
                upgrade_authority_address,
            } = programdata.get_state()?
            {
                if upgrade_authority_address.is_none() {
                    ic_logger_msg!(log_collector, "Program not upgradeable");
                    return Err(InstructionError::Immutable);
                }
                if upgrade_authority_address != authority_key {
                    ic_logger_msg!(log_collector, "Incorrect upgrade authority provided");
                    return Err(InstructionError::IncorrectAuthority);
                }
                if !instruction_context.is_instruction_account_signer(6)? {
                    ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                    return Err(InstructionError::MissingRequiredSignature);
                }
            } else {
                ic_logger_msg!(log_collector, "Invalid ProgramData account");
                return Err(InstructionError::InvalidAccountData);
            }
            drop(programdata);

            // Load and verify the program bits
            let executor = create_executor(
                first_instruction_account.saturating_add(2),
                buffer_data_offset,
                invoke_context,
                use_jit,
                true,
                invoke_context
                    .feature_set
                    .is_active(&disable_deploy_of_alloc_free_syscall::id()),
            )?;
            invoke_context.update_executor(&new_program_id, executor);

            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;

            // Update the ProgramData account, record the upgraded data, and zero
            // the rest
            let mut programdata =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            {
                programdata.set_state(&UpgradeableLoaderState::ProgramData {
                    slot: clock.slot,
                    upgrade_authority_address: authority_key,
                })?;
                let dst_slice = programdata
                    .get_data_mut()?
                    .get_mut(
                        programdata_data_offset
                            ..programdata_data_offset.saturating_add(buffer_data_len),
                    )
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                let buffer =
                    instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
                let src_slice = buffer
                    .get_data()
                    .get(buffer_data_offset..)
                    .ok_or(InstructionError::AccountDataTooSmall)?;
                dst_slice.copy_from_slice(src_slice);
            }
            programdata
                .get_data_mut()?
                .get_mut(programdata_data_offset.saturating_add(buffer_data_len)..)
                .ok_or(InstructionError::AccountDataTooSmall)?
                .fill(0);

            // Fund ProgramData to rent-exemption, spill the rest

            let programdata_lamports = programdata.get_lamports();
            programdata.set_lamports(programdata_balance_required)?;
            drop(programdata);

            let mut buffer =
                instruction_context.try_borrow_instruction_account(transaction_context, 2)?;
            buffer.set_lamports(0)?;
            drop(buffer);

            let mut spill =
                instruction_context.try_borrow_instruction_account(transaction_context, 3)?;
            spill.checked_add_lamports(
                programdata_lamports
                    .saturating_add(buffer_lamports)
                    .saturating_sub(programdata_balance_required),
            )?;

            ic_logger_msg!(log_collector, "Upgraded program {:?}", new_program_id);
        }
        UpgradeableLoaderInstruction::SetAuthority => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            let mut account =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let present_authority_key = transaction_context.get_key_of_account_at_index(
                instruction_context.get_index_of_instruction_account_in_transaction(1)?,
            )?;
            let new_authority = instruction_context
                .get_index_of_instruction_account_in_transaction(2)
                .and_then(|index_in_transaction| {
                    transaction_context.get_key_of_account_at_index(index_in_transaction)
                })
                .ok();

            match account.get_state()? {
                UpgradeableLoaderState::Buffer { authority_address } => {
                    if new_authority.is_none() {
                        ic_logger_msg!(log_collector, "Buffer authority is not optional");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if authority_address.is_none() {
                        ic_logger_msg!(log_collector, "Buffer is immutable");
                        return Err(InstructionError::Immutable);
                    }
                    if authority_address != Some(*present_authority_key) {
                        ic_logger_msg!(log_collector, "Incorrect buffer authority provided");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if !instruction_context.is_instruction_account_signer(1)? {
                        ic_logger_msg!(log_collector, "Buffer authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    account.set_state(&UpgradeableLoaderState::Buffer {
                        authority_address: new_authority.cloned(),
                    })?;
                }
                UpgradeableLoaderState::ProgramData {
                    slot,
                    upgrade_authority_address,
                } => {
                    if upgrade_authority_address.is_none() {
                        ic_logger_msg!(log_collector, "Program not upgradeable");
                        return Err(InstructionError::Immutable);
                    }
                    if upgrade_authority_address != Some(*present_authority_key) {
                        ic_logger_msg!(log_collector, "Incorrect upgrade authority provided");
                        return Err(InstructionError::IncorrectAuthority);
                    }
                    if !instruction_context.is_instruction_account_signer(1)? {
                        ic_logger_msg!(log_collector, "Upgrade authority did not sign");
                        return Err(InstructionError::MissingRequiredSignature);
                    }
                    account.set_state(&UpgradeableLoaderState::ProgramData {
                        slot,
                        upgrade_authority_address: new_authority.cloned(),
                    })?;
                }
                _ => {
                    ic_logger_msg!(log_collector, "Account does not support authorities");
                    return Err(InstructionError::InvalidArgument);
                }
            }

            ic_logger_msg!(log_collector, "New authority {:?}", new_authority);
        }
        UpgradeableLoaderInstruction::Close => {
            instruction_context.check_number_of_instruction_accounts(2)?;
            if instruction_context.get_index_of_instruction_account_in_transaction(0)?
                == instruction_context.get_index_of_instruction_account_in_transaction(1)?
            {
                ic_logger_msg!(
                    log_collector,
                    "Recipient is the same as the account being closed"
                );
                return Err(InstructionError::InvalidArgument);
            }
            let mut close_account =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            let close_key = *close_account.get_key();
            match close_account.get_state()? {
                UpgradeableLoaderState::Uninitialized => {
                    let close_lamports = close_account.get_lamports();
                    close_account.set_lamports(0)?;
                    drop(close_account);
                    let mut recipient_account = instruction_context
                        .try_borrow_instruction_account(transaction_context, 1)?;
                    recipient_account.checked_add_lamports(close_lamports)?;

                    ic_logger_msg!(log_collector, "Closed Uninitialized {}", close_key);
                }
                UpgradeableLoaderState::Buffer { authority_address } => {
                    instruction_context.check_number_of_instruction_accounts(3)?;
                    drop(close_account);
                    common_close_account(
                        &authority_address,
                        transaction_context,
                        instruction_context,
                        &log_collector,
                    )?;

                    ic_logger_msg!(log_collector, "Closed Buffer {}", close_key);
                }
                UpgradeableLoaderState::ProgramData {
                    slot: _,
                    upgrade_authority_address: authority_address,
                } => {
                    instruction_context.check_number_of_instruction_accounts(4)?;
                    drop(close_account);
                    let program_account = instruction_context
                        .try_borrow_instruction_account(transaction_context, 3)?;
                    let program_key = *program_account.get_key();

                    if !program_account.is_writable() {
                        ic_logger_msg!(log_collector, "Program account is not writable");
                        return Err(InstructionError::InvalidArgument);
                    }
                    if program_account.get_owner() != program_id {
                        ic_logger_msg!(log_collector, "Program account not owned by loader");
                        return Err(InstructionError::IncorrectProgramId);
                    }

                    match program_account.get_state()? {
                        UpgradeableLoaderState::Program {
                            programdata_address,
                        } => {
                            if programdata_address != close_key {
                                ic_logger_msg!(
                                    log_collector,
                                    "ProgramData account does not match ProgramData account"
                                );
                                return Err(InstructionError::InvalidArgument);
                            }

                            drop(program_account);
                            common_close_account(
                                &authority_address,
                                transaction_context,
                                instruction_context,
                                &log_collector,
                            )?;
                        }
                        _ => {
                            ic_logger_msg!(log_collector, "Invalid Program account");
                            return Err(InstructionError::InvalidArgument);
                        }
                    }

                    ic_logger_msg!(log_collector, "Closed Program {}", program_key);
                }
                _ => {
                    ic_logger_msg!(log_collector, "Account does not support closing");
                    return Err(InstructionError::InvalidArgument);
                }
            }
        }
    }

    Ok(())
}

fn common_close_account(
    authority_address: &Option<Pubkey>,
    transaction_context: &TransactionContext,
    instruction_context: &InstructionContext,
    log_collector: &Option<Rc<RefCell<LogCollector>>>,
) -> Result<(), InstructionError> {
    if authority_address.is_none() {
        ic_logger_msg!(log_collector, "Account is immutable");
        return Err(InstructionError::Immutable);
    }
    if *authority_address
        != Some(*transaction_context.get_key_of_account_at_index(
            instruction_context.get_index_of_instruction_account_in_transaction(2)?,
        )?)
    {
        ic_logger_msg!(log_collector, "Incorrect authority provided");
        return Err(InstructionError::IncorrectAuthority);
    }
    if !instruction_context.is_instruction_account_signer(2)? {
        ic_logger_msg!(log_collector, "Authority did not sign");
        return Err(InstructionError::MissingRequiredSignature);
    }

    let mut close_account =
        instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
    let mut recipient_account =
        instruction_context.try_borrow_instruction_account(transaction_context, 1)?;
    recipient_account.checked_add_lamports(close_account.get_lamports())?;
    close_account.set_lamports(0)?;
    close_account.set_state(&UpgradeableLoaderState::Uninitialized)?;
    Ok(())
}

fn process_loader_instruction(
    first_instruction_account: usize,
    invoke_context: &mut InvokeContext,
    use_jit: bool,
) -> Result<(), InstructionError> {
    let transaction_context = &invoke_context.transaction_context;
    let instruction_context = transaction_context.get_current_instruction_context()?;
    let instruction_data = instruction_context.get_instruction_data();
    let program_id = instruction_context.get_last_program_key(transaction_context)?;
    let program = instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
    if program.get_owner() != program_id {
        ic_msg!(
            invoke_context,
            "Executable account not owned by the BPF loader"
        );
        return Err(InstructionError::IncorrectProgramId);
    }
    let is_program_signer = program.is_signer();
    drop(program);
    match limited_deserialize(instruction_data)? {
        LoaderInstruction::Write { offset, bytes } => {
            if !is_program_signer {
                ic_msg!(invoke_context, "Program account did not sign");
                return Err(InstructionError::MissingRequiredSignature);
            }
            write_program_data(
                first_instruction_account,
                offset as usize,
                &bytes,
                invoke_context,
            )?;
        }
        LoaderInstruction::Finalize => {
            if !is_program_signer {
                ic_msg!(invoke_context, "key[0] did not sign the transaction");
                return Err(InstructionError::MissingRequiredSignature);
            }
            let executor = create_executor(
                first_instruction_account,
                0,
                invoke_context,
                use_jit,
                true,
                invoke_context
                    .feature_set
                    .is_active(&disable_deploy_of_alloc_free_syscall::id()),
            )?;
            let transaction_context = &invoke_context.transaction_context;
            let instruction_context = transaction_context.get_current_instruction_context()?;
            let mut program =
                instruction_context.try_borrow_instruction_account(transaction_context, 0)?;
            invoke_context.update_executor(program.get_key(), executor);
            program.set_executable(true)?;
            ic_msg!(invoke_context, "Finalized account {:?}", program.get_key());
        }
    }

    Ok(())
}

/// Passed to the VM to enforce the compute budget
pub struct ThisInstructionMeter {
    pub compute_meter: Rc<RefCell<ComputeMeter>>,
}
impl ThisInstructionMeter {
    #[allow(dead_code)]
    fn new(compute_meter: Rc<RefCell<ComputeMeter>>) -> Self {
        Self { compute_meter }
    }
}
impl InstructionMeter for ThisInstructionMeter {
    fn consume(&mut self, amount: u64) {
        // 1 to 1 instruction to compute unit mapping
        // ignore error, Ebpf will bail if exceeded
        let _ = self.compute_meter.borrow_mut().consume(amount);
    }
    fn get_remaining(&self) -> u64 {
        self.compute_meter.borrow().get_remaining()
    }
}

/// WASM Loader's Executor implementation
pub struct WasmExecutor {
    engine: wasmi::Engine,
    verified_executable: wasmi::Module,
    #[allow(dead_code)]
    syscall_registry: SyscallRegistry,
    config: solana_rbpf::vm::Config,
}

// Well, implement Debug for solana_rbpf::vm::Executable in solana-rbpf...
impl Debug for WasmExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "WasmExecutor({:p})", self)
    }
}

type HostState<'a, 'b> = (WasiCtx, Rc<RefCell<&'a mut InvokeContext<'b>>>, solana_rbpf::vm::Config);

fn map_wasmi_to_bpf_syscalls<'a, 'b>(
    init: fn(Rc<RefCell<&'a mut InvokeContext<'b>>>) -> Box<(dyn SyscallObject<BpfError> + 'a)>,
    mut caller: Caller<'_, HostState<'a, 'b>>,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> Result<i32, Trap> {
    let ic = caller.data().1.clone();
    let config = caller.data().2;

    let mut syscall = init(ic);

    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => panic!("failed to find host memory"),
    };
    let data = mem.data_mut(&mut caller);

    let ro_region = MemoryRegion::new_readonly(&[], MM_PROGRAM_START);

    let mut stack = solana_rbpf::call_frames::CallFrames::new(&config);

    let mut parameter_bytes = [];
    let parameter_region = MemoryRegion::new_writable(&mut parameter_bytes, MM_INPUT_START);

    let regions: Vec<MemoryRegion> = vec![
        MemoryRegion::new_readonly(&[], 0),
        ro_region,
        stack.get_memory_region(),
        MemoryRegion::new_writable(data, MM_HEAP_START),
        parameter_region,
    ];

    let mut memory_mapping = MemoryMapping::new::<BpfError>(regions, &config).unwrap();

    let mut result = Ok(0);

    syscall.call(arg1, arg2, arg3, arg4, arg5, &mut memory_mapping, &mut result);

    match result {
        Ok(i) => Ok(i as _),
        Err(e) => Err(Trap::new(format!("{:?}", e))),
    }
}

impl Executor for WasmExecutor {
    fn execute(
        &self,
        _first_instruction_account: usize,
        invoke_context: &mut InvokeContext,
    ) -> Result<(), InstructionError> {
        let blake3_syscall_enabled = invoke_context
            .feature_set
            .is_active(&blake3_syscall_enabled::id());
        let zk_token_sdk_enabled = invoke_context
            .feature_set
            .is_active(&zk_token_sdk_enabled::id());
        let disable_fees_sysvar = invoke_context
            .feature_set
            .is_active(&disable_fees_sysvar::id());
        let disable_deploy_of_alloc_free_syscall = invoke_context
            .feature_set
            .is_active(&disable_deploy_of_alloc_free_syscall::id());

        let log_collector = invoke_context.get_log_collector();
        let compute_meter = invoke_context.get_compute_meter();
        let stack_height = invoke_context.get_stack_height();
        let transaction_context = &invoke_context.transaction_context;
        let instruction_context = transaction_context.get_current_instruction_context().inspect_err(|x| { dbg!(x); })?;
        let program_id = *instruction_context.get_last_program_key(transaction_context).inspect_err(|x| { dbg!(x); })?;

        let mut serialize_time = Measure::start("serialize");
        let (mut parameter_bytes, account_lengths) =
            serialize_parameters(invoke_context.transaction_context, instruction_context).inspect_err(|x| { dbg!(x); })?;
        serialize_time.stop();

        let invoke_context = Rc::new(RefCell::new(invoke_context));

        let mut create_vm_time = Measure::start("create_vm");
        let mut execute_time;
        let execution_result = {
            // TODO: create_vm
            let ctx = wasmi_wasi::WasiCtxBuilder::new().build();
            let mut store = wasmi::Store::new(&self.engine, (ctx, invoke_context.clone(), self.config));
            let mut linker = <wasmi::Linker<HostState>>::new(&self.engine);

            wasmi_wasi::add_to_linker(&mut linker, |ctx| &mut ctx.0)
                .map_err(|error| format!("failed to add WASI definitions to the linker: {error}")).unwrap();

            linker.define(
                "env",
                "abort",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>| {
                    map_wasmi_to_bpf_syscalls(SyscallAbort::init, caller, 0, 0, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_panic_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, file: u64, len: u64, line: u64, column: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallPanic::init, caller, file, len, line, column, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_log_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, message: i32, len: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallLog::init, caller, message as _, len as _, 0, 0, 0)?;
                    Ok(())
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_log_64_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallLogU64::init, caller, arg1, arg2, arg3, arg4, arg5)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_log_compute_units_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>| {
                    map_wasmi_to_bpf_syscalls(SyscallLogBpfComputeUnits::init, caller, 0, 0, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_log_pubkey",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, pubkey_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallLogPubkey::init, caller, pubkey_addr as _, 0, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_create_program_address",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, seeds_addr: u64, seeds_len: u64, program_id_addr: u64, address_addr: u64,| {
                    map_wasmi_to_bpf_syscalls(SyscallCreateProgramAddress::init, caller, seeds_addr, seeds_len, program_id_addr, address_addr, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_try_find_program_address",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, seeds_addr: u64, seeds_len: u64, program_id_addr: u64, address_addr: u64, bump_seed_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallTryFindProgramAddress::init, caller, seeds_addr, seeds_len, program_id_addr, address_addr, bump_seed_addr)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_sha256",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, vals_addr: u32, vals_len: u64, result_addr: u32| {
                    map_wasmi_to_bpf_syscalls(SyscallSha256::init, caller, vals_addr as _, vals_len, result_addr as _, 0, 0)
                        .map(|i| i as u64)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_keccak256",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, vals_addr: u32, vals_len: u64, result_addr: u32| {
                    map_wasmi_to_bpf_syscalls(SyscallKeccak256::init, caller, vals_addr as _, vals_len, result_addr as _, 0, 0)
                        .map(|i| i as u64)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_secp256k1_recover",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, hash_addr: u64, recovery_id_val: u64, signature_addr: u64, result_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallSecp256k1Recover::init, caller, hash_addr, recovery_id_val, signature_addr, result_addr, 0)
                }),
            ).unwrap();

            if blake3_syscall_enabled {
                linker.define(
                    "env",
                    "sol_blake3",
                    Func::wrap(&mut store, |caller: Caller<'_, HostState>, vals_addr: u32, vals_len: u64, result_addr: u32| {
                        map_wasmi_to_bpf_syscalls(SyscallBlake3::init, caller, vals_addr as _, vals_len, result_addr as _, 0, 0)
                            .map(|i| i as u64)
                    }),
                ).unwrap();
            }

            if zk_token_sdk_enabled {
                linker.define(
                    "env",
                    "sol_zk_token_elgamal_op",
                    Func::wrap(&mut store, |caller: Caller<'_, HostState>, op: u64, ct_0_addr: u64, ct_1_addr: u64, ct_result_addr: u64| {
                        map_wasmi_to_bpf_syscalls(SyscallZkTokenElgamalOp::init, caller, op, ct_0_addr, ct_1_addr, ct_result_addr, 0)
                    }),
                ).unwrap();

                linker.define(
                    "env",
                    "sol_zk_token_elgamal_op_with_lo_hi",
                    Func::wrap(&mut store, |caller: Caller<'_, HostState>, op: u64, ct_0_addr: u64, ct_1_lo_addr: u64, ct_1_hi_addr: u64, ct_result_addr: u64| {
                        map_wasmi_to_bpf_syscalls(SyscallZkTokenElgamalOpWithLoHi::init, caller, op, ct_0_addr, ct_1_lo_addr, ct_1_hi_addr, ct_result_addr)
                    }),
                ).unwrap();

                linker.define(
                    "env",
                    "sol_zk_token_elgamal_op_with_scalar",
                    Func::wrap(&mut store, |caller: Caller<'_, HostState>, op: u64, ct_addr: u64, scalar: u64, ct_result_addr: u64| {
                        map_wasmi_to_bpf_syscalls(SyscallZkTokenElgamalOpWithScalar::init, caller, op, ct_addr, scalar, ct_result_addr, 0)
                    }),
                ).unwrap();
            }

            linker.define(
                "env",
                "sol_curve_validate_point",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, curve_id: u64, point_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallCurvePointValidation::init, caller, curve_id, point_addr, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_curve_group_op",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, curve_id: u64, group_op: u64, left_input_addr: u64, right_input_addr: u64, result_point_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallCurveGroupOps::init, caller, curve_id, group_op, left_input_addr, right_input_addr, result_point_addr)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_get_clock_sysvar",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, var_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallGetClockSysvar::init, caller, var_addr, 0, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_get_epoch_schedule_sysvar",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, var_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallGetEpochScheduleSysvar::init, caller, var_addr, 0, 0, 0, 0)
                }),
            ).unwrap();

            if !disable_fees_sysvar {
                linker.define(
                    "env",
                    "sol_get_fees_sysvar",
                    Func::wrap(&mut store, |caller: Caller<'_, HostState>, var_addr: u64| {
                        map_wasmi_to_bpf_syscalls(SyscallGetFeesSysvar::init, caller, var_addr, 0, 0, 0, 0)
                    }),
                ).unwrap();
            }

            linker.define(
                "env",
                "sol_get_rent_sysvar",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, var_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallGetRentSysvar::init, caller, var_addr, 0, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_memcpy_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, dst_addr: u64, src_addr: u64, n: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallMemcpy::init, caller, dst_addr, src_addr, n, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_memmove_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, dst_addr: u64, src_addr: u64, n: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallMemmove::init, caller, dst_addr, src_addr, n, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_memcmp_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, s1_addr: u64, s2_addr: u64, n: u64, cmp_result_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallMemcmp::init, caller, s1_addr, s2_addr, n, cmp_result_addr, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_memset_",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, s_addr: u64, c: u64, n: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallMemset::init, caller, s_addr, c, n, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_invoke_signed_c",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, instruction_addr: u64, account_infos_addr: u64, account_infos_len: u64, signers_seeds_addr: u64, signers_seeds_len: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallInvokeSignedC::init, caller, instruction_addr, account_infos_addr, account_infos_len, signers_seeds_addr, signers_seeds_len)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_invoke_signed_rust",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, instruction_addr: u64, account_infos_addr: u64, account_infos_len: u64, signers_seeds_addr: u64, signers_seeds_len: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallInvokeSignedRust::init, caller, instruction_addr, account_infos_addr, account_infos_len, signers_seeds_addr, signers_seeds_len)
                }),
            ).unwrap();

            if !disable_deploy_of_alloc_free_syscall {
                linker.define(
                    "env",
                    "sol_alloc_free_",
                    Func::wrap(&mut store, |caller: Caller<'_, HostState>, size: u64, free_addr: u64| {
                        map_wasmi_to_bpf_syscalls(SyscallAllocFree::init, caller, size, free_addr, 0, 0, 0)
                    }),
                ).unwrap();
            }

            linker.define(
                "env",
                "sol_set_return_data",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, addr: u64, len: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallSetReturnData::init, caller, addr, len, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_get_return_data",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, return_data_addr: u64, length: u64, program_id_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallGetReturnData::init, caller, return_data_addr, length, program_id_addr, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_log_data",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, addr: u64, len: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallLogData::init, caller, addr, len, 0, 0, 0)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_get_processed_sibling_instruction",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>, index: u64, meta_addr: u64, program_id_addr: u64, data_addr: u64, accounts_addr: u64| {
                    map_wasmi_to_bpf_syscalls(SyscallGetProcessedSiblingInstruction::init, caller, index, meta_addr, program_id_addr, data_addr, accounts_addr)
                }),
            ).unwrap();

            linker.define(
                "env",
                "sol_get_stack_height",
                Func::wrap(&mut store, |caller: Caller<'_, HostState>| {
                    map_wasmi_to_bpf_syscalls(SyscallGetStackHeight::init, caller, 0, 0, 0, 0, 0)
                }),
            ).unwrap();

            let instance = linker
                .instantiate(&mut store, &self.verified_executable).unwrap()
                .start(&mut store).unwrap();

            let memory = instance.get_memory(&mut store, "memory").unwrap();
            let parameter_bytes_slice = parameter_bytes.as_slice();
            let parameter_bytes_slice_len = parameter_bytes_slice.len();
            memory.data_mut(&mut store)[0..parameter_bytes_slice_len]
                .copy_from_slice(parameter_bytes_slice);

            let vm = instance.get_typed_func::<i32, i64>(&store, "entrypoint").unwrap();

            let check_aligned = true;
            let check_size = invoke_context.borrow()
                .feature_set
                .is_active(&check_slice_translation_size::id());
            let heap_size = invoke_context.borrow().get_compute_budget().heap_size.unwrap_or(HEAP_LENGTH);
            let heap =
                AlignedMemory::new_with_size(heap_size, HOST_ALIGN);
            invoke_context.borrow_mut()
                .set_syscall_context(
                    check_aligned,
                    check_size,
                    account_lengths,
                    Rc::new(RefCell::new( crate::allocator_bump::BpfAllocator::new(heap, solana_rbpf::ebpf::MM_HEAP_START))),
                )
                .map_err(SyscallError::InstructionError).unwrap();

            create_vm_time.stop();

            execute_time = Measure::start("execute");
            stable_log::program_invoke(&log_collector, &program_id, stack_height);

            // TODO(Dev): return instruction meter and the rest
            // let mut instruction_meter = ThisInstructionMeter::new(compute_meter.clone());
            let before = compute_meter.borrow().get_remaining();
            store.add_fuel(before).unwrap();

            let result = vm.call(&mut store, 0); // sending NULL pointer to params

            // let result = if self.use_jit {
            //     vm.execute_program_jit(&mut instruction_meter)
            // } else {
            //     vm.execute_program_interpreted(&mut instruction_meter)
            // };

            let consumed_fuel = store.fuel_consumed().unwrap();
            let after_syscalls = compute_meter.borrow().get_remaining();
            let compute_meter_result = compute_meter.borrow_mut().consume(consumed_fuel);
            let after = compute_meter.borrow().get_remaining();
            ic_logger_msg!(
                log_collector,
                "Program {} consumed {} of {} compute units ({} for WASM, {} for syscalls)",
                &program_id,
                before.saturating_sub(after),
                before,
                consumed_fuel,
                before.saturating_sub(after_syscalls),
            );
            // if log_enabled!(Trace) {
            //     let mut trace_buffer = Vec::<u8>::new();
            //     let analysis =
            //         Analysis::from_executable(self.verified_executable.get_executable()).unwrap();
            //     vm.get_tracer().write(&mut trace_buffer, &analysis).unwrap();
            //     let trace_string = String::from_utf8(trace_buffer).unwrap();
            //     trace!("BPF Program Instruction Trace:\n{}", trace_string);
            // }
            // drop(vm);
            // let (_returned_from_program_id, return_data) =
            //     invoke_context.transaction_context.get_return_data();

            // if !return_data.is_empty() {
            //     stable_log::program_return(&log_collector, &program_id, return_data);
            // }
            match result {
                // Ok(status) if status != SUCCESS => {
                //     let error: InstructionError = if status == MAX_ACCOUNTS_DATA_SIZE_EXCEEDED
                //         && !invoke_context
                //             .feature_set
                //             .is_active(&cap_accounts_data_len::id())
                //     {
                //         // Until the cap_accounts_data_len feature is enabled, map the
                //         // MAX_ACCOUNTS_DATA_SIZE_EXCEEDED error to InvalidError
                //         InstructionError::InvalidError
                //     } else {
                //         status.into()
                //     };
                //     stable_log::program_failure(&log_collector, &program_id, &error);
                //     Err(error)
                // }
                // Err(error) => {
                //     let error = match error {
                //         EbpfError::UserError(BpfError::SyscallError(
                //             SyscallError::InstructionError(error),
                //         )) => error,
                //         err => {
                //             ic_logger_msg!(log_collector, "Program failed to complete: {}", err);
                //             InstructionError::ProgramFailedToComplete
                //         }
                //     };
                //     stable_log::program_failure(&log_collector, &program_id, &error);
                //     Err(error)
                // }
                Ok(0) => {
                    // To deserialize data back
                    parameter_bytes.as_slice_mut().copy_from_slice(
                        &memory.data(&store)[0..parameter_bytes_slice_len]
                    );
                    Ok(())
                },
                Ok(error_code) => {
                    let error: InstructionError = error_code.into();
                    panic!("WASM exited with error \"{error}\"");
                },
                Err(wasm_error) => {
                    match wasm_error.trap_code() {
                        Some(wasmi::core::TrapCode::OutOfFuel) => {
                            dbg!(&wasm_error);
                            // stable_log::program_failure(&log_collector, &program_id, &error);
                            Err(InstructionError::ComputationalBudgetExceeded)
                        }
                        Some(error) => {
                            dbg!(error);
                            panic!("WASM exited with error \"{wasm_error}\"");
                        }
                        None => {
                            dbg!(&wasm_error);
                            // stable_log::program_failure(&log_collector, &program_id, &error);
                            panic!("WASM exited with error \"{wasm_error}\"");
                        }
                    }
                }
            }.and(compute_meter_result)
        };
        execute_time.stop();

        if let Err(ref error) = execution_result {
            stable_log::program_failure(&log_collector, &program_id, error);
        }

        let mut deserialize_time = Measure::start("deserialize");
        // TODO: deserialize_parameters
        // let execute_or_deserialize_result = execution_result;
        let invoke_context_ref = invoke_context.borrow_mut();

        let execute_or_deserialize_result = execution_result.and_then(|_| {
            deserialize_parameters(
                invoke_context_ref.transaction_context,
                invoke_context_ref
                    .transaction_context
                    .get_current_instruction_context().inspect_err(|x| { dbg!(x); })?,
                parameter_bytes.as_slice(),
                invoke_context_ref.get_orig_account_lengths().inspect_err(|x| { dbg!(x); })?,
            ).inspect_err(|x| { dbg!(x); })
        });
        deserialize_time.stop();

        // Update the timings
        // let lock = invoke_context.lock().unwrap();
        //
        // // let timings = &mut invoke_context.timings;
        // lock.timings.serialize_us = lock.timings.serialize_us.saturating_add(serialize_time.as_us());
        // lock.timings.create_vm_us = lock.timings.create_vm_us.saturating_add(create_vm_time.as_us());
        // lock.timings.execute_us = lock.timings.execute_us.saturating_add(execute_time.as_us());
        // lock.timings.deserialize_us = lock.timings.deserialize_us
        //     .saturating_add(deserialize_time.as_us());

        if execute_or_deserialize_result.is_ok() {
            stable_log::program_success(&log_collector, &program_id);
        }
        execute_or_deserialize_result
    }
}