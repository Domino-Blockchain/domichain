use std::collections::HashMap;
use {domichain_sdk::pubkey::Pubkey, std::collections::HashSet};
use domichain_runtime::contains::Contains;

#[derive(Default, Clone)]
pub struct VoteStakeTracker {
    // Mapping from Pubkey to weight
    voted: HashMap<Pubkey, u64>,
    stake: u64,
    // Total weight
    weight: u64,
}

#[derive(Debug)]
pub struct ReachedThresholdResults {
    pub majority: bool,
    pub quorum: bool,
}

impl VoteStakeTracker {
    // Returns tuple (reached_threshold_results, is_new) where
    // Each index in `reached_threshold_results` is true if the corresponding threshold in the input
    // `thresholds_to_check` was newly reached by adding the stake of the input `vote_pubkey`
    // `is_new` is true if the vote has not been seen before
    pub fn add_vote_pubkey(
        &mut self,
        vote_pubkey: Pubkey,
        _stake: u64,
        _total_stake: u64,
        weight: u64,
        thresholds_to_check: [f64; 2],
        total_weight: u64,
    ) -> (ReachedThresholdResults, bool) {
        println!("Test vote majority {:?}, {:?}, {:?}, {:?}, {:?}", vote_pubkey, _stake, _total_stake, weight, total_weight);
        let is_new = !self.voted.contains(&vote_pubkey);
        println!("Test vote majority, is_new{:?}", is_new);
        if is_new {
            self.voted.insert(vote_pubkey, weight);
            let old_weight = self.weight;
            let new_weight = self.weight + weight;
            self.weight = new_weight;
            let check = |threshold| {
                let threshold_weight = (total_weight as f64 * threshold) as u64;
                info!("TPU: threshold_weight={threshold_weight} old_weight={old_weight} new_weight={new_weight}");
                old_weight <= threshold_weight && threshold_weight < new_weight
            };
            println!("Test vote majority, majority check{:?}, quorum check{:?}", check(thresholds_to_check[0]), check(thresholds_to_check[1]));
            (
                ReachedThresholdResults {
                    majority: check(thresholds_to_check[0]),
                    quorum: check(thresholds_to_check[1]),
                },
                is_new,
            )
        } else {
            (ReachedThresholdResults {majority: false, quorum: false}, is_new)
        }
    }

    pub fn voted(&self) -> &HashMap<Pubkey, u64> {
        &self.voted
    }

    pub fn stake(&self) -> u64 {
        self.stake
    }

    pub fn weight(&self) -> u64 {
        self.weight
    }
}

/* #[cfg(test)]
mod test {
    use {super::*, domichain_runtime::commitment::VOTE_THRESHOLD_SIZE};

    #[test]
    fn test_add_vote_pubkey() {
        let total_epoch_stake = 10;
        let mut vote_stake_tracker = VoteStakeTracker::default();
        for i in 0..10 {
            let pubkey = domichain_sdk::pubkey::new_rand();
            let (is_confirmed_thresholds, is_new) = vote_stake_tracker.add_vote_pubkey(
                pubkey,
                1,
                total_epoch_stake,
                0,
                [VOTE_THRESHOLD_SIZE, 0.0],
                3000,
            );
            let stake = vote_stake_tracker.stake();
            let (is_confirmed_thresholds2, is_new2) = vote_stake_tracker.add_vote_pubkey(
                pubkey,
                1,
                total_epoch_stake,
                0,
                [VOTE_THRESHOLD_SIZE, 0.0],
                3000,
            );
            let stake2 = vote_stake_tracker.stake();

            // Stake should not change from adding same pubkey twice
            assert_eq!(stake, stake2);
            assert!(!is_new2);

            // at i == 6, the voted stake is 70%, which is the first time crossing
            // the supermajority threshold
            if i == 6 {
                assert!(is_confirmed_thresholds[0]);
            } else {
                assert!(!is_confirmed_thresholds[0]);
            }

            // at i == 6, the voted stake is 10%, which is the first time crossing
            // the 0% threshold
            if i == 0 {
                assert!(is_confirmed_thresholds[1]);
            } else {
                assert!(!is_confirmed_thresholds[1]);
            }
            assert!(is_new);
        }
    } 
} */
