# TxAsm - Low-Level Solana Transaction Builder

Rust library for constructing Solana transactions at the byte level, providing maximum control over transaction encoding, optimization, and fee calculation.

## Features

- ** Manual Instruction Encoding/Decoding**: build instructions byte-by-byte with complete control
- ** Custom Serialization Formats**: implement custom binary formats with compact encoding
- ** Transaction Optimization**: reduce transaction size and improve efficiency
- ** Priority Fee Calculator**: calculate optimal fees with multiple strategies
- ** Transaction Analysis**: deep insights into transaction structure and costs
- ** Builder Pattern API**: fluent, ergonomic API for transaction construction
- ** Full Type Safety**: leverage Rust's type system for correctness

## Installation

Add TxAsm to your `Cargo.toml`:

```toml
[dependencies]
txasm = "0.1.0"
solana-sdk = "1.18"
```

## Quick Start

```rust
use txasm::prelude::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, hash::Hash};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a fee payer
    let payer = Keypair::new();
    let program_id = Pubkey::new_unique();
    
    // Build an instruction with byte-level control
    let instruction = InstructionEncoder::from_pubkey(&program_id)
        .signer(payer.pubkey().to_bytes(), true)
        .append_u8(0)        // Operation ID
        .append_u64(1000)    // Amount
        .build();
    
    // Construct transaction
    let transaction = TransactionBuilder::new()
        .payer_pubkey(&payer.pubkey())
        .recent_blockhash_hash(&Hash::default())
        .add_instruction(instruction)
        .build_unsigned()?;
    
    println!("Transaction size: {} bytes", transaction.size());
    Ok(())
}
```

## Core Components

### 1. Instruction Encoder

Build instructions with precise control over every byte:

```rust
use txasm::instruction::InstructionEncoder;

let instruction = InstructionEncoder::new(program_id)
    .signer(account1, true)           // Add signer account
    .writable(account2, false)        // Add writable account
    .readonly(account3)               // Add readonly account
    .append_u8(5)                     // Add u8 to data
    .append_u32(42)                   // Add u32 to data
    .append_u64(1_000_000)            // Add u64 to data
    .append_data(&custom_bytes)       // Add raw bytes
    .build();
```

### 2. Transaction Builder

Construct transactions with a fluent API:

```rust
use txasm::transaction::TransactionBuilder;

let transaction = TransactionBuilder::new()
    .payer(payer_pubkey)
    .recent_blockhash(blockhash)
    .add_instruction(instruction1)
    .add_instruction(instruction2)
    .build_unsigned()?;

// Or build and sign in one step
let signed_tx = TransactionBuilder::new()
    .payer_pubkey(&payer.pubkey())
    .recent_blockhash_hash(&blockhash)
    .add_instruction(instruction)
    .build_and_sign(&[&payer])?;
```

### 3. Fee Calculator

Calculate optimal transaction fees:

```rust
use txasm::fee_calculator::{PriorityFeeCalculator, FeeStrategy};

let calculator = PriorityFeeCalculator::new();

// Estimate fees with different strategies
let low_fee = calculator.estimate_fee(&transaction, FeeStrategy::Low);
let high_fee = calculator.estimate_fee(&transaction, FeeStrategy::High);

println!("Low priority: {} lamports", low_fee.total_cost);
println!("High priority: {} lamports", high_fee.total_cost);

// Get recommendation based on urgency
use txasm::fee_calculator::TransactionUrgency;
let strategy = calculator.recommend_strategy(TransactionUrgency::Urgent);
```

### 4. Transaction Optimizer

Analyze and optimize transactions:

```rust
use txasm::optimizer::{TransactionOptimizer, OptimizationStrategy};

let optimizer = TransactionOptimizer::new(OptimizationStrategy::Size);

// Analyze transaction
let analysis = optimizer.analyze(&transaction);
println!("Size: {} bytes", analysis.total_size);
println!("Efficiency: {}/100", optimizer.calculate_efficiency_score(&transaction));

// Get optimization suggestions
for suggestion in &analysis.suggestions {
    println!("Suggestion: {}", suggestion);
}

// Optimize transaction
let (optimized_tx, report) = optimizer.optimize(transaction)?;
println!("Saved {} bytes", report.bytes_saved);
```

### 5. Byte-Level Serialization

Low-level serialization utilities:

```rust
use txasm::serialization::*;

let mut buffer = Vec::new();

// Encode various types
encode_u8(42, &mut buffer)?;
encode_u64(1_000_000, &mut buffer)?;
encode_compact_u16(256, &mut buffer)?;
encode_pubkey(&pubkey_bytes, &mut buffer)?;

// Decode from bytes
let mut cursor = std::io::Cursor::new(&buffer);
let value = decode_u8(&mut cursor)?;
```

## Architecture

Core modules:

- **`serialization`**: Low-level byte encoding/decoding primitives
- **`instruction`**: Instruction construction and manipulation
- **`transaction`**: Transaction building and compilation
- **`fee_calculator`**: Fee estimation and priority calculation
- **`optimizer`**: Transaction analysis and optimization
- **`error`**: Comprehensive error types

## Examples

```bash
# Basic usage examples
cargo run --example basic_usage

# Advanced patterns
cargo run --example advanced_usage
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_transaction_builder
```

## Use Cases

### 1. Custom Program Interactions

Build transactions for custom Solana programs with precise control over instruction data:

```rust
let instruction = InstructionEncoder::from_pubkey(&my_program)
    .append_u8(0x01)  // Custom instruction discriminator
    .append_data(&custom_encoding)
    .build();
```

### 2. Transaction Optimization

Minimize transaction costs for high-frequency trading or batch operations:

```rust
let optimizer = TransactionOptimizer::new(OptimizationStrategy::Cost);
let (optimized, report) = optimizer.optimize(transaction)?;
```

### 3. Fee Management

Calculate optimal fees for different network conditions:

```rust
let calculator = PriorityFeeCalculator::new();
let estimates = calculator.compare_strategies(&transaction);
```

### 4. Transaction Analysis

Debug and analyze transaction structure:

```rust
let analysis = optimizer.analyze(&transaction);
let breakdown = analysis.size_breakdown();
println!("Signatures: {}%", breakdown.signatures_percent);
```

## Advanced Features

### Compute Budget Instructions

Add compute budget instructions to control CU limits and priority fees:

```rust
use txasm::fee_calculator::compute_budget::*;

let limit_data = create_compute_unit_limit_instruction(200_000);
let price_data = create_compute_unit_price_instruction(1000);

let limit_ix = InstructionEncoder::new(COMPUTE_BUDGET_PROGRAM_ID)
    .data(limit_data)
    .build();
```

### Transaction Serialization

Serialize and deserialize transactions at the byte level:

```rust
// Serialize
let bytes = transaction.serialize()?;

// Deserialize
let decoded = CompiledTransaction::deserialize(&bytes)?;

// Get message bytes for signing
let message_bytes = transaction.message_bytes()?;
```

### Custom Serialization

Implement custom serialization for your types:

```rust
use txasm::serialization::{ByteSerialize, encode_u64};

struct MyData {
    value: u64,
}

impl ByteSerialize for MyData {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()> {
        encode_u64(self.value, writer)?;
        Ok(())
    }
    
    fn byte_size(&self) -> usize {
        8
    }
}
```

## Performance Considerations

- **Zero-copy deserialization**: Efficient parsing without unnecessary allocations
- **Compact encoding**: Variable-length integer encoding reduces transaction size
- **Builder pattern**: Minimal overhead with compile-time optimizations
- **No unsafe code**: Memory-safe implementation throughout

## Comparison with Standard Library

| Feature | TxAsm | solana-sdk |
|---------|-------|------------|
| Byte-level control | ✅ Full | ❌ Limited |
| Custom serialization | ✅ Yes | ⚠️ Indirect |
| Fee optimization | ✅ Built-in | ❌ Manual |
| Transaction analysis | ✅ Comprehensive | ❌ Basic |
| Size optimization | ✅ Advanced | ❌ None |

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Format code: `cargo fmt`
6. Check for issues: `cargo clippy`
7. Submit a pull request

## License

MIT License

## Resources

- [Solana Documentation](https://docs.solana.com/)
- [Solana Transaction Format](https://docs.solana.com/developing/programming-model/transactions)
- [Examples](./examples/)

## Roadmap

- [ ] Version 0 (versioned) transaction support
- [ ] Address lookup tables integration
- [ ] Advanced optimization algorithms
- [ ] Network fee estimation integration
- [ ] SIMD instruction support
- [ ] Transaction scheduling utilities

## Support

For questions and support:
- Open an issue on GitHub
- Join our community discussions
- Check the [examples](./examples/) directory

---