use {
    crate::{
        stakes::{create_and_add_stakes, StakerInfo},
        unlocks::UnlockInfo,
    },
    domichain_sdk::{genesis_config::GenesisConfig, native_token::SATOMIS_PER_DOMI},
};

// 3 years schedule, then monthly for 20 months (by 5%)
const UNLOCKS_BY_5_PERCENT_AFTER_3_YEARS: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.05, // 1.0 / 20.0
    cliff_years: 3.0,
    unlocks: 19,
    unlock_years: 1.5833333333333333, // 1.0 / 12.0 * 19.0
    custodian: "Mc5XB47H3DKJHym5RLa9mPzWv5snERsF3KNv5AauXK8",
};

// 1 years schedule, then monthly for 20 months (by 5%)
const UNLOCKS_BY_5_PERCENT_AFTER_1_YEAR: UnlockInfo = UnlockInfo {
    cliff_fraction: 0.05, // 1.0 / 20.0
    cliff_years: 1.0,
    unlocks: 19,
    unlock_years: 1.5833333333333333, // 1.0 / 12.0 * 19.0
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
        staker: "3Y8Gfv4bpqVh15Y71Zy6a1ezvKfLkoDHYJZ8CKac74gy",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("ACkRSLK6xsHXbXT2Zr5DnyhmCLEjg1aXwKjc6GMExy77"),
    },
    StakerInfo {
        name: "legal gate",
        staker: "FDSASFuLNtjxk5cEpNvcDnKa8VijjpFYppGsK4GDQoaZ",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("EMJjT9WuT2mzEuVGQhamzZskF7P12BV8a2Qghssa3byc"),
    },
    StakerInfo {
        name: "cluttered complaint",
        staker: "A4bpL66yo47EXs5civo1o2Kn5ohQfVCM4oK1FdzA77L5",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("9i11wBcKUNui9bq188s7NedF4dc9bNmheT8X7GyoPEkV"),
    },
];

// ./target/release/domichain-keygen new --silent --no-bip39-passphrase --outfile - | ./target/release/domichain-keygen pubkey -
pub const MAINTANACE_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "unbecoming silver",
        staker: "4jyUPfUU4nA59E7pkran1vkcLkN6dRAJTNmSPbEuD6GL",
        satomis: 40_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("2Mk89SdDymFcxEDFZVwU2zMgz1wP2vri8iuFeAieN3ET"),
    },
];

pub const FOUNDATION_STAKER_INFOS: &[StakerInfo] = &[
    StakerInfo {
        name: "shrill charity",
        staker: "JBPTHdeVwbZsx4yZ9RMcPkojjAnd318HvmW2WVriehkW",
        satomis: 50_000_000 * SATOMIS_PER_DOMI,
        withdrawer: Some("CUpX8BoNMoHfTJGsPaFjiGExMhShwD8ALtefVFfC3tPr"),
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
            &UNLOCKS_BY_5_PERCENT_AFTER_3_YEARS,
        ) + add_stakes(
            genesis_config,
            FOUNDATION_STAKER_INFOS,
            &UNLOCKS_BY_5_PERCENT_AFTER_1_YEAR,
        );

    // "one thanks" (maintanance pool) gets 10_000_000 DOMI - above distributions
    create_and_add_stakes(
        genesis_config,
        &StakerInfo {
            name: "one thanks",
            staker: "Ay9XxmEc3YZy5PMxWTuNqRCyu9HiJ6QKqp2rAfqBrtiH",
            satomis: (250_000_000 * SATOMIS_PER_DOMI).saturating_sub(issued_satomis),
            withdrawer: Some("9He2rQZWGobpx65LdaUuU2u6Dkb5y4EoC5oV16P5B3Go"),
        },
        &UNLOCKS_BY_5_PERCENT_AFTER_3_YEARS,
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
