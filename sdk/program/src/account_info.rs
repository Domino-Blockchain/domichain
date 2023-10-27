//! Account information.

use std::{marker::PhantomData, mem::transmute};

use {
    crate::{
        clock::Epoch, debug_account_data::*, entrypoint::MAX_PERMITTED_DATA_INCREASE,
        program_error::ProgramError, program_memory::sol_memset, pubkey::Pubkey,
    },
    std::{
        cell::{Ref, RefCell, RefMut},
        fmt,
        rc::Rc,
        slice::from_raw_parts_mut,
    },
};

// [programs/wasm_loader/src/syscalls/cpi.rs:406] size_of::<AccountInfo>() = 48
// [programs/wasm_loader/src/syscalls/cpi.rs:407] size_of::<&Pubkey>() = 8
// [programs/wasm_loader/src/syscalls/cpi.rs:408] size_of::<Rc<RefCell<&'a mut u64>>>() = 8
// [programs/wasm_loader/src/syscalls/cpi.rs:409] size_of::<Rc<RefCell<&'a mut [u8]>>>() = 8

/// Account information
#[derive(Clone)]
#[repr(C)]
pub struct AccountInfo<'a> {
    /// Public key of the account
    pub key: &'a Pubkey,
    /// The satomis in the account.  Modifiable by programs.
    pub satomis: Rc<RefCell<&'a mut u64>>,
    /// The data held in this account.  Modifiable by programs.
    pub data: Rc<RefCell<&'a mut [u8]>>,
    /// Program that owns this account
    pub owner: &'a Pubkey,
    /// The epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
    /// Was the transaction signed by this account's public key?
    pub is_signer: bool,
    /// Is the account writable?
    pub is_writable: bool,
    /// This account's data contains a loaded program (and is now read-only)
    pub executable: bool,
}

impl<'a> AccountInfo<'a> {
    pub fn into_raw(&self) -> AccountInfoRaw<'a> {
        let satomis = self.satomis.as_ptr() as *const _;
        let satomis = unsafe { *transmute::<_, *const &u64>(satomis) };

        let ptr_to_slice = self.data.as_ptr() as *const &mut [u8];
        let slice = unsafe { *transmute::<_, *const &[u8]>(ptr_to_slice) };

        AccountInfoRaw {
            key: (self.key as *const _ as u64).try_into().unwrap(),
            satomis: (satomis as *const _ as u64).try_into().unwrap(),
            ptr_to_slice: (ptr_to_slice as usize as u64).try_into().unwrap(),
            data: (slice as *const [u8] as *const () as usize as u64).try_into().unwrap(),
            data_len: slice.len().try_into().unwrap(),
            owner: (self.owner as *const _ as u64).try_into().unwrap(),
            rent_epoch: self.rent_epoch,
            is_signer: self.is_signer,
            is_writable: self.is_writable,
            executable: self.executable,
            phantom_data: PhantomData,
        }
    }
}


/// Account information with raw mut pointers
#[derive(Clone)]
#[repr(C)]
pub struct AccountInfoRaw<'a> {
    /// Public key of the account
    pub key: u32, // &'a Pubkey,
    /// The satomis in the account.  Modifiable by programs.
    pub satomis: u32, // *mut u64,
    /// The data held in this account.  Modifiable by programs.
    pub ptr_to_slice: u32, // *const *mut [u8],
    pub data: u32, // *mut [u8],
    pub data_len: u32, // *mut usize
    /// Program that owns this account
    pub owner: u32, // &'a Pubkey,
    /// The epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
    /// Was the transaction signed by this account's public key?
    pub is_signer: bool,
    /// Is the account writable?
    pub is_writable: bool,
    /// This account's data contains a loaded program (and is now read-only)
    pub executable: bool,

    phantom_data: PhantomData<&'a Pubkey>,
}

impl<'a> fmt::Debug for AccountInfoRaw<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("AccountInfoRaw");

        f.field("key", &format_args!("{:p}", self.key as *const ()))
            .field("owner", &format_args!("{:p}", self.owner as *const ()))
            .field("is_signer", &self.is_signer)
            .field("is_writable", &self.is_writable)
            .field("executable", &self.executable)
            .field("rent_epoch", &self.rent_epoch)
            .field("satomis", &format_args!("{:p}", self.satomis as *const ()))
            .field("data.len", &self.data_len)
            .field("data", &format_args!("{:p}", self.data as *const ()))
            .field("ptr_to_slice", &format_args!("{:p}", self.ptr_to_slice as *const ()));

        f.finish_non_exhaustive()
    }
}





// [syscall] [/home/zotho/domichain_backporting/sdk/program/src/program.rs:382] size_of::<AccountInfo>() = 32
// [syscall] [/home/zotho/domichain_backporting/sdk/program/src/program.rs:383] size_of::<&Pubkey>() = 4
// [syscall] [/home/zotho/domichain_backporting/sdk/program/src/program.rs:384] size_of::<Rc<RefCell<&mut u64>>>() = 4
// [syscall] [/home/zotho/domichain_backporting/sdk/program/src/program.rs:385] size_of::<Rc<RefCell<&mut [u8]>>>() = 4


// [syscall] [/home/zotho/domichain_backporting/sdk/program/src/program.rs:382] size_of::<AccountInfoFromWasm>() = 32
// [programs/wasm_loader/src/syscalls/cpi.rs:406] size_of::<AccountInfoFromWasm>() = 32

/// Account information from WASM
#[derive(Debug, Clone)]
#[repr(C)]
pub struct AccountInfoFromWasm<'a> {
    /// Public key of the account
    pub key: u32, // &'a Pubkey
    /// The satomis in the account.  Modifiable by programs.
    pub satomis: u32, // Rc<RefCell<&'a mut u64>>
    /// The data held in this account.  Modifiable by programs.
    pub data: u32, // Rc<RefCell<&'a mut [u8]>>
    /// Program that owns this account
    pub owner: u32, // &'a Pubkey
    /// The epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
    /// Was the transaction signed by this account's public key?
    pub is_signer: bool,
    /// Is the account writable?
    pub is_writable: bool,
    /// This account's data contains a loaded program (and is now read-only)
    pub executable: bool,

    phantom_data: PhantomData<&'a Pubkey>,
}

impl<'a> AccountInfoFromWasm<'a> {
    pub fn key(&self) -> &'a Pubkey {
        unsafe { transmute(self.key as u64 as *mut Pubkey) }
    }
    pub fn satomis(&self) -> &Rc<RefCell<&'a mut u64>> {
        dbg!(self.satomis as u64 as *mut Rc<RefCell<&'a mut u64>>);
        unsafe { transmute(self.satomis as u64 as *mut Rc<RefCell<&'a mut u64>>) }
    }
    pub fn data(&self) -> &Rc<RefCell<&'a mut [u8]>> {
        unsafe { transmute(self.data as u64 as *mut Rc<RefCell<&'a mut [u8]>>) }
    }
    pub fn owner(&self) -> &'a Pubkey {
        unsafe { transmute(self.owner as u64 as *mut Pubkey) }
    }
}

impl<'a> fmt::Debug for AccountInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("AccountInfo");

        f.field("key", &self.key)
            .field("owner", &self.owner)
            .field("is_signer", &self.is_signer)
            .field("is_writable", &self.is_writable)
            .field("executable", &self.executable)
            .field("rent_epoch", &self.rent_epoch)
            .field("satomis", &self.satomis())
            .field("data.len", &self.data_len());
        debug_account_data(&self.data.borrow(), &mut f);

        f.finish_non_exhaustive()
    }
}

impl<'a> AccountInfo<'a> {
    pub fn signer_key(&self) -> Option<&Pubkey> {
        if self.is_signer {
            Some(self.key)
        } else {
            None
        }
    }

    pub fn unsigned_key(&self) -> &Pubkey {
        self.key
    }

    pub fn satomis(&self) -> u64 {
        **self.satomis.borrow()
    }

    pub fn try_satomis(&self) -> Result<u64, ProgramError> {
        Ok(**self.try_borrow_satomis()?)
    }

    /// Return the account's original data length when it was serialized for the
    /// current program invocation.
    ///
    /// # Safety
    ///
    /// This method assumes that the original data length was serialized as a u32
    /// integer in the 4 bytes immediately preceding the serialized account key.
    pub unsafe fn original_data_len(&self) -> usize {
        let key_ptr = self.key as *const _ as *const u8;
        let original_data_len_ptr = key_ptr.offset(-4) as *const u32;
        *original_data_len_ptr as usize
    }

    pub fn data_len(&self) -> usize {
        self.data.borrow().len()
    }

    pub fn try_data_len(&self) -> Result<usize, ProgramError> {
        Ok(self.try_borrow_data()?.len())
    }

    pub fn data_is_empty(&self) -> bool {
        self.data.borrow().is_empty()
    }

    pub fn try_data_is_empty(&self) -> Result<bool, ProgramError> {
        Ok(self.try_borrow_data()?.is_empty())
    }

    pub fn try_borrow_satomis(&self) -> Result<Ref<&mut u64>, ProgramError> {
        self.satomis
            .try_borrow()
            .map_err(|_| ProgramError::AccountBorrowFailed)
    }

    pub fn try_borrow_mut_satomis(&self) -> Result<RefMut<&'a mut u64>, ProgramError> {
        self.satomis
            .try_borrow_mut()
            .map_err(|_| ProgramError::AccountBorrowFailed)
    }

    pub fn try_borrow_data(&self) -> Result<Ref<&mut [u8]>, ProgramError> {
        self.data
            .try_borrow()
            .map_err(|_| ProgramError::AccountBorrowFailed)
    }

    pub fn try_borrow_mut_data(&self) -> Result<RefMut<&'a mut [u8]>, ProgramError> {
        self.data
            .try_borrow_mut()
            .map_err(|_| ProgramError::AccountBorrowFailed)
    }

    /// Realloc the account's data and optionally zero-initialize the new
    /// memory.
    ///
    /// Note:  Account data can be increased within a single call by up to
    /// `domichain_program::entrypoint::MAX_PERMITTED_DATA_INCREASE` bytes.
    ///
    /// Note: Memory used to grow is already zero-initialized upon program
    /// entrypoint and re-zeroing it wastes compute units.  If within the same
    /// call a program reallocs from larger to smaller and back to larger again
    /// the new space could contain stale data.  Pass `true` for `zero_init` in
    /// this case, otherwise compute units will be wasted re-zero-initializing.
    ///
    /// # Safety
    ///
    /// This method makes assumptions about the layout and location of memory
    /// referenced by `AccountInfo` fields. It should only be called for
    /// instances of `AccountInfo` that were created by the runtime and received
    /// in the `process_instruction` entrypoint of a program.
    pub fn realloc(&self, new_len: usize, zero_init: bool) -> Result<(), ProgramError> {
        let mut data = self.try_borrow_mut_data()?;
        let old_len = data.len();

        // Return early if length hasn't changed
        if new_len == old_len {
            return Ok(());
        }

        // Return early if the length increase from the original serialized data
        // length is too large and would result in an out of bounds allocation.
        let original_data_len = unsafe { self.original_data_len() };
        if new_len.saturating_sub(original_data_len) > MAX_PERMITTED_DATA_INCREASE {
            return Err(ProgramError::InvalidRealloc);
        }

        // realloc
        unsafe {
            let data_ptr = data.as_mut_ptr();

            // First set new length in the serialized data
            *(data_ptr.offset(-8) as *mut u64) = new_len as u64;

            // Then recreate the local slice with the new length
            *data = from_raw_parts_mut(data_ptr, new_len)
        }

        if zero_init {
            let len_increase = new_len.saturating_sub(old_len);
            if len_increase > 0 {
                sol_memset(&mut data[old_len..], 0, len_increase);
            }
        }

        Ok(())
    }

    pub fn assign(&self, new_owner: &Pubkey) {
        // Set the non-mut owner field
        unsafe {
            std::ptr::write_volatile(
                self.owner as *const Pubkey as *mut [u8; 32],
                new_owner.to_bytes(),
            );
        }
    }

    pub fn new(
        key: &'a Pubkey,
        is_signer: bool,
        is_writable: bool,
        satomis: &'a mut u64,
        data: &'a mut [u8],
        owner: &'a Pubkey,
        executable: bool,
        rent_epoch: Epoch,
    ) -> Self {
        Self {
            key,
            is_signer,
            is_writable,
            satomis: Rc::new(RefCell::new(satomis)),
            data: Rc::new(RefCell::new(data)),
            owner,
            executable,
            rent_epoch,
        }
    }

    pub fn deserialize_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, bincode::Error> {
        bincode::deserialize(&self.data.borrow())
    }

    pub fn serialize_data<T: serde::Serialize>(&self, state: &T) -> Result<(), bincode::Error> {
        if bincode::serialized_size(state)? > self.data_len() as u64 {
            return Err(Box::new(bincode::ErrorKind::SizeLimit));
        }
        bincode::serialize_into(&mut self.data.borrow_mut()[..], state)
    }
}

/// Constructs an `AccountInfo` from self, used in conversion implementations.
pub trait IntoAccountInfo<'a> {
    fn into_account_info(self) -> AccountInfo<'a>;
}
impl<'a, T: IntoAccountInfo<'a>> From<T> for AccountInfo<'a> {
    fn from(src: T) -> Self {
        src.into_account_info()
    }
}

/// Provides information required to construct an `AccountInfo`, used in
/// conversion implementations.
pub trait Account {
    fn get(&mut self) -> (&mut u64, &mut [u8], &Pubkey, bool, Epoch);
}

/// Convert (&'a Pubkey, &'a mut T) where T: Account into an `AccountInfo`
impl<'a, T: Account> IntoAccountInfo<'a> for (&'a Pubkey, &'a mut T) {
    fn into_account_info(self) -> AccountInfo<'a> {
        let (key, account) = self;
        let (satomis, data, owner, executable, rent_epoch) = account.get();
        AccountInfo::new(
            key, false, false, satomis, data, owner, executable, rent_epoch,
        )
    }
}

/// Convert (&'a Pubkey, bool, &'a mut T)  where T: Account into an
/// `AccountInfo`.
impl<'a, T: Account> IntoAccountInfo<'a> for (&'a Pubkey, bool, &'a mut T) {
    fn into_account_info(self) -> AccountInfo<'a> {
        let (key, is_signer, account) = self;
        let (satomis, data, owner, executable, rent_epoch) = account.get();
        AccountInfo::new(
            key, is_signer, false, satomis, data, owner, executable, rent_epoch,
        )
    }
}

/// Convert &'a mut (Pubkey, T) where T: Account into an `AccountInfo`.
impl<'a, T: Account> IntoAccountInfo<'a> for &'a mut (Pubkey, T) {
    fn into_account_info(self) -> AccountInfo<'a> {
        let (ref key, account) = self;
        let (satomis, data, owner, executable, rent_epoch) = account.get();
        AccountInfo::new(
            key, false, false, satomis, data, owner, executable, rent_epoch,
        )
    }
}

/// Convenience function for accessing the next item in an [`AccountInfo`]
/// iterator.
///
/// This is simply a wrapper around [`Iterator::next`] that returns a
/// [`ProgramError`] instead of an option.
///
/// # Errors
///
/// Returns [`ProgramError::NotEnoughAccountKeys`] if there are no more items in
/// the iterator.
///
/// # Examples
///
/// ```
/// use domichain_program::{
///    account_info::{AccountInfo, next_account_info},
///    entrypoint::ProgramResult,
///    pubkey::Pubkey,
/// };
/// # use domichain_program::program_error::ProgramError;
///
/// pub fn process_instruction(
///     program_id: &Pubkey,
///     accounts: &[AccountInfo],
///     instruction_data: &[u8],
/// ) -> ProgramResult {
///     let accounts_iter = &mut accounts.iter();
///     let signer = next_account_info(accounts_iter)?;
///     let payer = next_account_info(accounts_iter)?;
///
///     // do stuff ...
///
///     Ok(())
/// }
/// # let p = Pubkey::new_unique();
/// # let l = &mut 0;
/// # let d = &mut [0u8];
/// # let a = AccountInfo::new(&p, false, false, l, d, &p, false, 0);
/// # let accounts = &[a.clone(), a];
/// # process_instruction(
/// #    &Pubkey::new_unique(),
/// #    accounts,
/// #    &[],
/// # )?;
/// # Ok::<(), ProgramError>(())
/// ```
pub fn next_account_info<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
) -> Result<I::Item, ProgramError> {
    iter.next().ok_or(ProgramError::NotEnoughAccountKeys)
}

/// Convenience function for accessing multiple next items in an [`AccountInfo`]
/// iterator.
///
/// Returns a slice containing the next `count` [`AccountInfo`]s.
///
/// # Errors
///
/// Returns [`ProgramError::NotEnoughAccountKeys`] if there are not enough items
/// in the iterator to satisfy the request.
///
/// # Examples
///
/// ```
/// use domichain_program::{
///    account_info::{AccountInfo, next_account_info, next_account_infos},
///    entrypoint::ProgramResult,
///    pubkey::Pubkey,
/// };
/// # use domichain_program::program_error::ProgramError;
///
/// pub fn process_instruction(
///     program_id: &Pubkey,
///     accounts: &[AccountInfo],
///     instruction_data: &[u8],
/// ) -> ProgramResult {
///     let accounts_iter = &mut accounts.iter();
///     let signer = next_account_info(accounts_iter)?;
///     let payer = next_account_info(accounts_iter)?;
///     let outputs = next_account_infos(accounts_iter, 3)?;
///
///     // do stuff ...
///
///     Ok(())
/// }
/// # let p = Pubkey::new_unique();
/// # let l = &mut 0;
/// # let d = &mut [0u8];
/// # let a = AccountInfo::new(&p, false, false, l, d, &p, false, 0);
/// # let accounts = &[a.clone(), a.clone(), a.clone(), a.clone(), a];
/// # process_instruction(
/// #    &Pubkey::new_unique(),
/// #    accounts,
/// #    &[],
/// # )?;
/// # Ok::<(), ProgramError>(())
/// ```
pub fn next_account_infos<'a, 'b: 'a>(
    iter: &mut std::slice::Iter<'a, AccountInfo<'b>>,
    count: usize,
) -> Result<&'a [AccountInfo<'b>], ProgramError> {
    let accounts = iter.as_slice();
    if accounts.len() < count {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let (accounts, remaining) = accounts.split_at(count);
    *iter = remaining.iter();
    Ok(accounts)
}

impl<'a> AsRef<AccountInfo<'a>> for AccountInfo<'a> {
    fn as_ref(&self) -> &AccountInfo<'a> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_account_infos() {
        let k1 = Pubkey::new_unique();
        let k2 = Pubkey::new_unique();
        let k3 = Pubkey::new_unique();
        let k4 = Pubkey::new_unique();
        let k5 = Pubkey::new_unique();
        let l1 = &mut 0;
        let l2 = &mut 0;
        let l3 = &mut 0;
        let l4 = &mut 0;
        let l5 = &mut 0;
        let d1 = &mut [0u8];
        let d2 = &mut [0u8];
        let d3 = &mut [0u8];
        let d4 = &mut [0u8];
        let d5 = &mut [0u8];

        let infos = &[
            AccountInfo::new(&k1, false, false, l1, d1, &k1, false, 0),
            AccountInfo::new(&k2, false, false, l2, d2, &k2, false, 0),
            AccountInfo::new(&k3, false, false, l3, d3, &k3, false, 0),
            AccountInfo::new(&k4, false, false, l4, d4, &k4, false, 0),
            AccountInfo::new(&k5, false, false, l5, d5, &k5, false, 0),
        ];
        let infos_iter = &mut infos.iter();
        let info1 = next_account_info(infos_iter).unwrap();
        let info2_3_4 = next_account_infos(infos_iter, 3).unwrap();
        let info5 = next_account_info(infos_iter).unwrap();

        assert_eq!(k1, *info1.key);
        assert_eq!(k2, *info2_3_4[0].key);
        assert_eq!(k3, *info2_3_4[1].key);
        assert_eq!(k4, *info2_3_4[2].key);
        assert_eq!(k5, *info5.key);
    }

    #[test]
    fn test_account_info_as_ref() {
        let k = Pubkey::new_unique();
        let l = &mut 0;
        let d = &mut [0u8];
        let info = AccountInfo::new(&k, false, false, l, d, &k, false, 0);
        assert_eq!(info.key, info.as_ref().key);
    }

    #[test]
    fn test_account_info_debug_data() {
        let key = Pubkey::new_unique();
        let mut satomis = 42;
        let mut data = vec![5; 80];
        let data_str = format!("{:?}", Hex(&data[..MAX_DEBUG_ACCOUNT_DATA]));
        let info = AccountInfo::new(&key, false, false, &mut satomis, &mut data, &key, false, 0);
        assert_eq!(
            format!("{info:?}"),
            format!(
                "AccountInfo {{ \
                key: {}, \
                owner: {}, \
                is_signer: {}, \
                is_writable: {}, \
                executable: {}, \
                rent_epoch: {}, \
                satomis: {}, \
                data.len: {}, \
                data: {}, .. }}",
                key,
                key,
                false,
                false,
                false,
                0,
                satomis,
                data.len(),
                data_str,
            )
        );

        let mut data = vec![5; 40];
        let data_str = format!("{:?}", Hex(&data));
        let info = AccountInfo::new(&key, false, false, &mut satomis, &mut data, &key, false, 0);
        assert_eq!(
            format!("{info:?}"),
            format!(
                "AccountInfo {{ \
                key: {}, \
                owner: {}, \
                is_signer: {}, \
                is_writable: {}, \
                executable: {}, \
                rent_epoch: {}, \
                satomis: {}, \
                data.len: {}, \
                data: {}, .. }}",
                key,
                key,
                false,
                false,
                false,
                0,
                satomis,
                data.len(),
                data_str,
            )
        );

        let mut data = vec![];
        let info = AccountInfo::new(&key, false, false, &mut satomis, &mut data, &key, false, 0);
        assert_eq!(
            format!("{info:?}"),
            format!(
                "AccountInfo {{ \
                key: {}, \
                owner: {}, \
                is_signer: {}, \
                is_writable: {}, \
                executable: {}, \
                rent_epoch: {}, \
                satomis: {}, \
                data.len: {}, .. }}",
                key,
                key,
                false,
                false,
                false,
                0,
                satomis,
                data.len(),
            )
        );
    }
}
