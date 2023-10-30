use {
    crate::{
        stakes::{create_and_add_stakes, StakerInfo},
        unlocks::UnlockInfo,
    },
    domichain_sdk::{genesis_config::GenesisConfig, native_token::SATOMIS_PER_DOMI},
};

// 12 month schedule is 100% after 12 months
const UNLOCKS_ALL_AT_12_MONTHS: UnlockInfo = UnlockInfo {
    cliff_fraction: 1.0,
    cliff_years: 1.0,
    unlocks: 0,
    unlock_years: 0.0,
    custodian: "9DJa2pT1Q6hrMsHrXo2S7ZEANYJeUsB9Kz9FAjbT3dma",
};

// 36 month schedule is 100% after 36 months
const UNLOCKS_ALL_AT_36_MONTHS: UnlockInfo = UnlockInfo {
    cliff_fraction: 1.0,
    cliff_years: 3.0,
    unlocks: 0,
    unlock_years: 0.0,
    custodian: "9DJa2pT1Q6hrMsHrXo2S7ZEANYJeUsB9Kz9FAjbT3dma",
};

// no lockups
const UNLOCKS_ALL_DAY_ZERO: UnlockInfo = UnlockInfo {
    cliff_fraction: 1.0,
    cliff_years: 0.0,
    unlocks: 0,
    unlock_years: 0.0,
    custodian: "9DJa2pT1Q6hrMsHrXo2S7ZEANYJeUsB9Kz9FAjbT3dma",
};

pub const INVESTORS_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "investor one",
        staker: "3133crZgMiahrNTPb59buLg4m1Np8qpSzMUiDAXscAmc",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("52ZGBKLkVBjHzaJczUxzBWEiX2XyWFMzK3KtJC4ZLRhm"),
    },
    StakerInfo {
        name: "investor two",
        staker: "Bq91HHt2wdAjfb1ZgjE6KvfH92vjnCsCJfqhakP5LmKw",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("GAJrsn2XbWpZ2gQMC6UeZPQrcTJQD1hkMqNkPAPWPU8T"),
    },
    StakerInfo {
        name: "investor three",
        staker: "5yRBR5wWvAe3ztJa3BUYsBfrFtqYTv8Xc69dDAULuKQ1",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("3PGU6uPCYFBry8MFqqZCUkezDi738pW3otbWdMcdwEry"),
    },
];

// ./target/release/domichain-keygen new --silent --no-bip39-passphrase --outfile - | ./target/release/domichain-keygen pubkey -
pub const MAINTANACE_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "maintanance one",
        staker: "A8AHHDe2imEDPoiRiL962d8VTZfrNPgzvXXhosEpnnBp",
        satomis: 40_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("Gyz2yS5MzF4PF8RYMszDAaEF7Lfs7zDAeCGvw4r238cJ"),
    },
];

pub const FOUNDATION_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "foundation one",
        staker: "3KAqEmLwmfm6YMxJ8qk4Kr5jrQHZxRiXBcmWfbuypez3",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("6q45U7aQhEkouWj8yajL7YxZHuqWFE8ALuJ5Z5JWJP4z"),
    },
];

fn add_stakes(
    genesis_config: &mut GenesisConfig,
    staker_infos: &[StakerInfo],
    unlock_info: &UnlockInfo,
) -> u64 {
    staker_infos
        .iter()
        .map(|staker_info| create_and_add_stakes(genesis_config, staker_info, unlock_info, None))
        .sum::<u64>()
}

pub fn add_genesis_accounts(genesis_config: &mut GenesisConfig, mut issued_satomis: u64) {
    // add_stakes() and add_validators() award tokens for rent exemption and
    //  to cover an initial transfer-free period of the network

    issued_satomis += add_stakes(
            genesis_config,
            INVESTORS_STAKER_INFOS,
            &UNLOCKS_ALL_DAY_ZERO,
        ) + add_stakes(
            genesis_config,
            MAINTANACE_STAKER_INFOS,
            &UNLOCKS_ALL_AT_36_MONTHS,
        ) + add_stakes(
            genesis_config,
            FOUNDATION_STAKER_INFOS,
            &UNLOCKS_ALL_AT_12_MONTHS,
        );

    // "one thanks" (maintanance pool) gets 10_000_000 DOMI - above distributions
    create_and_add_stakes(
        genesis_config,
        &StakerInfo {
            name: "maintanance two",
            staker: "CSk9RCJHL8CEEphRcZyzCKCCkGza1vmLBAJvbAMJuzL8",
            satomis: (250_000_000 * SATOMIS_PER_DOMI).saturating_sub(issued_satomis),
            withdrawer: Some("5kmYkMNcburjiihYgbb5AwLh4PXzp4r8zDo8QH9pc8Mc"),
        },
        &UNLOCKS_ALL_AT_36_MONTHS,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_genesis_accounts() {
        let mut genesis_config = GenesisConfig::default();

        add_genesis_accounts(&mut genesis_config, 0);

        let satomis = genesis_config
            .accounts
            .values()
            .map(|account| account.satomis)
            .sum::<u64>();

        assert_eq!(250_000_000 * SATOMIS_PER_DOMI, satomis);
    }
}
