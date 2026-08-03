// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    ballot::VotingStatusChannel,
    types::tally_sheets::VotingChannel as TallySheetVotingChannel,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

/// A channel that contributes to participation, whether its ballots were cast
/// electronically or entered through tally sheets.
///
/// Unknown values are preserved to keep persisted data forward-compatible with
/// channels introduced by newer versions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParticipationChannel {
    CastVote(VotingStatusChannel),
    TallySheet(TallySheetVotingChannel),
    Unknown(String),
}

pub type VotesByChannel = BTreeMap<ParticipationChannel, u64>;

impl ParticipationChannel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::CastVote(channel) => channel.as_ref(),
            Self::TallySheet(channel) => channel.as_ref(),
            Self::Unknown(channel) => channel,
        }
    }

    pub const fn sort_order(&self) -> usize {
        match self {
            Self::CastVote(VotingStatusChannel::ONLINE) => 0,
            Self::CastVote(VotingStatusChannel::KIOSK) => 1,
            Self::CastVote(VotingStatusChannel::EARLY_VOTING) => 2,
            Self::CastVote(VotingStatusChannel::TELEPHONE) => 3,
            Self::TallySheet(TallySheetVotingChannel::PAPER) => 4,
            Self::TallySheet(TallySheetVotingChannel::POSTAL) => 5,
            Self::TallySheet(TallySheetVotingChannel::IN_PERSON) => 6,
            Self::Unknown(_) => usize::MAX,
        }
    }

    pub fn report_label(&self) -> Cow<'_, str> {
        match self {
            Self::CastVote(VotingStatusChannel::ONLINE) => {
                Cow::Borrowed("Online")
            }
            Self::CastVote(VotingStatusChannel::KIOSK) => {
                Cow::Borrowed("Kiosk")
            }
            Self::CastVote(VotingStatusChannel::EARLY_VOTING) => {
                Cow::Borrowed("Early voting")
            }
            Self::CastVote(VotingStatusChannel::TELEPHONE) => {
                Cow::Borrowed("Telephone")
            }
            Self::TallySheet(TallySheetVotingChannel::PAPER) => {
                Cow::Borrowed("Paper")
            }
            Self::TallySheet(TallySheetVotingChannel::POSTAL) => {
                Cow::Borrowed("Postal")
            }
            Self::TallySheet(TallySheetVotingChannel::IN_PERSON) => {
                Cow::Borrowed("In person")
            }
            Self::Unknown(channel) => Cow::Owned(humanize_channel(channel)),
        }
    }
}

fn humanize_channel(channel: &str) -> String {
    channel
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.to_lowercase().chars().collect::<Vec<_>>();
            if let Some(first) = chars.first_mut() {
                first.make_ascii_uppercase();
            }
            chars.into_iter().collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl From<VotingStatusChannel> for ParticipationChannel {
    fn from(channel: VotingStatusChannel) -> Self {
        Self::CastVote(channel)
    }
}

impl From<TallySheetVotingChannel> for ParticipationChannel {
    fn from(channel: TallySheetVotingChannel) -> Self {
        Self::TallySheet(channel)
    }
}

impl From<String> for ParticipationChannel {
    fn from(channel: String) -> Self {
        if let Ok(channel) = VotingStatusChannel::from_str(&channel) {
            Self::CastVote(channel)
        } else if let Ok(channel) = TallySheetVotingChannel::from_str(&channel)
        {
            Self::TallySheet(channel)
        } else {
            Self::Unknown(channel)
        }
    }
}

impl From<&str> for ParticipationChannel {
    fn from(channel: &str) -> Self {
        Self::from(channel.to_string())
    }
}

impl Display for ParticipationChannel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Ord for ParticipationChannel {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_order()
            .cmp(&other.sort_order())
            .then_with(|| self.as_str().cmp(other.as_str()))
    }
}

impl PartialOrd for ParticipationChannel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for ParticipationChannel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ParticipationChannel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_both_channel_families_and_preserves_unknown_values() {
        assert_eq!(
            ParticipationChannel::from("ONLINE"),
            ParticipationChannel::CastVote(VotingStatusChannel::ONLINE)
        );
        assert_eq!(
            ParticipationChannel::from("PAPER"),
            ParticipationChannel::TallySheet(TallySheetVotingChannel::PAPER)
        );
        assert_eq!(
            ParticipationChannel::from("FUTURE_CHANNEL"),
            ParticipationChannel::Unknown("FUTURE_CHANNEL".to_string())
        );
    }

    #[test]
    fn serializes_channel_maps_as_the_existing_flat_string_format() {
        let json = json!({"ONLINE": 3, "PAPER": 2, "FUTURE_CHANNEL": 1});
        let channels: VotesByChannel =
            serde_json::from_value(json.clone()).unwrap();

        assert_eq!(serde_json::to_value(channels).unwrap(), json);
    }

    #[test]
    fn provides_canonical_report_order_and_labels() {
        let mut channels = vec![
            ParticipationChannel::Unknown("FUTURE_CHANNEL".to_string()),
            TallySheetVotingChannel::IN_PERSON.into(),
            VotingStatusChannel::TELEPHONE.into(),
            TallySheetVotingChannel::POSTAL.into(),
            VotingStatusChannel::EARLY_VOTING.into(),
            TallySheetVotingChannel::PAPER.into(),
            VotingStatusChannel::KIOSK.into(),
            VotingStatusChannel::ONLINE.into(),
        ];

        channels.sort();

        assert_eq!(
            channels
                .iter()
                .map(ParticipationChannel::as_str)
                .collect::<Vec<_>>(),
            vec![
                "ONLINE",
                "KIOSK",
                "EARLY_VOTING",
                "TELEPHONE",
                "PAPER",
                "POSTAL",
                "IN_PERSON",
                "FUTURE_CHANNEL",
            ]
        );
        assert_eq!(
            channels
                .iter()
                .map(|channel| channel.report_label().into_owned())
                .collect::<Vec<_>>(),
            vec![
                "Online",
                "Kiosk",
                "Early voting",
                "Telephone",
                "Paper",
                "Postal",
                "In person",
                "Future Channel",
            ]
        );
    }
}
