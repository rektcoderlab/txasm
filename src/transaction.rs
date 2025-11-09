//! Low-level transaction builder with byte-level control
//! 
//! This module provides comprehensive transaction construction capabilities,
//! including signature handling, account management, and message compilation.

use crate::error::{Result, TxAsmError};
use crate::instruction::RawInstruction;
use crate::serialization::{
    ByteSerialize, encode_compact_u16, encode_pubkey, encode_u8, encode_u64,
    decode_compact_u16, decode_pubkey, decode_u8, decode_u64,
};
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};
use std::collections::HashMap;
use std::io::Cursor;

/// Transaction version
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionVersion {
    Legacy,
    V0,
}

/// Message header containing account metadata
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub num_required_signatures: u8,
    pub num_readonly_signed_accounts: u8,
    pub num_readonly_unsigned_accounts: u8,
}

impl MessageHeader {
    pub fn new(
        num_required_signatures: u8,
        num_readonly_signed_accounts: u8,
        num_readonly_unsigned_accounts: u8,
    ) -> Self {
        Self {
            num_required_signatures,
            num_readonly_signed_accounts,
            num_readonly_unsigned_accounts,
        }
    }
}

impl ByteSerialize for MessageHeader {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()> {
        encode_u8(self.num_required_signatures, writer)?;
        encode_u8(self.num_readonly_signed_accounts, writer)?;
        encode_u8(self.num_readonly_unsigned_accounts, writer)?;
        Ok(())
    }

    fn byte_size(&self) -> usize {
        3
    }
}

/// Compiled message ready for signing
#[derive(Debug, Clone)]
pub struct CompiledMessage {
    pub header: MessageHeader,
    pub account_keys: Vec<[u8; 32]>,
    pub recent_blockhash: [u8; 32],
    pub instructions: Vec<CompiledInstruction>,
}

/// Compiled instruction with resolved account indices
#[derive(Debug, Clone)]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

impl ByteSerialize for CompiledInstruction {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()> {
        encode_u8(self.program_id_index, writer)?;
        encode_compact_u16(self.account_indices.len() as u16, writer)?;
        for &index in &self.account_indices {
            encode_u8(index, writer)?;
        }
        encode_compact_u16(self.data.len() as u16, writer)?;
        writer.extend_from_slice(&self.data);
        Ok(())
    }

    fn byte_size(&self) -> usize {
        let accounts_len_size = if self.account_indices.len() <= 0x7f { 1 } else { 2 };
        let data_len_size = if self.data.len() <= 0x7f { 1 } else if self.data.len() <= 0x3fff { 2 } else { 3 };
        1 + accounts_len_size + self.account_indices.len() + data_len_size + self.data.len()
    }
}

impl ByteSerialize for CompiledMessage {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()> {
        // Serialize header
        self.header.serialize_bytes(writer)?;
        
        // Serialize account keys
        encode_compact_u16(self.account_keys.len() as u16, writer)?;
        for key in &self.account_keys {
            encode_pubkey(key, writer)?;
        }
        
        // Serialize recent blockhash
        encode_pubkey(&self.recent_blockhash, writer)?;
        
        // Serialize instructions
        encode_compact_u16(self.instructions.len() as u16, writer)?;
        for instruction in &self.instructions {
            instruction.serialize_bytes(writer)?;
        }
        
        Ok(())
    }

    fn byte_size(&self) -> usize {
        let keys_len_size = if self.account_keys.len() <= 0x7f { 1 } else { 2 };
        let instructions_len_size = if self.instructions.len() <= 0x7f { 1 } else { 2 };
        
        self.header.byte_size()
            + keys_len_size
            + (self.account_keys.len() * 32)
            + 32  // blockhash
            + instructions_len_size
            + self.instructions.iter().map(|i| i.byte_size()).sum::<usize>()
    }
}

/// A fully compiled transaction ready for signing and sending
#[derive(Debug, Clone)]
pub struct CompiledTransaction {
    pub message: CompiledMessage,
    pub signatures: Vec<[u8; 64]>,
}

impl CompiledTransaction {
    /// Serialize the entire transaction to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        
        // Serialize signatures
        encode_compact_u16(self.signatures.len() as u16, &mut bytes)?;
        for sig in &self.signatures {
            bytes.extend_from_slice(sig);
        }
        
        // Serialize message
        self.message.serialize_bytes(&mut bytes)?;
        
        Ok(bytes)
    }

    /// Get the serialized message (for signing)
    pub fn message_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.message.serialize_bytes(&mut bytes)?;
        Ok(bytes)
    }

    /// Calculate transaction size in bytes
    pub fn size(&self) -> usize {
        let sigs_len_size = if self.signatures.len() <= 0x7f { 1 } else { 2 };
        sigs_len_size + (self.signatures.len() * 64) + self.message.byte_size()
    }

    /// Decode a transaction from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        
        // Decode signatures
        let num_signatures = decode_compact_u16(&mut cursor)? as usize;
        let mut signatures = Vec::with_capacity(num_signatures);
        for _ in 0..num_signatures {
            let position = cursor.position() as usize;
            let data = cursor.get_ref();
            if position + 64 > data.len() {
                return Err(TxAsmError::BufferTooSmall {
                    needed: position + 64,
                    available: data.len(),
                });
            }
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&data[position..position + 64]);
            signatures.push(sig);
            cursor.set_position((position + 64) as u64);
        }
        
        // Decode message header
        let num_required_signatures = decode_u8(&mut cursor)?;
        let num_readonly_signed_accounts = decode_u8(&mut cursor)?;
        let num_readonly_unsigned_accounts = decode_u8(&mut cursor)?;
        let header = MessageHeader::new(
            num_required_signatures,
            num_readonly_signed_accounts,
            num_readonly_unsigned_accounts,
        );
        
        // Decode account keys
        let num_account_keys = decode_compact_u16(&mut cursor)? as usize;
        let mut account_keys = Vec::with_capacity(num_account_keys);
        for _ in 0..num_account_keys {
            account_keys.push(decode_pubkey(&mut cursor)?);
        }
        
        // Decode recent blockhash
        let recent_blockhash = decode_pubkey(&mut cursor)?;
        
        // Decode instructions
        let num_instructions = decode_compact_u16(&mut cursor)? as usize;
        let mut instructions = Vec::with_capacity(num_instructions);
        for _ in 0..num_instructions {
            let program_id_index = decode_u8(&mut cursor)?;
            let num_accounts = decode_compact_u16(&mut cursor)? as usize;
            let mut account_indices = Vec::with_capacity(num_accounts);
            for _ in 0..num_accounts {
                account_indices.push(decode_u8(&mut cursor)?);
            }
            let data_len = decode_compact_u16(&mut cursor)? as usize;
            let position = cursor.position() as usize;
            let data_bytes = cursor.get_ref();
            let data = data_bytes[position..position + data_len].to_vec();
            cursor.set_position((position + data_len) as u64);
            
            instructions.push(CompiledInstruction {
                program_id_index,
                account_indices,
                data,
            });
        }
        
        Ok(CompiledTransaction {
            message: CompiledMessage {
                header,
                account_keys,
                recent_blockhash,
                instructions,
            },
            signatures,
        })
    }
}

/// Transaction builder with fluent API
pub struct TransactionBuilder {
    instructions: Vec<RawInstruction>,
    payer: Option<[u8; 32]>,
    recent_blockhash: Option<[u8; 32]>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            payer: None,
            recent_blockhash: None,
        }
    }

    /// Set the fee payer
    pub fn payer(mut self, payer: [u8; 32]) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Set the fee payer from Pubkey
    pub fn payer_pubkey(mut self, payer: &Pubkey) -> Self {
        self.payer = Some(payer.to_bytes());
        self
    }

    /// Set the recent blockhash
    pub fn recent_blockhash(mut self, blockhash: [u8; 32]) -> Self {
        self.recent_blockhash = Some(blockhash);
        self
    }

    /// Set recent blockhash from Hash
    pub fn recent_blockhash_hash(mut self, blockhash: &Hash) -> Self {
        self.recent_blockhash = Some(blockhash.to_bytes());
        self
    }

    /// Add an instruction
    pub fn add_instruction(mut self, instruction: RawInstruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Add multiple instructions
    pub fn add_instructions(mut self, instructions: Vec<RawInstruction>) -> Self {
        self.instructions.extend(instructions);
        self
    }

    /// Compile the transaction into a message
    pub fn compile(self) -> Result<CompiledMessage> {
        let payer = self.payer.ok_or_else(|| {
            TxAsmError::InvalidTransaction("Payer not set".to_string())
        })?;

        let recent_blockhash = self.recent_blockhash.ok_or_else(|| {
            TxAsmError::InvalidTransaction("Recent blockhash not set".to_string())
        })?;

        if self.instructions.is_empty() {
            return Err(TxAsmError::InvalidTransaction(
                "No instructions provided".to_string(),
            ));
        }

        // Collect all unique account keys
        let mut account_keys_map: HashMap<[u8; 32], (bool, bool)> = HashMap::new();
        
        // Payer is always first and writable signer
        account_keys_map.insert(payer, (true, true));

        // Process all instructions
        for instruction in &self.instructions {
            // Add program ID as readonly
            account_keys_map
                .entry(instruction.program_id)
                .or_insert((false, false));

            for account in &instruction.accounts {
                let entry = account_keys_map.entry(account.pubkey).or_insert((false, false));
                if account.is_signer {
                    entry.0 = true;
                }
                if account.is_writable {
                    entry.1 = true;
                }
            }
        }

        // Sort accounts: writable signers, readonly signers, writable non-signers, readonly non-signers
        let mut account_keys: Vec<([u8; 32], bool, bool)> = account_keys_map
            .into_iter()
            .map(|(key, (is_signer, is_writable))| (key, is_signer, is_writable))
            .collect();

        account_keys.sort_by_key(|(key, is_signer, is_writable)| {
            let priority = match (*is_signer, *is_writable) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 3,
            };
            (priority, *key)
        });

        // Create account key index map
        let account_index_map: HashMap<[u8; 32], u8> = account_keys
            .iter()
            .enumerate()
            .map(|(i, (key, _, _))| (*key, i as u8))
            .collect();

        // Build header
        let num_writable_signers = account_keys.iter().filter(|(_, s, w)| *s && *w).count() as u8;
        let num_readonly_signers = account_keys.iter().filter(|(_, s, w)| *s && !*w).count() as u8;
        let num_readonly_unsigned = account_keys.iter().filter(|(_, s, w)| !*s && !*w).count() as u8;

        let header = MessageHeader::new(
            num_writable_signers + num_readonly_signers,
            num_readonly_signers,
            num_readonly_unsigned,
        );

        // Compile instructions
        let compiled_instructions: Vec<CompiledInstruction> = self
            .instructions
            .iter()
            .map(|instruction| {
                let program_id_index = *account_index_map
                    .get(&instruction.program_id)
                    .ok_or_else(|| {
                        TxAsmError::InvalidInstruction("Program ID not found in accounts".to_string())
                    })?;

                let account_indices: Vec<u8> = instruction
                    .accounts
                    .iter()
                    .map(|account| {
                        account_index_map.get(&account.pubkey).copied().ok_or_else(|| {
                            TxAsmError::InvalidInstruction("Account not found in accounts".to_string())
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(CompiledInstruction {
                    program_id_index,
                    account_indices,
                    data: instruction.data.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(CompiledMessage {
            header,
            account_keys: account_keys.into_iter().map(|(key, _, _)| key).collect(),
            recent_blockhash,
            instructions: compiled_instructions,
        })
    }

    /// Compile and create an unsigned transaction
    pub fn build_unsigned(self) -> Result<CompiledTransaction> {
        let message = self.compile()?;
        let num_signatures = message.header.num_required_signatures as usize;
        let signatures = vec![[0u8; 64]; num_signatures];

        Ok(CompiledTransaction { message, signatures })
    }

    /// Compile and sign the transaction
    pub fn build_and_sign(self, signers: &[&Keypair]) -> Result<CompiledTransaction> {
        let message = self.compile()?;
        let message_bytes = {
            let mut bytes = Vec::new();
            message.serialize_bytes(&mut bytes)?;
            bytes
        };

        let num_required = message.header.num_required_signatures as usize;
        if signers.len() != num_required {
            return Err(TxAsmError::SignatureError(format!(
                "Expected {} signers, got {}",
                num_required,
                signers.len()
            )));
        }

        let mut signatures = Vec::with_capacity(num_required);
        for signer in signers {
            let signature = signer.sign_message(&message_bytes);
            let sig_bytes = signature.as_ref();
            let mut sig_array = [0u8; 64];
            sig_array.copy_from_slice(sig_bytes);
            signatures.push(sig_array);
        }

        Ok(CompiledTransaction { message, signatures })
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{InstructionEncoder, AccountMeta};

    #[test]
    fn test_message_header() {
        let header = MessageHeader::new(2, 1, 3);
        let mut bytes = Vec::new();
        header.serialize_bytes(&mut bytes).unwrap();
        assert_eq!(bytes, vec![2, 1, 3]);
    }

    #[test]
    fn test_transaction_builder() {
        let payer = [1u8; 32];
        let program_id = [2u8; 32];
        let blockhash = [3u8; 32];

        let instruction = InstructionEncoder::new(program_id)
            .readonly(payer)
            .append_u8(42)
            .build();

        let builder = TransactionBuilder::new()
            .payer(payer)
            .recent_blockhash(blockhash)
            .add_instruction(instruction);

        let message = builder.compile().unwrap();
        assert_eq!(message.header.num_required_signatures, 1);
        assert!(!message.account_keys.is_empty());
    }

    #[test]
    fn test_compiled_transaction_serialization() {
        let payer = [1u8; 32];
        let program_id = [2u8; 32];
        let blockhash = [3u8; 32];

        let instruction = InstructionEncoder::new(program_id)
            .readonly(payer)
            .append_u8(100)
            .build();

        let tx = TransactionBuilder::new()
            .payer(payer)
            .recent_blockhash(blockhash)
            .add_instruction(instruction)
            .build_unsigned()
            .unwrap();

        let bytes = tx.serialize().unwrap();
        assert!(!bytes.is_empty());

        let decoded = CompiledTransaction::deserialize(&bytes).unwrap();
        assert_eq!(decoded.message.account_keys.len(), tx.message.account_keys.len());
    }
}
