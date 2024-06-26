//! Fee structures.

use crate::native_token::domi_to_satomis;

/// A fee and its associated compute unit limit
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct FeeBin {
    /// maximum compute units for which this fee will be charged
    pub limit: u64,
    /// fee in satomis
    pub fee: u64,
}

/// Information used to calculate fees
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FeeStructure {
    /// satomis per signature
    pub satomis_per_signature: u64,
    /// satomis_per_write_lock
    pub satomis_per_write_lock: u64,
    /// Compute unit fee bins
    pub compute_fee_bins: Vec<FeeBin>,
}

impl FeeStructure {
    pub fn new(
        sol_per_signature: f64,
        sol_per_write_lock: f64,
        compute_fee_bins: Vec<(u64, f64)>,
    ) -> Self {
        let compute_fee_bins = compute_fee_bins
            .iter()
            .map(|(limit, domi)| FeeBin {
                limit: *limit,
                fee: domi_to_satomis(*domi),
            })
            .collect::<Vec<_>>();
        FeeStructure {
            satomis_per_signature: domi_to_satomis(sol_per_signature),
            satomis_per_write_lock: domi_to_satomis(sol_per_write_lock),
            compute_fee_bins,
        }
    }

    pub fn get_max_fee(&self, num_signatures: u64, num_write_locks: u64) -> u64 {
        num_signatures
            .saturating_mul(self.satomis_per_signature)
            .saturating_add(num_write_locks.saturating_mul(self.satomis_per_write_lock))
            .saturating_add(
                self.compute_fee_bins
                    .last()
                    .map(|bin| bin.fee)
                    .unwrap_or_default(),
            )
    }
}

impl Default for FeeStructure {
    fn default() -> Self {
        Self::new(0.001, 0.0, vec![(1_400_000, 0.0)])
    }
}

#[cfg(RUSTC_WITH_SPECIALIZATION)]
impl ::domichain_frozen_abi::abi_example::AbiExample for FeeStructure {
    fn example() -> Self {
        FeeStructure::default()
    }
}
