//! Definitions for the native DOMI token and its fractional lamports.

#![allow(clippy::integer_arithmetic)]

/// There are 10^9 lamports in one DOMI
pub const LAMPORTS_PER_DOMI: u64 = 1_000_000_000;

/// Approximately convert fractional native tokens (lamports) into native tokens (DOMI)
pub fn lamports_to_domi(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_DOMI as f64
}

/// Approximately convert native tokens (DOMI) into fractional native tokens (lamports)
pub fn domi_to_lamports(domi: f64) -> u64 {
    (domi * LAMPORTS_PER_DOMI as f64) as u64
}

use std::fmt::{Debug, Display, Formatter, Result};
pub struct Domi(pub u64);

impl Domi {
    fn write_in_domi(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "◎{}.{:09}",
            self.0 / LAMPORTS_PER_DOMI,
            self.0 % LAMPORTS_PER_DOMI
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
