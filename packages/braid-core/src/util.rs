// Minimal subset of braid::util used by the core trustee engine.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("{0}")]
    Msg(String),

    #[error("{0}: {1}")]
    WithContext(&'static str, String),
}

impl ProtocolError {
    pub fn with_context(self, ctx: &'static str) -> ProtocolError {
        match self {
            ProtocolError::Msg(m) => ProtocolError::WithContext(ctx, m),
            ProtocolError::WithContext(_, m) => ProtocolError::WithContext(ctx, m),
        }
    }
}

pub struct ProtocolContext;

impl ProtocolContext {
    pub fn new() -> Self {
        ProtocolContext
    }
}

pub fn dbg_hash(label: &str, bytes: &[u8]) {
    use hex::encode;
    log::trace!("{}: {}", label, encode(bytes));
}
