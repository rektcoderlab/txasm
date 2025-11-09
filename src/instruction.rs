//! Manual instruction encoding and decoding
//! 
//! This module provides low-level control over Solana instruction construction,
//! allowing byte-by-byte manipulation of instruction data.

use crate::error::{Result, TxAsmError};
use crate::serialization::{
    ByteSerialize, encode_compact_u16, decode_compact_u16,
    encode_pubkey, decode_pubkey, encode_u8, decode_u8,
};
use solana_sdk::pubkey::Pubkey;
use std::io::Cursor;

/// Account metadata for an instruction
#[derive(Debug, Clone, PartialEq)]
pub struct AccountMeta {
    pub pubkey: [u8; 32],
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn new(pubkey: [u8; 32], is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable,
        }
    }

    pub fn new_readonly(pubkey: [u8; 32], is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }

    pub fn new_writable(pubkey: [u8; 32], is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    /// Convert from solana_sdk::Pubkey
    pub fn from_pubkey(pubkey: &Pubkey, is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey: pubkey.to_bytes(),
            is_signer,
            is_writable,
        }
    }
}

impl ByteSerialize for AccountMeta {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()> {
        encode_pubkey(&self.pubkey, writer)?;
        encode_u8(if self.is_signer { 1 } else { 0 }, writer)?;
        encode_u8(if self.is_writable { 1 } else { 0 }, writer)?;
        Ok(())
    }

    fn byte_size(&self) -> usize {
        32 + 1 + 1  // pubkey + is_signer + is_writable
    }
}

/// A raw Solana instruction with manual byte-level control
#[derive(Debug, Clone)]
pub struct RawInstruction {
    /// Program ID that this instruction invokes
    pub program_id: [u8; 32],
    /// Account keys required by this instruction
    pub accounts: Vec<AccountMeta>,
    /// Instruction data (opaque bytes)
    pub data: Vec<u8>,
}

impl RawInstruction {
    pub fn new(program_id: [u8; 32], accounts: Vec<AccountMeta>, data: Vec<u8>) -> Self {
        Self {
            program_id,
            accounts,
            data,
        }
    }

    /// Create from solana_sdk types
    pub fn from_sdk_instruction(
        program_id: &Pubkey,
        accounts: &[solana_sdk::instruction::AccountMeta],
        data: &[u8],
    ) -> Self {
        let accounts = accounts
            .iter()
            .map(|acc| AccountMeta::from_pubkey(&acc.pubkey, acc.is_signer, acc.is_writable))
            .collect();

        Self {
            program_id: program_id.to_bytes(),
            accounts,
            data: data.to_vec(),
        }
    }

    /// Get all unique account keys
    pub fn account_keys(&self) -> Vec<[u8; 32]> {
        let mut keys = vec![self.program_id];
        keys.extend(self.accounts.iter().map(|a| a.pubkey));
        keys
    }
}

impl ByteSerialize for RawInstruction {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()> {
        // Encode program ID index (will be resolved during transaction compilation)
        encode_u8(0, writer)?;
        
        // Encode accounts
        encode_compact_u16(self.accounts.len() as u16, writer)?;
        for account in &self.accounts {
            encode_u8(0, writer)?; // Account index placeholder
        }
        
        // Encode data
        encode_compact_u16(self.data.len() as u16, writer)?;
        writer.extend_from_slice(&self.data);
        
        Ok(())
    }

    fn byte_size(&self) -> usize {
        let accounts_size = 1 + self.accounts.len();
        let data_len_size = if self.data.len() <= 0x7f {
            1
        } else if self.data.len() <= 0x3fff {
            2
        } else {
            3
        };
        1 + 1 + accounts_size + data_len_size + self.data.len()
    }
}

/// High-level instruction encoder with builder pattern
pub struct InstructionEncoder {
    program_id: [u8; 32],
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
}

impl InstructionEncoder {
    pub fn new(program_id: [u8; 32]) -> Self {
        Self {
            program_id,
            accounts: Vec::new(),
            data: Vec::new(),
        }
    }

    pub fn from_pubkey(program_id: &Pubkey) -> Self {
        Self::new(program_id.to_bytes())
    }

    /// Add an account to the instruction
    pub fn account(mut self, meta: AccountMeta) -> Self {
        self.accounts.push(meta);
        self
    }

    /// Add multiple accounts
    pub fn accounts(mut self, metas: Vec<AccountMeta>) -> Self {
        self.accounts.extend(metas);
        self
    }

    /// Add a signer account
    pub fn signer(mut self, pubkey: [u8; 32], is_writable: bool) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, true, is_writable));
        self
    }

    /// Add a writable account
    pub fn writable(mut self, pubkey: [u8; 32], is_signer: bool) -> Self {
        self.accounts.push(AccountMeta::new(pubkey, is_signer, true));
        self
    }

    /// Add a readonly account
    pub fn readonly(mut self, pubkey: [u8; 32]) -> Self {
        self.accounts.push(AccountMeta::new_readonly(pubkey, false));
        self
    }

    /// Set instruction data directly
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Append bytes to instruction data
    pub fn append_data(mut self, data: &[u8]) -> Self {
        self.data.extend_from_slice(data);
        self
    }

    /// Append u8 to instruction data
    pub fn append_u8(mut self, value: u8) -> Self {
        self.data.push(value);
        self
    }

    /// Append u32 (little-endian) to instruction data
    pub fn append_u32(mut self, value: u32) -> Self {
        self.data.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Append u64 (little-endian) to instruction data
    pub fn append_u64(mut self, value: u64) -> Self {
        self.data.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Build the final instruction
    pub fn build(self) -> RawInstruction {
        RawInstruction::new(self.program_id, self.accounts, self.data)
    }

    /// Serialize directly to bytes
    pub fn serialize(self) -> Result<Vec<u8>> {
        let instruction = self.build();
        let mut bytes = Vec::new();
        instruction.serialize_bytes(&mut bytes)?;
        Ok(bytes)
    }
}

/// Instruction decoder for parsing raw instruction bytes
pub struct InstructionDecoder;

impl InstructionDecoder {
    /// Decode a raw instruction from bytes
    pub fn decode(bytes: &[u8]) -> Result<DecodedInstruction> {
        let mut cursor = Cursor::new(bytes);
        
        let program_id_index = decode_u8(&mut cursor)?;
        
        let accounts_len = decode_compact_u16(&mut cursor)?;
        let mut account_indices = Vec::with_capacity(accounts_len as usize);
        for _ in 0..accounts_len {
            account_indices.push(decode_u8(&mut cursor)?);
        }
        
        let data_len = decode_compact_u16(&mut cursor)?;
        let position = cursor.position() as usize;
        let data = bytes[position..position + data_len as usize].to_vec();
        
        Ok(DecodedInstruction {
            program_id_index,
            account_indices,
            data,
        })
    }

    /// Extract instruction discriminator (first 8 bytes of data, Anchor pattern)
    pub fn extract_discriminator(data: &[u8]) -> Option<[u8; 8]> {
        if data.len() >= 8 {
            let mut disc = [0u8; 8];
            disc.copy_from_slice(&data[0..8]);
            Some(disc)
        } else {
            None
        }
    }

    /// Parse data as borsh-serialized structure
    pub fn parse_borsh_data<T: borsh::BorshDeserialize>(data: &[u8]) -> Result<T> {
        borsh::BorshDeserialize::deserialize(&mut &data[..])
            .map_err(|e| TxAsmError::DeserializationError(e.to_string()))
    }
}

/// Decoded instruction structure
#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    pub program_id_index: u8,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

impl DecodedInstruction {
    /// Get data size in bytes
    pub fn data_size(&self) -> usize {
        self.data.len()
    }

    /// Check if instruction matches a specific discriminator
    pub fn matches_discriminator(&self, discriminator: &[u8; 8]) -> bool {
        self.data.len() >= 8 && &self.data[0..8] == discriminator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_encoder_builder() {
        let program_id = [1u8; 32];
        let account = [2u8; 32];
        
        let instruction = InstructionEncoder::new(program_id)
            .signer(account, true)
            .append_u8(42)
            .append_u64(1000)
            .build();
        
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 1);
        assert_eq!(instruction.data.len(), 9);
    }

    #[test]
    fn test_account_meta() {
        let pubkey = [3u8; 32];
        let meta = AccountMeta::new(pubkey, true, false);
        
        assert_eq!(meta.pubkey, pubkey);
        assert!(meta.is_signer);
        assert!(!meta.is_writable);
    }

    #[test]
    fn test_instruction_data_building() {
        let program_id = [0u8; 32];
        let bytes = InstructionEncoder::new(program_id)
            .append_u8(1)
            .append_u32(0x12345678)
            .append_u64(0xABCDEF0123456789)
            .serialize()
            .unwrap();
        
        assert!(!bytes.is_empty());
    }
}
