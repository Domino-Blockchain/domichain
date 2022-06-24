#![cfg(feature = "program")]

pub use domichain_program::log::*;

#[macro_export]
#[deprecated(
    since = "1.4.3",
    note = "Please use `domichain_program::log::info` instead"
)]
macro_rules! info {
    ($msg:expr) => {
        $crate::log::sol_log($msg)
    };
}
