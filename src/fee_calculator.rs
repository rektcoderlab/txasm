//! Priority fee calculator for Solana transactions
//! 
//! This module provides utilities for calculating and optimizing transaction fees,
//! including priority fees and compute unit optimizations.

use crate::error::{Result, TxAsmError};
use crate::transaction::CompiledTransaction;

/// Priority fee calculation strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeeStrategy {
    /// Minimal fee (most economical)
    Low,
    /// Medium priority
    Medium,
    /// High priority (fastest confirmation)
    High,
    /// Custom fee in microlamports per compute unit
    Custom(u64),
}

/// Fee estimation data
#[derive(Debug, Clone)]
pub struct FeeEstimate {
    /// Base transaction fee in lamports
    pub base_fee: u64,
    /// Priority fee in microlamports per compute unit
    pub priority_fee_per_cu: u64,
    /// Estimated compute units
    pub estimated_compute_units: u32,
    /// Total estimated cost in lamports
    pub total_cost: u64,
}

/// Priority fee calculator with various strategies
pub struct PriorityFeeCalculator {
    /// Base fee per signature (typically 5000 lamports on Solana)
    base_fee_per_signature: u64,
}

impl PriorityFeeCalculator {
    /// Create a new fee calculator with default Solana base fee
    pub fn new() -> Self {
        Self {
            base_fee_per_signature: 5000,
        }
    }

    /// Create with custom base fee
    pub fn with_base_fee(base_fee_per_signature: u64) -> Self {
        Self {
            base_fee_per_signature,
        }
    }

    /// Calculate base transaction fee based on number of signatures
    pub fn calculate_base_fee(&self, num_signatures: usize) -> u64 {
        self.base_fee_per_signature * num_signatures as u64
    }

    /// Estimate compute units based on transaction size and complexity
    /// This is a heuristic estimation - actual compute units depend on program logic
    pub fn estimate_compute_units(&self, transaction: &CompiledTransaction) -> u32 {
        let base_cu = 200; // Base compute units
        let per_instruction = 1000; // Compute units per instruction
        let per_account = 100; // Compute units per account
        let per_data_byte = 1; // Compute units per byte of instruction data

        let num_instructions = transaction.message.instructions.len() as u32;
        let num_accounts = transaction.message.account_keys.len() as u32;
        let total_data_bytes: u32 = transaction
            .message
            .instructions
            .iter()
            .map(|i| i.data.len() as u32)
            .sum();

        base_cu
            + (per_instruction * num_instructions)
            + (per_account * num_accounts)
            + (per_data_byte * total_data_bytes)
    }

    /// Calculate priority fee based on strategy
    pub fn get_priority_fee(&self, strategy: FeeStrategy) -> u64 {
        match strategy {
            FeeStrategy::Low => 1,           // 1 microlamport per CU
            FeeStrategy::Medium => 100,      // 100 microlamports per CU
            FeeStrategy::High => 1000,       // 1000 microlamports per CU
            FeeStrategy::Custom(fee) => fee,
        }
    }

    /// Calculate total fee estimate for a transaction
    pub fn estimate_fee(
        &self,
        transaction: &CompiledTransaction,
        strategy: FeeStrategy,
    ) -> FeeEstimate {
        let num_signatures = transaction.signatures.len();
        let base_fee = self.calculate_base_fee(num_signatures);
        let estimated_compute_units = self.estimate_compute_units(transaction);
        let priority_fee_per_cu = self.get_priority_fee(strategy);

        // Convert microlamports to lamports (divide by 1,000,000)
        let priority_fee_lamports =
            (estimated_compute_units as u64 * priority_fee_per_cu) / 1_000_000;
        let total_cost = base_fee + priority_fee_lamports;

        FeeEstimate {
            base_fee,
            priority_fee_per_cu,
            estimated_compute_units,
            total_cost,
        }
    }

    /// Calculate the optimal priority fee based on network conditions
    /// In a real implementation, this would query recent fee history from the network
    pub fn calculate_optimal_fee(
        &self,
        transaction: &CompiledTransaction,
        percentile: u8, // 0-100, where 50 is median, 75 is 75th percentile, etc.
    ) -> Result<FeeEstimate> {
        if percentile > 100 {
            return Err(TxAsmError::FeeCalculationError(
                "Percentile must be between 0 and 100".to_string(),
            ));
        }

        // Simulated fee calculation based on percentile
        // In production, this would use actual network data
        let strategy = match percentile {
            0..=33 => FeeStrategy::Low,
            34..=66 => FeeStrategy::Medium,
            _ => FeeStrategy::High,
        };

        Ok(self.estimate_fee(transaction, strategy))
    }

    /// Recommend fee strategy based on urgency
    pub fn recommend_strategy(&self, urgency: TransactionUrgency) -> FeeStrategy {
        match urgency {
            TransactionUrgency::NotUrgent => FeeStrategy::Low,
            TransactionUrgency::Normal => FeeStrategy::Medium,
            TransactionUrgency::Urgent => FeeStrategy::High,
            TransactionUrgency::Critical => FeeStrategy::Custom(5000),
        }
    }

    /// Calculate cost per byte for the transaction
    pub fn cost_per_byte(&self, transaction: &CompiledTransaction, strategy: FeeStrategy) -> f64 {
        let estimate = self.estimate_fee(transaction, strategy);
        let size = transaction.size() as f64;
        estimate.total_cost as f64 / size
    }

    /// Compare costs across different strategies
    pub fn compare_strategies(
        &self,
        transaction: &CompiledTransaction,
    ) -> Vec<(FeeStrategy, FeeEstimate)> {
        vec![
            (
                FeeStrategy::Low,
                self.estimate_fee(transaction, FeeStrategy::Low),
            ),
            (
                FeeStrategy::Medium,
                self.estimate_fee(transaction, FeeStrategy::Medium),
            ),
            (
                FeeStrategy::High,
                self.estimate_fee(transaction, FeeStrategy::High),
            ),
        ]
    }
}

impl Default for PriorityFeeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction urgency level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionUrgency {
    /// Can wait for next block or longer
    NotUrgent,
    /// Should be included soon
    Normal,
    /// Needs fast confirmation
    Urgent,
    /// Time-critical transaction
    Critical,
}

/// Helper function to create compute budget instructions
pub mod compute_budget {
    use super::*;

    /// Compute budget program ID
    pub const COMPUTE_BUDGET_PROGRAM_ID: [u8; 32] = [
        0x03, 0x06, 0x46, 0x6f, 0xe5, 0x21, 0x17, 0x32,
        0xff, 0xec, 0xad, 0xba, 0x72, 0xc3, 0x9b, 0xe7,
        0xbc, 0x8c, 0xe5, 0xbb, 0xc5, 0xf7, 0x12, 0x6b,
        0x2c, 0x43, 0x9b, 0x3a, 0x40, 0x00, 0x00, 0x00,
    ];

    /// Create compute unit limit instruction data
    pub fn create_compute_unit_limit_instruction(units: u32) -> Vec<u8> {
        let mut data = vec![0x02]; // SetComputeUnitLimit discriminator
        data.extend_from_slice(&units.to_le_bytes());
        data
    }

    /// Create compute unit price instruction data (priority fee)
    pub fn create_compute_unit_price_instruction(microlamports: u64) -> Vec<u8> {
        let mut data = vec![0x03]; // SetComputeUnitPrice discriminator
        data.extend_from_slice(&microlamports.to_le_bytes());
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::InstructionEncoder;
    use crate::transaction::TransactionBuilder;

    #[test]
    fn test_base_fee_calculation() {
        let calculator = PriorityFeeCalculator::new();
        assert_eq!(calculator.calculate_base_fee(1), 5000);
        assert_eq!(calculator.calculate_base_fee(2), 10000);
    }

    #[test]
    fn test_priority_fee_strategies() {
        let calculator = PriorityFeeCalculator::new();
        assert_eq!(calculator.get_priority_fee(FeeStrategy::Low), 1);
        assert_eq!(calculator.get_priority_fee(FeeStrategy::Medium), 100);
        assert_eq!(calculator.get_priority_fee(FeeStrategy::High), 1000);
        assert_eq!(calculator.get_priority_fee(FeeStrategy::Custom(500)), 500);
    }

    #[test]
    fn test_fee_estimation() {
        let calculator = PriorityFeeCalculator::new();
        let payer = [1u8; 32];
        let program_id = [2u8; 32];
        let blockhash = [3u8; 32];

        let instruction = InstructionEncoder::new(program_id)
            .readonly(payer)
            .append_u8(42)
            .build();

        let tx = TransactionBuilder::new()
            .payer(payer)
            .recent_blockhash(blockhash)
            .add_instruction(instruction)
            .build_unsigned()
            .unwrap();

        let estimate = calculator.estimate_fee(&tx, FeeStrategy::Medium);
        assert!(estimate.base_fee > 0);
        assert!(estimate.estimated_compute_units > 0);
        assert!(estimate.total_cost >= estimate.base_fee);
    }

    #[test]
    fn test_urgency_recommendations() {
        let calculator = PriorityFeeCalculator::new();
        assert_eq!(
            calculator.recommend_strategy(TransactionUrgency::NotUrgent),
            FeeStrategy::Low
        );
        assert_eq!(
            calculator.recommend_strategy(TransactionUrgency::Normal),
            FeeStrategy::Medium
        );
        assert_eq!(
            calculator.recommend_strategy(TransactionUrgency::Urgent),
            FeeStrategy::High
        );
    }

    #[test]
    fn test_compute_budget_instructions() {
        let limit_data = compute_budget::create_compute_unit_limit_instruction(200_000);
        assert_eq!(limit_data[0], 0x02);
        assert_eq!(limit_data.len(), 5);

        let price_data = compute_budget::create_compute_unit_price_instruction(1000);
        assert_eq!(price_data[0], 0x03);
        assert_eq!(price_data.len(), 9);
    }
}
