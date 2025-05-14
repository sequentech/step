use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum LocalVotingStatus {
    NOT_STARTED,
    OPEN,
    PAUSED,
    CLOSED,
}