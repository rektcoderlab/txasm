//! Low-level byte serialization utilities for Solana transactions
//! 
//! This module provides manual byte-level encoding and decoding capabilities,
//! giving you complete control over the binary format of transactions.

use crate::error::{Result, TxAsmError};
use std::io::{Cursor, Write};

/// Trait for types that can be serialized at the byte level
pub trait ByteSerialize {
    fn serialize_bytes(&self, writer: &mut Vec<u8>) -> Result<()>;
    fn byte_size(&self) -> usize;
}

/// Trait for types that can be deserialized from bytes
pub trait ByteDeserialize: Sized {
    fn deserialize_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self>;
}

/// Compact-u16 encoding (variable-length encoding used by Solana)
pub fn encode_compact_u16(value: u16, writer: &mut Vec<u8>) -> Result<()> {
    if value <= 0x7f {
        writer.write_all(&[value as u8])?;
    } else if value <= 0x3fff {
        writer.write_all(&[
            ((value & 0x7f) | 0x80) as u8,
            (value >> 7) as u8,
        ])?;
    } else {
        writer.write_all(&[
            ((value & 0x7f) | 0x80) as u8,
            (((value >> 7) & 0x7f) | 0x80) as u8,
            (value >> 14) as u8,
        ])?;
    }
    Ok(())
}

/// Decode compact-u16
pub fn decode_compact_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16> {
    let mut value: u16 = 0;
    let mut shift = 0;

    for i in 0..3 {
        let position = cursor.position() as usize;
        let data = cursor.get_ref();
        
        if position >= data.len() {
            return Err(TxAsmError::DeserializationError(
                "Unexpected end of buffer".to_string()
            ));
        }

        let byte = data[position];
        cursor.set_position(position as u64 + 1);

        value |= ((byte & 0x7f) as u16) << shift;

        if byte & 0x80 == 0 {
            return Ok(value);
        }

        shift += 7;
    }

    Err(TxAsmError::DeserializationError(
        "Invalid compact-u16 encoding".to_string()
    ))
}

/// Manual encoding of length-prefixed byte arrays
pub fn encode_length_prefixed(data: &[u8], writer: &mut Vec<u8>) -> Result<()> {
    encode_compact_u16(data.len() as u16, writer)?;
    writer.write_all(data)?;
    Ok(())
}

/// Decode length-prefixed byte arrays
pub fn decode_length_prefixed(cursor: &mut Cursor<&[u8]>) -> Result<Vec<u8>> {
    let length = decode_compact_u16(cursor)? as usize;
    let position = cursor.position() as usize;
    let data = cursor.get_ref();

    if position + length > data.len() {
        return Err(TxAsmError::BufferTooSmall {
            needed: position + length,
            available: data.len(),
        });
    }

    let result = data[position..position + length].to_vec();
    cursor.set_position((position + length) as u64);
    Ok(result)
}

/// Encode a 32-byte public key
pub fn encode_pubkey(pubkey: &[u8; 32], writer: &mut Vec<u8>) -> Result<()> {
    writer.write_all(pubkey)?;
    Ok(())
}

/// Decode a 32-byte public key
pub fn decode_pubkey(cursor: &mut Cursor<&[u8]>) -> Result<[u8; 32]> {
    let position = cursor.position() as usize;
    let data = cursor.get_ref();

    if position + 32 > data.len() {
        return Err(TxAsmError::BufferTooSmall {
            needed: position + 32,
            available: data.len(),
        });
    }

    let mut pubkey = [0u8; 32];
    pubkey.copy_from_slice(&data[position..position + 32]);
    cursor.set_position((position + 32) as u64);
    Ok(pubkey)
}

/// Encode a u64 in little-endian format
pub fn encode_u64(value: u64, writer: &mut Vec<u8>) -> Result<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

/// Decode a u64 in little-endian format
pub fn decode_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64> {
    let position = cursor.position() as usize;
    let data = cursor.get_ref();

    if position + 8 > data.len() {
        return Err(TxAsmError::BufferTooSmall {
            needed: position + 8,
            available: data.len(),
        });
    }

    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[position..position + 8]);
    cursor.set_position((position + 8) as u64);
    Ok(u64::from_le_bytes(bytes))
}

/// Encode a u8
pub fn encode_u8(value: u8, writer: &mut Vec<u8>) -> Result<()> {
    writer.write_all(&[value])?;
    Ok(())
}

/// Decode a u8
pub fn decode_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8> {
    let position = cursor.position() as usize;
    let data = cursor.get_ref();

    if position >= data.len() {
        return Err(TxAsmError::BufferTooSmall {
            needed: position + 1,
            available: data.len(),
        });
    }

    let value = data[position];
    cursor.set_position((position + 1) as u64);
    Ok(value)
}

/// Custom serialization helpers for common Solana types
pub mod helpers {
    use super::*;

    /// Serialize a vector with length prefix
    pub fn serialize_vec<T: ByteSerialize>(items: &[T], writer: &mut Vec<u8>) -> Result<()> {
        encode_compact_u16(items.len() as u16, writer)?;
        for item in items {
            item.serialize_bytes(writer)?;
        }
        Ok(())
    }

    /// Calculate total byte size of a vector
    pub fn vec_byte_size<T: ByteSerialize>(items: &[T]) -> usize {
        let len = items.len();
        let len_size = if len <= 0x7f {
            1
        } else if len <= 0x3fff {
            2
        } else {
            3
        };
        len_size + items.iter().map(|item| item.byte_size()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_u16_encoding() {
        let mut buf = Vec::new();
        encode_compact_u16(42, &mut buf).unwrap();
        
        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = decode_compact_u16(&mut cursor).unwrap();
        assert_eq!(decoded, 42);
    }

    #[test]
    fn test_compact_u16_large() {
        let mut buf = Vec::new();
        encode_compact_u16(16383, &mut buf).unwrap();
        
        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = decode_compact_u16(&mut cursor).unwrap();
        assert_eq!(decoded, 16383);
    }

    #[test]
    fn test_length_prefixed() {
        let data = b"Hello, TxAsm!";
        let mut buf = Vec::new();
        encode_length_prefixed(data, &mut buf).unwrap();
        
        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = decode_length_prefixed(&mut cursor).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_u64_encoding() {
        let mut buf = Vec::new();
        encode_u64(0x1234567890ABCDEF, &mut buf).unwrap();
        
        let mut cursor = Cursor::new(buf.as_slice());
        let decoded = decode_u64(&mut cursor).unwrap();
        assert_eq!(decoded, 0x1234567890ABCDEF);
    }
}
