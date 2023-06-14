#![allow(clippy::integer_arithmetic)]
use {
    common::*,
    domichain_core::validator::ValidatorConfig,
    domichain_ledger::{ancestor_iterator::AncestorIterator, leader_schedule::FixedSchedule},
    domichain_local_cluster::{
        cluster::Cluster,
        local_cluster::{ClusterConfig, LocalCluster},
        validator_configs::*,
    },
    domichain_sdk::{
        clock::Slot,
        signature::{Keypair, Signer},
    },
    domichain_streamer::socket::SocketAddrSpace,
    log::*,
    serial_test::serial,
    std::{
        sync::Arc,
        thread::sleep,
        time::{Duration, Instant},
    },
};

mod common;

#[test]
#[serial]
fn test_choice_committee() {
    do_test_choice_committee();
}

fn do_test_choice_committee() {
    domichain_logger::setup_with_default(RUST_LOG_FILTER);

    // First set up the cluster with 4 nodes
    let slots_per_epoch = 2048;
    let node_stakes = vec![
        31 * DEFAULT_NODE_STAKE,
        36 * DEFAULT_NODE_STAKE,
        33 * DEFAULT_NODE_STAKE,
        0,
    ];

    let base_slot: Slot = 26; // S2
    let next_slot_on_a: Slot = 27; // S3
    let truncated_slots: Slot = 100; // just enough to purge all following slots after the S2 and S3

    let validator_keys = vec![
        "28bN3xyvrP4E8LwEgtLjhnkb7cY4amQb6DrYAbAYjgRV4GAGgkVM2K7wnxnAS7WDneuavza7x21MiafLu1HkwQt4",
        "2saHBBoTkLMmttmPQP8KfBkcCw45S5cwtV3wTdGCscRC8uxdgvHxpHiWXKx4LvJjNJtnNcbSv5NdheokFFqnNDt8",
        "4mx9yoFBeYasDKBGDWCTWGJdWuJCKbgqmuP8bN9umybCh5Jzngw7KQxe99Rf5uzfyzgba1i65rJW4Wqk7Ab5S8ye",
        "3zsEPEDsjfEay7te9XqNjRTCE7vwuT6u4DHzBJC19yp7GS8BuNRMRjnpVrKCBzb3d44kxc4KPGSHkCmk6tEfswCg",
    ]
    .iter()
    .map(|s| (Arc::new(Keypair::from_base58_string(s)), true))
    .take(node_stakes.len())
    .collect::<Vec<_>>();
    let validators = validator_keys
        .iter()
        .map(|(kp, _)| kp.pubkey())
        .collect::<Vec<_>>();
    let (validator_a_pubkey, validator_b_pubkey, validator_c_pubkey) =
        (validators[0], validators[1], validators[2]);

    let mut default_config = ValidatorConfig::default_for_test();

    let validator_to_slots = vec![
        // Ensure validator b is leader for slots <= `next_slot_on_a`
        (validator_b_pubkey, next_slot_on_a as usize + 1),
        (validator_c_pubkey, next_slot_on_a as usize + 1),
    ];

    let leader_schedule = create_custom_leader_schedule(validator_to_slots.into_iter());
    for slot in 0..=next_slot_on_a {
        assert_eq!(leader_schedule[slot], validator_b_pubkey);
    }

    default_config.fixed_leader_schedule = Some(FixedSchedule {
        leader_schedule: Arc::new(leader_schedule),
    });
    let mut validator_configs =
        make_identical_validator_configs(&default_config, node_stakes.len());

    // Disable voting on validator C
    validator_configs[2].voting_disabled = true;

    let mut config = ClusterConfig {
        cluster_lamports: DEFAULT_CLUSTER_LAMPORTS + node_stakes.iter().sum::<u64>(),
        node_stakes,
        validator_configs,
        validator_keys: Some(validator_keys),
        slots_per_epoch,
        stakers_slot_offset: slots_per_epoch,
        skip_warmup_slots: true,
        ..ClusterConfig::default()
    };
    let mut cluster = LocalCluster::new(&mut config, SocketAddrSpace::Unspecified);

    let val_a_ledger_path = cluster.ledger_path(&validator_a_pubkey);
    let val_b_ledger_path = cluster.ledger_path(&validator_b_pubkey);
    let val_c_ledger_path = cluster.ledger_path(&validator_c_pubkey);

    info!(
        "val_a {} ledger path {:?}",
        validator_a_pubkey, val_a_ledger_path
    );
    info!(
        "val_b {} ledger path {:?}",
        validator_b_pubkey, val_b_ledger_path
    );
    info!(
        "val_c {} ledger path {:?}",
        validator_c_pubkey, val_c_ledger_path
    );

    // Immediately kill validator C. No need to kill validator A because
    // 1) It has no slots in the leader schedule, so no way to make forks
    // 2) We need it to vote
    info!("Exiting validator C");
    let mut validator_c_info = cluster.exit_node(&validator_c_pubkey);

    // Step 1:
    // Let validator A, B, (D) run. Wait for both `A` and `B` to have voted on `next_slot_on_a` or
    // one of its descendants
    info!(
        "Waiting on both validators A and B to vote on fork at slot {}",
        next_slot_on_a
    );
    let now = Instant::now();
    let mut last_b_vote = 0;
    let mut last_a_vote = 0;
    loop {
        let elapsed = now.elapsed();
        assert!(
            elapsed <= Duration::from_secs(30),
            "One of the validators failed to vote on a slot >= {} in {} secs,
            last validator A vote: {},
            last validator B vote: {}",
            next_slot_on_a,
            elapsed.as_secs(),
            last_a_vote,
            last_b_vote,
        );
        sleep(Duration::from_millis(100));

        if let Some((last_vote, _)) = last_vote_in_tower(&val_b_ledger_path, &validator_b_pubkey) {
            last_b_vote = last_vote;
            if last_vote < next_slot_on_a {
                continue;
            }
        }

        if let Some((last_vote, _)) = last_vote_in_tower(&val_a_ledger_path, &validator_a_pubkey) {
            last_a_vote = last_vote;
            if last_vote >= next_slot_on_a {
                break;
            }
        }
    }

    // kill A and B
    let _validator_b_info = cluster.exit_node(&validator_b_pubkey);
    let validator_a_info = cluster.exit_node(&validator_a_pubkey);

    // Step 2:
    // Stop validator and truncate ledger, copy over B's ledger to C
    info!("Create validator C's ledger");
    {
        // first copy from validator B's ledger
        std::fs::remove_dir_all(&validator_c_info.info.ledger_path).unwrap();
        let mut opt = fs_extra::dir::CopyOptions::new();
        opt.copy_inside = true;
        fs_extra::dir::copy(&val_b_ledger_path, &val_c_ledger_path, &opt).unwrap();
        // Remove B's tower in C's new copied ledger
        remove_tower(&val_c_ledger_path, &validator_b_pubkey);

        let blockstore = open_blockstore(&val_c_ledger_path);
        purge_slots(&blockstore, base_slot + 1, truncated_slots);
    }
    info!("Create validator A's ledger");
    {
        // Find latest vote in B, and wait for it to reach blockstore
        let b_last_vote =
            wait_for_last_vote_in_tower_to_land_in_ledger(&val_b_ledger_path, &validator_b_pubkey);

        // Now we copy these blocks to A
        let b_blockstore = open_blockstore(&val_b_ledger_path);
        let a_blockstore = open_blockstore(&val_a_ledger_path);
        copy_blocks(b_last_vote, &b_blockstore, &a_blockstore);

        // Purge uneccessary slots
        purge_slots(&a_blockstore, next_slot_on_a + 1, truncated_slots);
    }

    // This should be guaranteed because we waited for validator `A` to vote on a slot > `next_slot_on_a`
    // before killing it earlier.
    info!("Checking A's tower for a vote on slot descended from slot `next_slot_on_a`");
    let last_vote_slot = last_vote_in_tower(&val_a_ledger_path, &validator_a_pubkey)
        .unwrap()
        .0;
    assert!(last_vote_slot >= next_slot_on_a);

    {
        let blockstore = open_blockstore(&val_a_ledger_path);
        purge_slots(&blockstore, next_slot_on_a + 1, truncated_slots);
    }

    // Step 3:
    // Run validator C only to make it produce and vote on its own fork.
    info!("Restart validator C again!!!");
    validator_c_info.config.voting_disabled = false;
    cluster.restart_node(
        &validator_c_pubkey,
        validator_c_info,
        SocketAddrSpace::Unspecified,
    );

    let mut votes_on_c_fork = std::collections::BTreeSet::new(); // S4 and S5
    for _ in 0..100 {
        sleep(Duration::from_millis(100));

        if let Some((last_vote, _)) = last_vote_in_tower(&val_c_ledger_path, &validator_c_pubkey) {
            if last_vote != base_slot {
                votes_on_c_fork.insert(last_vote);
                // Collect 4 votes
                if votes_on_c_fork.len() >= 4 {
                    break;
                }
            }
        }
    }
    assert!(!votes_on_c_fork.is_empty());
    info!("collected validator C's votes: {:?}", votes_on_c_fork);

    // Step 4:
    // verify whether there was violation or not
    info!("Restart validator A again!!!");
    cluster.restart_node(
        &validator_a_pubkey,
        validator_a_info,
        SocketAddrSpace::Unspecified,
    );

    // monitor for actual votes from validator A
    let mut bad_vote_detected = false;
    let mut a_votes = vec![];
    for _ in 0..100 {
        sleep(Duration::from_millis(100));

        if let Some((last_vote, _)) = last_vote_in_tower(&val_a_ledger_path, &validator_a_pubkey) {
            a_votes.push(last_vote);
            let blockstore = open_blockstore(&val_a_ledger_path);
            let mut ancestors = AncestorIterator::new(last_vote, &blockstore);
            if ancestors.any(|a| votes_on_c_fork.contains(&a)) {
                bad_vote_detected = true;
                break;
            }
        }
    }

    info!("Observed A's votes on: {:?}", a_votes);
}
