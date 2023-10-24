use {
    crate::{
        stakes::{create_and_add_stakes, StakerInfo},
        unlocks::UnlockInfo,
    },
    domichain_sdk::{genesis_config::GenesisConfig, native_token::LAMPORTS_PER_DOMI},
};

// 3 years schedule, then monthly for 20 months (5%)
const UNLOCKS_BY_5_PERCENT_AFTER_3_YEARS: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.0,
    cliff_years: 3.0,
    unlocks: 20,
    unlock_years: 1.6666666666666665, // 1.0 / 12.0 * 20.0
    custodian: "Mc5XB47H3DKJHym5RLa9mPzWv5snERsF3KNv5AauXK8",
};

// 1 years schedule, then monthly for 20 months (5%)
const UNLOCKS_BY_5_PERCENT_AFTER_1_YEAR: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.0,
    cliff_years: 1.0,
    unlocks: 20,
    unlock_years: 1.6666666666666665, // 1.0 / 12.0 * 20.0
    custodian: "Mc5XB47H3DKJHym5RLa9mPzWv5snERsF3KNv5AauXK8",
};

// no lockups
const UNLOCKS_ALL_DAY_ZERO: UnlockInfo = UnlockInfo {
    cliff_fraction: 1.0,
    cliff_years: 0.0,
    unlocks: 0,
    unlock_years: 0.0,
    custodian: "Mc5XB47H3DKJHym5RLa9mPzWv5snERsF3KNv5AauXK8",
};

pub const INVESTORS_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "shrill charity",
        staker: "Eo1iDtrZZiAkQFA8u431hedChaSUnPbU8MWg849MFvEZ",
        lamports: 50_000_000 * LAMPORTS_PER_DOMI,
        withdrawer: Some("8CUUMKYNGxdgYio5CLHRHyzMEhhVRMcqefgE6dLqnVRK"),
    },
    StakerInfo {
        name: "legal gate",
        staker: "7KCzZCbZz6V1U1YXUpBNaqQzQCg2DKo8JsNhKASKtYxe",
        lamports: 50_000_000 * LAMPORTS_PER_DOMI,
        withdrawer: Some("92viKFftk1dJjqJwreFqT2qHXxjSUuEE9VyHvTdY1mpY"),
    },
    StakerInfo {
        name: "cluttered complaint",
        staker: "2J8mJU6tWg78DdQVEqMfpN3rMeNbcRT9qGL3yLbmSXYL",
        lamports: 50_000_000 * LAMPORTS_PER_DOMI,
        withdrawer: Some("7kgfDmgbEfypBujqn4tyApjf8H7ZWuaL3F6Ah9vQHzgR"),
    },
];

pub const MAINTANACE_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "shrill charity",
        staker: "Eo1iDtrZZiAkQFA8u431hedChaSUnPbU8MWg849MFvEZ",
        lamports: 50_000_000 * LAMPORTS_PER_DOMI,
        withdrawer: Some("8CUUMKYNGxdgYio5CLHRHyzMEhhVRMcqefgE6dLqnVRK"),
    },
];

pub const FOUNDATION_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "shrill charity",
        staker: "Eo1iDtrZZiAkQFA8u431hedChaSUnPbU8MWg849MFvEZ",
        lamports: 50_000_000 * LAMPORTS_PER_DOMI,
        withdrawer: Some("8CUUMKYNGxdgYio5CLHRHyzMEhhVRMcqefgE6dLqnVRK"),
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

pub fn add_genesis_accounts(genesis_config: &mut GenesisConfig, mut issued_lamports: u64) {
    // add_stakes() and add_validators() award tokens for rent exemption and
    //  to cover an initial transfer-free period of the network

    issued_lamports += add_stakes(
            genesis_config,
            INVESTORS_STAKER_INFOS,
            &UNLOCKS_ALL_DAY_ZERO,
        ) + add_stakes(
            genesis_config,
            MAINTANACE_STAKER_INFOS,
            &UNLOCKS_BY_5_PERCENT_AFTER_3_YEARS,
        ) + add_stakes(
            genesis_config,
            FOUNDATION_STAKER_INFOS,
            &UNLOCKS_BY_5_PERCENT_AFTER_1_YEAR,
        );

    // "one thanks" (community pool) gets 250_000_000DOMI (total) - above distributions
    create_and_add_stakes(
        genesis_config,
        &StakerInfo {
            name: "one thanks",
            staker: "7vEAL3nS9CWmy1q6njUUyHE7Cf5RmyQpND6CsoHjzPiR",
            lamports: (250_000_000 * LAMPORTS_PER_DOMI).saturating_sub(issued_lamports),
            withdrawer: Some("3FFaheyqtyAXZSYxDzsr5CVKvJuvZD1WE1VEsBtDbRqB"),
        },
        &UNLOCKS_ALL_DAY_ZERO,
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

        let lamports = genesis_config
            .accounts
            .values()
            .map(|account| account.lamports)
            .sum::<u64>();

        assert_eq!(500_000_000 * LAMPORTS_PER_DOMI, lamports);
    }
}
