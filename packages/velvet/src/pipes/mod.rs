use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub enum PipeName {
    DecodeBallots,
    DoTally,
    Consolidation,
    TiesResolution,
    ComputeResult,
    GenerateReport,
}

impl FromStr for PipeName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim_start_matches("Velvet") {
            "DecodeBallots" => Ok(PipeName::DecodeBallots),
            "DoTally" => Ok(PipeName::DoTally),
            "Consolidation" => Ok(PipeName::Consolidation),
            "TiesResolution" => Ok(PipeName::TiesResolution),
            "ComputeResult" => Ok(PipeName::ComputeResult),
            "GenerateReport" => Ok(PipeName::GenerateReport),
            _ => Err(format!("'{}' cannot be parsed into a Pipe", s)),
        }
    }
}

struct PipeNameVisitor;

impl<'de> Visitor<'de> for PipeNameVisitor {
    type Value = PipeName;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string starting with 'Velvet' and followed by a PipeName variant")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        v.trim_start_matches("Velvet").parse().map_err(E::custom)
    }
}

pub fn deserialize_pipe<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PipeName, D::Error> {
    deserializer.deserialize_str(PipeNameVisitor)
}
