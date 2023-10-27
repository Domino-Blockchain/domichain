//! calculate and collect rent from Accounts
use domichain_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    clock::Epoch,
    epoch_schedule::EpochSchedule,
    genesis_config::GenesisConfig,
    incinerator,
    pubkey::Pubkey,
    rent::{Rent, RentDue},
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, AbiExample)]
pub struct RentCollector {
    pub epoch: Epoch,
    pub epoch_schedule: EpochSchedule,
    pub slots_per_year: f64,
    pub rent: Rent,
}

impl Default for RentCollector {
    fn default() -> Self {
        Self {
            epoch: Epoch::default(),
            epoch_schedule: EpochSchedule::default(),
            // derive default value using GenesisConfig::default()
            slots_per_year: GenesisConfig::default().slots_per_year(),
            rent: Rent::default(),
        }
    }
}

/// When rent is collected from an exempt account, rent_epoch is set to this
/// value. The idea is to have a fixed, consistent value for rent_epoch for all accounts that do not collect rent.
/// This enables us to get rid of the field completely.
pub const RENT_EXEMPT_RENT_EPOCH: Epoch = Epoch::MAX;

/// when rent is collected for this account, this is the action to apply to the account
#[derive(Debug)]
enum RentResult {
    /// this account will never have rent collected from it
    Exempt,
    /// maybe we collect rent later, but not now
    NoRentCollectionNow,
    /// collect rent
    CollectRent {
        new_rent_epoch: Epoch,
        rent_due: u64, // satomis, could be 0
    },
}

impl RentCollector {
    pub(crate) fn new(
        epoch: Epoch,
        epoch_schedule: EpochSchedule,
        slots_per_year: f64,
        rent: Rent,
    ) -> Self {
        Self {
            epoch,
            epoch_schedule,
            slots_per_year,
            rent,
        }
    }

    pub(crate) fn clone_with_epoch(&self, epoch: Epoch) -> Self {
        Self {
            epoch,
            ..self.clone()
        }
    }

    /// true if it is easy to determine this account should consider having rent collected from it
    pub(crate) fn should_collect_rent(
        &self,
        address: &Pubkey,
        account: &impl ReadableAccount,
    ) -> bool {
        !(account.executable() // executable accounts must be rent-exempt balance
            || *address == incinerator::id())
    }

    /// given an account that 'should_collect_rent'
    /// returns (amount rent due, is_exempt_from_rent)
    pub(crate) fn get_rent_due(&self, account: &impl ReadableAccount) -> RentDue {
        if self
            .rent
            .is_exempt(account.satomis(), account.data().len())
        {
            RentDue::Exempt
        } else {
            let account_rent_epoch = account.rent_epoch();
            let slots_elapsed: u64 = (account_rent_epoch..=self.epoch)
                .map(|epoch| self.epoch_schedule.get_slots_in_epoch(epoch + 1))
                .sum();

            // avoid infinite rent in rust 1.45
            let years_elapsed = if self.slots_per_year != 0.0 {
                slots_elapsed as f64 / self.slots_per_year
            } else {
                0.0
            };

            // we know this account is not exempt
            let due = self.rent.due_amount(account.data().len(), years_elapsed);
            RentDue::Paying(due)
        }
    }

    // Updates the account's satomis and status, and returns the amount of rent collected, if any.
    // This is NOT thread safe at some level. If we try to collect from the same account in
    // parallel, we may collect twice.
    #[must_use = "add to Bank::collected_rent"]
    pub(crate) fn collect_from_existing_account(
        &self,
        address: &Pubkey,
        account: &mut AccountSharedData,
        filler_account_suffix: Option<&Pubkey>,
        set_exempt_rent_epoch_max: bool,
    ) -> CollectedInfo {
        match self.calculate_rent_result(address, account, filler_account_suffix) {
            RentResult::Exempt => {
                if set_exempt_rent_epoch_max {
                    account.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                }
                CollectedInfo::default()
            }
            RentResult::NoRentCollectionNow => CollectedInfo::default(),
            RentResult::CollectRent {
                new_rent_epoch,
                rent_due,
            } => match account.satomis().checked_sub(rent_due) {
                None | Some(0) => {
                    let account = std::mem::take(account);
                    CollectedInfo {
                        rent_amount: account.satomis(),
                        account_data_len_reclaimed: account.data().len() as u64,
                    }
                }
                Some(satomis) => {
                    account.set_satomis(satomis);
                    account.set_rent_epoch(new_rent_epoch);
                    CollectedInfo {
                        rent_amount: rent_due,
                        account_data_len_reclaimed: 0u64,
                    }
                }
            },
        }
    }

    /// determine what should happen to collect rent from this account
    #[must_use]
    fn calculate_rent_result(
        &self,
        address: &Pubkey,
        account: &impl ReadableAccount,
        filler_account_suffix: Option<&Pubkey>,
    ) -> RentResult {
        if account.rent_epoch() == RENT_EXEMPT_RENT_EPOCH || account.rent_epoch() > self.epoch {
            // potentially rent paying account (or known and already marked exempt)
            // Maybe collect rent later, leave account alone for now.
            return RentResult::NoRentCollectionNow;
        }
        if !self.should_collect_rent(address, account)
            || crate::accounts_db::AccountsDb::is_filler_account_helper(
                address,
                filler_account_suffix,
            )
        {
            // easy to determine this account should not consider having rent collected from it
            return RentResult::Exempt;
        }
        match self.get_rent_due(account) {
            // account will not have rent collected ever
            RentDue::Exempt => RentResult::Exempt,
            // potentially rent paying account
            // Maybe collect rent later, leave account alone for now.
            RentDue::Paying(0) => RentResult::NoRentCollectionNow,
            // Rent is collected for next epoch.
            RentDue::Paying(rent_due) => RentResult::CollectRent {
                new_rent_epoch: self.epoch + 1,
                rent_due,
            },
        }
    }
}

/// Information computed during rent collection
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub(crate) struct CollectedInfo {
    /// Amount of rent collected from account
    pub(crate) rent_amount: u64,
    /// Size of data reclaimed from account (happens when account's satomis go to zero)
    pub(crate) account_data_len_reclaimed: u64,
}

impl std::ops::Add for CollectedInfo {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            rent_amount: self.rent_amount + other.rent_amount,
            account_data_len_reclaimed: self.account_data_len_reclaimed
                + other.account_data_len_reclaimed,
        }
    }
}

impl std::ops::AddAssign for CollectedInfo {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        domichain_sdk::{account::Account, sysvar},
    };

    fn default_rent_collector_clone_with_epoch(epoch: Epoch) -> RentCollector {
        RentCollector::default().clone_with_epoch(epoch)
    }

    impl RentCollector {
        #[must_use = "add to Bank::collected_rent"]
        fn collect_from_created_account(
            &self,
            address: &Pubkey,
            account: &mut AccountSharedData,
            set_exempt_rent_epoch_max: bool,
        ) -> CollectedInfo {
            // initialize rent_epoch as created at this epoch
            account.set_rent_epoch(self.epoch);
            self.collect_from_existing_account(
                address,
                account,
                /*filler_account_suffix:*/ None,
                set_exempt_rent_epoch_max,
            )
        }
    }

    #[test]
    fn test_calculate_rent_result() {
        for set_exempt_rent_epoch_max in [false, true] {
            let mut rent_collector = RentCollector::default();

            let mut account = AccountSharedData::default();
            assert!(matches!(
                rent_collector.calculate_rent_result(&Pubkey::default(), &account, None,),
                RentResult::NoRentCollectionNow,
            ));
            {
                let mut account_clone = account.clone();
                assert_eq!(
                    rent_collector.collect_from_existing_account(
                        &Pubkey::default(),
                        &mut account_clone,
                        None,
                        set_exempt_rent_epoch_max
                    ),
                    CollectedInfo::default()
                );
                assert_eq!(account_clone, account);
            }

            account.set_executable(true);
            assert!(matches!(
                rent_collector.calculate_rent_result(&Pubkey::default(), &account, None,),
                RentResult::Exempt
            ));
            {
                let mut account_clone = account.clone();
                let mut account_expected = account.clone();
                if set_exempt_rent_epoch_max {
                    account_expected.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                }
                assert_eq!(
                    rent_collector.collect_from_existing_account(
                        &Pubkey::default(),
                        &mut account_clone,
                        None,
                        set_exempt_rent_epoch_max
                    ),
                    CollectedInfo::default()
                );
                assert_eq!(account_clone, account_expected);
            }

            account.set_executable(false);
            assert!(matches!(
                rent_collector.calculate_rent_result(&incinerator::id(), &account, None,),
                RentResult::Exempt
            ));
            {
                let mut account_clone = account.clone();
                let mut account_expected = account.clone();
                if set_exempt_rent_epoch_max {
                    account_expected.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                }
                assert_eq!(
                    rent_collector.collect_from_existing_account(
                        &incinerator::id(),
                        &mut account_clone,
                        None,
                        set_exempt_rent_epoch_max
                    ),
                    CollectedInfo::default()
                );
                assert_eq!(account_clone, account_expected);
            }

            // try a few combinations of rent collector rent epoch and collecting rent with and without filler accounts specified (but we aren't a filler)
            let filler_account = domichain_sdk::pubkey::new_rand();

            for filler_accounts in [None, Some(&filler_account)] {
                for (rent_epoch, rent_due_expected) in [(2, 2), (3, 5)] {
                    rent_collector.epoch = rent_epoch;
                    account.set_satomis(10);
                    account.set_rent_epoch(1);
                    let new_rent_epoch_expected = rent_collector.epoch + 1;
                    assert!(
                        matches!(
                            rent_collector.calculate_rent_result(&Pubkey::default(), &account, filler_accounts),
                            RentResult::CollectRent{ new_rent_epoch, rent_due} if new_rent_epoch == new_rent_epoch_expected && rent_due == rent_due_expected,
                        ),
                        "{:?}",
                        rent_collector.calculate_rent_result(&Pubkey::default(), &account, None,)
                    );

                    {
                        let mut account_clone = account.clone();
                        assert_eq!(
                            rent_collector.collect_from_existing_account(
                                &Pubkey::default(),
                                &mut account_clone,
                                filler_accounts,
                                set_exempt_rent_epoch_max
                            ),
                            CollectedInfo {
                                rent_amount: rent_due_expected,
                                account_data_len_reclaimed: 0
                            }
                        );
                        let mut account_expected = account.clone();
                        account_expected.set_satomis(account.satomis() - rent_due_expected);
                        account_expected.set_rent_epoch(new_rent_epoch_expected);
                        assert_eq!(account_clone, account_expected);
                    }
                }
            }

            // enough satomis to make us exempt
            account.set_satomis(1_000_000);
            let result = rent_collector.calculate_rent_result(&Pubkey::default(), &account, None);
            assert!(
                matches!(result, RentResult::Exempt),
                "{result:?}, set_exempt_rent_epoch_max: {set_exempt_rent_epoch_max}",
            );
            {
                let mut account_clone = account.clone();
                let mut account_expected = account.clone();
                if set_exempt_rent_epoch_max {
                    account_expected.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                }
                assert_eq!(
                    rent_collector.collect_from_existing_account(
                        &Pubkey::default(),
                        &mut account_clone,
                        None,
                        set_exempt_rent_epoch_max
                    ),
                    CollectedInfo::default()
                );
                assert_eq!(account_clone, account_expected);
            }

            // enough satomis to make us exempt
            // but, our rent_epoch is set in the future, so we can't know if we are exempt yet or not.
            // We don't calculate rent amount vs data if the rent_epoch is already in the future.
            account.set_rent_epoch(1_000_000);
            assert!(matches!(
                rent_collector.calculate_rent_result(&Pubkey::default(), &account, None,),
                RentResult::NoRentCollectionNow,
            ));
            {
                let mut account_clone = account.clone();
                assert_eq!(
                    rent_collector.collect_from_existing_account(
                        &Pubkey::default(),
                        &mut account_clone,
                        None,
                        set_exempt_rent_epoch_max
                    ),
                    CollectedInfo::default()
                );
                assert_eq!(account_clone, account);
            }

            // filler accounts are exempt
            account.set_rent_epoch(1);
            account.set_satomis(10);
            assert!(matches!(
                rent_collector.calculate_rent_result(
                    &filler_account,
                    &account,
                    Some(&filler_account),
                ),
                RentResult::Exempt,
            ));
            {
                let mut account_clone = account.clone();
                let mut account_expected = account.clone();
                if set_exempt_rent_epoch_max {
                    account_expected.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                }
                assert_eq!(
                    rent_collector.collect_from_existing_account(
                        &filler_account,
                        &mut account_clone,
                        Some(&filler_account),
                        set_exempt_rent_epoch_max
                    ),
                    CollectedInfo::default()
                );
                assert_eq!(account_clone, account_expected);
            }
        }
    }

    #[test]
    fn test_collect_from_account_created_and_existing() {
        for set_exempt_rent_epoch_max in [false, true] {
            let old_satomis = 1000;
            let old_epoch = 1;
            let new_epoch = 2;

            let (mut created_account, mut existing_account) = {
                let account = AccountSharedData::from(Account {
                    satomis: old_satomis,
                    rent_epoch: old_epoch,
                    ..Account::default()
                });

                (account.clone(), account)
            };

            let rent_collector = default_rent_collector_clone_with_epoch(new_epoch);

            // collect rent on a newly-created account
            let collected = rent_collector.collect_from_created_account(
                &domichain_sdk::pubkey::new_rand(),
                &mut created_account,
                set_exempt_rent_epoch_max,
            );
            assert!(created_account.satomis() < old_satomis);
            assert_eq!(
                created_account.satomis() + collected.rent_amount,
                old_satomis
            );
            assert_ne!(created_account.rent_epoch(), old_epoch);
            assert_eq!(collected.account_data_len_reclaimed, 0);

            // collect rent on a already-existing account
            let collected = rent_collector.collect_from_existing_account(
                &domichain_sdk::pubkey::new_rand(),
                &mut existing_account,
                None, // filler_account_suffix
                set_exempt_rent_epoch_max,
            );
            assert!(existing_account.satomis() < old_satomis);
            assert_eq!(
                existing_account.satomis() + collected.rent_amount,
                old_satomis
            );
            assert_ne!(existing_account.rent_epoch(), old_epoch);
            assert_eq!(collected.account_data_len_reclaimed, 0);

            // newly created account should be collected for less rent; thus more remaining balance
            assert!(created_account.satomis() > existing_account.satomis());
            assert_eq!(created_account.rent_epoch(), existing_account.rent_epoch());
        }
    }

    #[test]
    fn test_rent_exempt_temporal_escape() {
        for set_exempt_rent_epoch_max in [false, true] {
            for pass in 0..2 {
                let mut account = AccountSharedData::default();
                let epoch = 3;
                let huge_satomis = 123_456_789_012;
                let tiny_satomis = 789_012;
                let pubkey = domichain_sdk::pubkey::new_rand();

                assert_eq!(account.rent_epoch(), 0);

                // create a tested rent collector
                let rent_collector = default_rent_collector_clone_with_epoch(epoch);

                if pass == 0 {
                    account.set_satomis(huge_satomis);
                    // first mark account as being collected while being rent-exempt
                    let collected = rent_collector.collect_from_existing_account(
                        &pubkey,
                        &mut account,
                        None, // filler_account_suffix
                        set_exempt_rent_epoch_max,
                    );
                    assert_eq!(account.satomis(), huge_satomis);
                    assert_eq!(collected, CollectedInfo::default());
                    continue;
                }

                // decrease the balance not to be rent-exempt
                // In a real validator, it is not legal to reduce an account's satomis such that the account becomes rent paying.
                // So, pass == 0 above tests the case of rent that is exempt. pass == 1 tests the case where we are rent paying.
                account.set_satomis(tiny_satomis);

                // ... and trigger another rent collection on the same epoch and check that rent is working
                let collected = rent_collector.collect_from_existing_account(
                    &pubkey,
                    &mut account,
                    None, // filler_account_suffix
                    set_exempt_rent_epoch_max,
                );
                assert_eq!(account.satomis(), tiny_satomis - collected.rent_amount);
                assert_ne!(collected, CollectedInfo::default());
            }
        }
    }

    #[test]
    fn test_rent_exempt_sysvar() {
        for set_exempt_rent_epoch_max in [false, true] {
            let tiny_satomis = 1;
            let mut account = AccountSharedData::default();
            account.set_owner(sysvar::id());
            account.set_satomis(tiny_satomis);

            let pubkey = domichain_sdk::pubkey::new_rand();

            assert_eq!(account.rent_epoch(), 0);

            let epoch = 3;
            let rent_collector = default_rent_collector_clone_with_epoch(epoch);

            let collected = rent_collector.collect_from_existing_account(
                &pubkey,
                &mut account,
                None, // filler_account_suffix
                set_exempt_rent_epoch_max,
            );
            assert_eq!(account.satomis(), 0);
            assert_eq!(collected.rent_amount, 1);
        }
    }

    /// Ensure that when an account is "rent collected" away, its data len is returned.
    #[test]
    fn test_collect_cleans_up_account() {
        for set_exempt_rent_epoch_max in [false, true] {
            domichain_logger::setup();
            let account_satomis = 1; // must be *below* rent amount
            let account_data_len = 567;
            let account_rent_epoch = 11;
            let mut account = AccountSharedData::from(Account {
                satomis: account_satomis, // <-- must be below rent-exempt amount
                data: vec![u8::default(); account_data_len],
                rent_epoch: account_rent_epoch,
                ..Account::default()
            });
            let rent_collector = default_rent_collector_clone_with_epoch(account_rent_epoch + 1);

            let collected = rent_collector.collect_from_existing_account(
                &Pubkey::new_unique(),
                &mut account,
                None, // filler_account_suffix
                set_exempt_rent_epoch_max,
            );

            assert_eq!(collected.rent_amount, account_satomis);
            assert_eq!(
                collected.account_data_len_reclaimed,
                account_data_len as u64
            );
            assert_eq!(account, AccountSharedData::default());
        }
    }
}
