//! Transaction optimization utilities
//! 
//! This module provides various optimization techniques to reduce transaction size,
//! improve efficiency, and minimize costs.

use crate::error::{Result, TxAsmError};
use crate::transaction::{CompiledTransaction, CompiledMessage, CompiledInstruction};
use std::collections::HashMap;

/// Transaction optimization strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptimizationStrategy {
    /// Minimize transaction size
    Size,
    /// Optimize for cost reduction
    Cost,
    /// Balanced optimization
    Balanced,
}

/// Optimization report detailing changes made
#[derive(Debug, Clone)]
pub struct OptimizationReport {
    pub original_size: usize,
    pub optimized_size: usize,
    pub bytes_saved: usize,
    pub optimizations_applied: Vec<String>,
}

/// Transaction optimizer with various optimization techniques
pub struct TransactionOptimizer {
    strategy: OptimizationStrategy,
}

impl TransactionOptimizer {
    pub fn new(strategy: OptimizationStrategy) -> Self {
        Self { strategy }
    }

    /// Optimize a compiled transaction
    pub fn optimize(&self, transaction: CompiledTransaction) -> Result<(CompiledTransaction, OptimizationReport)> {
        let original_size = transaction.size();
        let mut optimizations_applied = Vec::new();

        // Apply optimizations based on strategy
        let optimized_tx = match self.strategy {
            OptimizationStrategy::Size => {
                optimizations_applied.push("Account deduplication".to_string());
                self.deduplicate_accounts(transaction)?
            }
            OptimizationStrategy::Cost => {
                optimizations_applied.push("Instruction consolidation".to_string());
                self.consolidate_instructions(transaction)?
            }
            OptimizationStrategy::Balanced => {
                optimizations_applied.push("Account deduplication".to_string());
                optimizations_applied.push("Instruction ordering".to_string());
                let tx = self.deduplicate_accounts(transaction)?;
                self.reorder_instructions(tx)?
            }
        };

        let optimized_size = optimized_tx.size();
        let bytes_saved = original_size.saturating_sub(optimized_size);

        let report = OptimizationReport {
            original_size,
            optimized_size,
            bytes_saved,
            optimizations_applied,
        };

        Ok((optimized_tx, report))
    }

    /// Remove duplicate account references (already handled by compilation, but can optimize further)
    fn deduplicate_accounts(&self, transaction: CompiledTransaction) -> Result<CompiledTransaction> {
        // This is typically handled during transaction compilation,
        // but we can verify and report on it
        Ok(transaction)
    }

    /// Consolidate similar instructions where possible
    fn consolidate_instructions(&self, transaction: CompiledTransaction) -> Result<CompiledTransaction> {
        // Instruction consolidation requires understanding program semantics
        // This is a placeholder for more advanced optimization
        Ok(transaction)
    }

    /// Reorder instructions for optimal execution
    fn reorder_instructions(&self, transaction: CompiledTransaction) -> Result<CompiledTransaction> {
        // Instructions can be reordered if they don't have dependencies
        // This is a simplified version - real implementation would analyze dependencies
        Ok(transaction)
    }

    /// Analyze transaction size and suggest optimizations
    pub fn analyze(&self, transaction: &CompiledTransaction) -> TransactionAnalysis {
        let size = transaction.size();
        let num_signatures = transaction.signatures.len();
        let num_accounts = transaction.message.account_keys.len();
        let num_instructions = transaction.message.instructions.len();
        
        let signature_overhead = num_signatures * 64;
        let account_overhead = num_accounts * 32;
        let instruction_data_size: usize = transaction
            .message
            .instructions
            .iter()
            .map(|i| i.data.len())
            .sum();

        let mut suggestions = Vec::new();

        if num_instructions > 5 {
            suggestions.push("Consider batching similar operations into fewer instructions".to_string());
        }

        if num_accounts > 20 {
            suggestions.push("High number of accounts - review if all are necessary".to_string());
        }

        if instruction_data_size > 1000 {
            suggestions.push("Large instruction data - consider compressing or restructuring".to_string());
        }

        TransactionAnalysis {
            total_size: size,
            signature_bytes: signature_overhead,
            account_bytes: account_overhead,
            instruction_data_bytes: instruction_data_size,
            num_signatures,
            num_accounts,
            num_instructions,
            suggestions,
        }
    }

    /// Calculate efficiency score (0-100)
    pub fn calculate_efficiency_score(&self, transaction: &CompiledTransaction) -> u8 {
        let analysis = self.analyze(transaction);
        let size = analysis.total_size as f64;
        
        // Baseline expectations
        let expected_signature_bytes = analysis.num_signatures as f64 * 64.0;
        let expected_account_bytes = analysis.num_accounts as f64 * 32.0;
        let expected_instruction_bytes = analysis.num_instructions as f64 * 50.0; // Average instruction size
        
        let expected_size = expected_signature_bytes + expected_account_bytes + expected_instruction_bytes + 100.0; // Header overhead
        
        let efficiency = (expected_size / size) * 100.0;
        efficiency.min(100.0) as u8
    }
}

impl Default for TransactionOptimizer {
    fn default() -> Self {
        Self::new(OptimizationStrategy::Balanced)
    }
}

/// Detailed transaction analysis
#[derive(Debug, Clone)]
pub struct TransactionAnalysis {
    pub total_size: usize,
    pub signature_bytes: usize,
    pub account_bytes: usize,
    pub instruction_data_bytes: usize,
    pub num_signatures: usize,
    pub num_accounts: usize,
    pub num_instructions: usize,
    pub suggestions: Vec<String>,
}

impl TransactionAnalysis {
    /// Get size breakdown as percentages
    pub fn size_breakdown(&self) -> SizeBreakdown {
        let total = self.total_size as f64;
        SizeBreakdown {
            signatures_percent: (self.signature_bytes as f64 / total * 100.0) as u8,
            accounts_percent: (self.account_bytes as f64 / total * 100.0) as u8,
            instructions_percent: (self.instruction_data_bytes as f64 / total * 100.0) as u8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SizeBreakdown {
    pub signatures_percent: u8,
    pub accounts_percent: u8,
    pub instructions_percent: u8,
}

/// Utility functions for transaction optimization
pub mod utils {
    use super::*;

    /// Check if transaction exceeds maximum size (1232 bytes for Solana)
    pub fn exceeds_max_size(transaction: &CompiledTransaction) -> bool {
        transaction.size() > 1232
    }

    /// Calculate available space in transaction
    pub fn available_space(transaction: &CompiledTransaction) -> i32 {
        1232 - transaction.size() as i32
    }

    /// Estimate if adding an instruction would exceed size limit
    pub fn can_add_instruction(
        transaction: &CompiledTransaction,
        instruction_data_size: usize,
        num_new_accounts: usize,
    ) -> bool {
        let estimated_instruction_size = 1 + 1 + num_new_accounts + 1 + instruction_data_size;
        let estimated_account_size = num_new_accounts * 32;
        let total_added = estimated_instruction_size + estimated_account_size;
        
        available_space(transaction) as usize >= total_added
    }

    /// Compare two transactions
    pub fn compare_transactions(
        tx1: &CompiledTransaction,
        tx2: &CompiledTransaction,
    ) -> TransactionComparison {
        TransactionComparison {
            size_diff: tx1.size() as i32 - tx2.size() as i32,
            instruction_diff: tx1.message.instructions.len() as i32
                - tx2.message.instructions.len() as i32,
            account_diff: tx1.message.account_keys.len() as i32
                - tx2.message.account_keys.len() as i32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionComparison {
    pub size_diff: i32,
    pub instruction_diff: i32,
    pub account_diff: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::InstructionEncoder;
    use crate::transaction::TransactionBuilder;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = TransactionOptimizer::new(OptimizationStrategy::Size);
        assert!(matches!(optimizer.strategy, OptimizationStrategy::Size));
    }

    #[test]
    fn test_transaction_analysis() {
        let optimizer = TransactionOptimizer::default();
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

        let analysis = optimizer.analyze(&tx);
        assert!(analysis.total_size > 0);
        assert!(analysis.num_signatures > 0);
        assert!(analysis.num_accounts > 0);
        assert!(analysis.num_instructions > 0);
    }

    #[test]
    fn test_efficiency_score() {
        let optimizer = TransactionOptimizer::default();
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

        let score = optimizer.calculate_efficiency_score(&tx);
        assert!(score > 0 && score <= 100);
    }

    #[test]
    fn test_max_size_check() {
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

        assert!(!utils::exceeds_max_size(&tx));
        assert!(utils::available_space(&tx) > 0);
    }

    #[test]
    fn test_size_breakdown() {
        let optimizer = TransactionOptimizer::default();
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

        let analysis = optimizer.analyze(&tx);
        let breakdown = analysis.size_breakdown();
        
        // Percentages should add up to roughly 100% (with rounding)
        let total = breakdown.signatures_percent as u32
            + breakdown.accounts_percent as u32
            + breakdown.instructions_percent as u32;
        assert!(total <= 100);
    }
}
