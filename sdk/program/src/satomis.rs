//! Defines the [`SatomisError`] type.

use {crate::instruction::InstructionError, thiserror::Error};

#[derive(Debug, Error)]
pub enum SatomisError {
    /// arithmetic underflowed
    #[error("Arithmetic underflowed")]
    ArithmeticUnderflow,

    /// arithmetic overflowed
    #[error("Arithmetic overflowed")]
    ArithmeticOverflow,
}

impl From<SatomisError> for InstructionError {
    fn from(error: SatomisError) -> Self {
        match error {
            SatomisError::ArithmeticOverflow => InstructionError::ArithmeticOverflow,
            SatomisError::ArithmeticUnderflow => InstructionError::ArithmeticOverflow,
        }
    }
}
