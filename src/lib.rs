//! TxAsm - Low-Level Solana Transaction Builder
//! 
//! A comprehensive library for constructing Solana transactions at the byte level,
//! providing maximum control over transaction encoding, optimization, and fee calculation.

pub mod serialization;
pub mod instruction;
pub mod transaction;
pub mod fee_calculator;
pub mod optimizer;
pub mod error;

pub use error::TxAsmError;
pub use transaction::{TransactionBuilder, CompiledTransaction};
pub use instruction::{InstructionEncoder, InstructionDecoder};
pub use fee_calculator::PriorityFeeCalculator;
pub use optimizer::TransactionOptimizer;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::transaction::{TransactionBuilder, CompiledTransaction};
    pub use crate::instruction::{InstructionEncoder, InstructionDecoder};
    pub use crate::fee_calculator::PriorityFeeCalculator;
    pub use crate::optimizer::TransactionOptimizer;
    pub use crate::error::TxAsmError;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_imports() {
        // Ensure all modules are accessible
        let _ = TransactionBuilder::new();
    }
}
