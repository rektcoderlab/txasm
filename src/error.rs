//! Error types for TxAsm

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TxAsmError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Invalid instruction data: {0}")]
    InvalidInstruction(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("Signature error: {0}")]
    SignatureError(String),

    #[error("Account error: {0}")]
    AccountError(String),

    #[error("Fee calculation error: {0}")]
    FeeCalculationError(String),

    #[error("Optimization error: {0}")]
    OptimizationError(String),

    #[error("Buffer too small: needed {needed} bytes, got {available}")]
    BufferTooSmall {
        needed: usize,
        available: usize,
    },

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Solana SDK error: {0}")]
    SolanaError(String),
}

impl From<std::io::Error> for TxAsmError {
    fn from(err: std::io::Error) -> Self {
        TxAsmError::SerializationError(err.to_string())
    }
}

impl From<bs58::decode::Error> for TxAsmError {
    fn from(err: bs58::decode::Error) -> Self {
        TxAsmError::DecodingError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, TxAsmError>;
