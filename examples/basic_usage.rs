//! Basic usage examples for TxAsm

use txasm::prelude::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, hash::Hash, signer::Signer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TxAsm Basic Usage Examples ===\n");

    // Example 1: Building a simple transaction
    example_basic_transaction()?;

    // Example 2: Using instruction encoder
    example_instruction_encoder()?;

    // Example 3: Fee calculation
    example_fee_calculation()?;

    // Example 4: Transaction optimization
    example_optimization()?;

    // Example 5: Transaction serialization/deserialization
    example_serialization()?;

    Ok(())
}

fn example_basic_transaction() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Building a Basic Transaction");
    println!("----------------------------------------");

    // Create keypair for the fee payer
    let payer = Keypair::new();
    let program_id = Pubkey::new_unique();
    
    // Create a simple instruction
    let instruction = InstructionEncoder::from_pubkey(&program_id)
        .signer(payer.pubkey().to_bytes(), true)
        .append_u8(0) // Instruction discriminator
        .append_u64(1000) // Amount parameter
        .build();

    // Build transaction
    let recent_blockhash = Hash::default();
    let transaction = TransactionBuilder::new()
        .payer_pubkey(&payer.pubkey())
        .recent_blockhash_hash(&recent_blockhash)
        .add_instruction(instruction)
        .build_unsigned()?;

    println!("✓ Transaction created");
    println!("  Size: {} bytes", transaction.size());
    println!("  Accounts: {}", transaction.message.account_keys.len());
    println!("  Instructions: {}", transaction.message.instructions.len());
    println!();

    Ok(())
}

fn example_instruction_encoder() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Using Instruction Encoder");
    println!("------------------------------------");

    let program_id = Pubkey::new_unique();
    let account1 = Pubkey::new_unique();
    let account2 = Pubkey::new_unique();

    // Build complex instruction with multiple accounts
    let instruction = InstructionEncoder::from_pubkey(&program_id)
        .signer(account1.to_bytes(), true)
        .writable(account2.to_bytes(), false)
        .readonly([0u8; 32])
        .append_u8(5) // Instruction type
        .append_u32(42)
        .append_u64(1_000_000)
        .build();

    println!("✓ Instruction created");
    println!("  Program ID: {}", program_id);
    println!("  Accounts: {}", instruction.accounts.len());
    println!("  Data size: {} bytes", instruction.data.len());
    println!();

    Ok(())
}

fn example_fee_calculation() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Fee Calculation");
    println!("--------------------------");

    let payer = Keypair::new();
    let program_id = Pubkey::new_unique();
    let instruction = InstructionEncoder::from_pubkey(&program_id)
        .signer(payer.pubkey().to_bytes(), true)
        .append_u8(0)
        .build();

    let transaction = TransactionBuilder::new()
        .payer_pubkey(&payer.pubkey())
        .recent_blockhash(Hash::default().to_bytes())
        .add_instruction(instruction)
        .build_unsigned()?;

    // Calculate fees with different strategies
    let calculator = PriorityFeeCalculator::new();
    
    println!("Fee Estimates:");
    for (name, strategy) in [
        ("Low", txasm::fee_calculator::FeeStrategy::Low),
        ("Medium", txasm::fee_calculator::FeeStrategy::Medium),
        ("High", txasm::fee_calculator::FeeStrategy::High),
    ] {
        let estimate = calculator.estimate_fee(&transaction, strategy);
        println!("  {}: {} lamports", name, estimate.total_cost);
        println!("    Base fee: {}", estimate.base_fee);
        println!("    Priority fee: {} microlamports/CU", estimate.priority_fee_per_cu);
        println!("    Estimated CUs: {}", estimate.estimated_compute_units);
    }
    println!();

    Ok(())
}

fn example_optimization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Transaction Optimization");
    println!("------------------------------------");

    let payer = Keypair::new();
    let program_id = Pubkey::new_unique();
    
    // Create transaction with multiple instructions
    let mut builder = TransactionBuilder::new()
        .payer_pubkey(&payer.pubkey())
        .recent_blockhash(Hash::default().to_bytes());

    for i in 0..3 {
        let instruction = InstructionEncoder::from_pubkey(&program_id)
            .signer(payer.pubkey().to_bytes(), true)
            .append_u8(i)
            .build();
        builder = builder.add_instruction(instruction);
    }

    let transaction = builder.build_unsigned()?;

    // Analyze and optimize
    let optimizer = TransactionOptimizer::default();
    let analysis = optimizer.analyze(&transaction);
    
    println!("Transaction Analysis:");
    println!("  Total size: {} bytes", analysis.total_size);
    println!("  Signatures: {} ({} bytes)", analysis.num_signatures, analysis.signature_bytes);
    println!("  Accounts: {} ({} bytes)", analysis.num_accounts, analysis.account_bytes);
    println!("  Instructions: {}", analysis.num_instructions);
    println!("  Efficiency score: {}/100", optimizer.calculate_efficiency_score(&transaction));
    
    if !analysis.suggestions.is_empty() {
        println!("\n  Optimization suggestions:");
        for suggestion in &analysis.suggestions {
            println!("    - {}", suggestion);
        }
    }
    println!();

    Ok(())
}

fn example_serialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 5: Serialization & Deserialization");
    println!("------------------------------------------");

    let payer = Keypair::new();
    let program_id = Pubkey::new_unique();
    
    let instruction = InstructionEncoder::from_pubkey(&program_id)
        .signer(payer.pubkey().to_bytes(), true)
        .append_u8(42)
        .build();

    let transaction = TransactionBuilder::new()
        .payer_pubkey(&payer.pubkey())
        .recent_blockhash(Hash::default().to_bytes())
        .add_instruction(instruction)
        .build_unsigned()?;

    // Serialize
    let bytes = transaction.serialize()?;
    println!("✓ Transaction serialized: {} bytes", bytes.len());

    // Deserialize
    let deserialized = CompiledTransaction::deserialize(&bytes)?;
    println!("✓ Transaction deserialized");
    println!("  Accounts match: {}", 
        deserialized.message.account_keys.len() == transaction.message.account_keys.len());
    println!("  Instructions match: {}", 
        deserialized.message.instructions.len() == transaction.message.instructions.len());
    println!();

    Ok(())
}
