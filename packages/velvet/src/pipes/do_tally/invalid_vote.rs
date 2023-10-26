use serde::Serialize;
use strum_macros::AsRefStr;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidVote {
    Implicit,
    Explicit,
    MarkedAsInvalid,
}
