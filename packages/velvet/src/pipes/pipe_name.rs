use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Debug, Serialize, Deserialize, EnumString, Clone, Copy, PartialEq)]
pub enum PipeName {
    DecodeBallots,
    DoTally,
    Consolidation,
    TiesResolution,
    ComputeResult,
    GenerateReport,
}

struct PipeNameVisitor;

impl<'de> Visitor<'de> for PipeNameVisitor {
    type Value = PipeName;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string starting with 'Velvet' and followed by a PipeName variant")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        PipeName::from_str(v.trim_start_matches("Velvet")).map_err(E::custom)
    }
}

pub fn deserialize_pipe<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PipeName, D::Error> {
    deserializer.deserialize_str(PipeNameVisitor)
}
