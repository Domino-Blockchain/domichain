//! Definitions for the native DOMI token and its fractional satomis.

#![allow(clippy::integer_arithmetic)]

/// There are 10^9 satomis in one DOMI
pub const SATOMIS_PER_DOMI: u64 = 1_000_000_000;

/// Approximately convert fractional native tokens (satomis) into native tokens (DOMI)
pub fn satomis_to_domi(satomis: u64) -> f64 {
    satomis as f64 / SATOMIS_PER_DOMI as f64
}

/// Approximately convert native tokens (DOMI) into fractional native tokens (satomis)
pub fn domi_to_satomis(domi: f64) -> u64 {
    (domi * SATOMIS_PER_DOMI as f64) as u64
}

use std::fmt::{Debug, Display, Formatter, Result};
pub struct Domi(pub u64);

impl Domi {
    fn write_in_domi(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "◎{}.{:09}",
            self.0 / SATOMIS_PER_DOMI,
            self.0 % SATOMIS_PER_DOMI
        )
    }
}

impl Display for Domi {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.write_in_domi(f)
    }
}

impl Debug for Domi {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.write_in_domi(f)
    }
}
