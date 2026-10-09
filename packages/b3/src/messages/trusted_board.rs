// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use strand::{context::Ctx, serialization::StrandDeserialize, signature::StrandSignaturePk};

use super::{artifact::Configuration, message::Message, newtypes::*, statement::Statement};

#[cfg(test)]
mod tests;

/// A board configuration is usable only under an independently trusted manager key.
pub enum BoardConfigurationState<C: Ctx> {
    Missing,
    Trusted(Configuration<C>),
    Foreign,
}

pub fn configuration_state<C: Ctx>(
    messages: &[Message],
    manager: &StrandSignaturePk,
) -> BoardConfigurationState<C> {
    let configs: Vec<_> = messages
        .iter()
        .filter(|m| matches!(m.statement, Statement::Configuration(..)))
        .collect();
    if configs.is_empty() {
        return BoardConfigurationState::Missing;
    }
    if configs.len() != 1 {
        return BoardConfigurationState::Foreign;
    }
    let message = configs[0];
    let config = message
        .artifact
        .as_ref()
        .and_then(|bytes| Configuration::<C>::strand_deserialize(bytes).ok());
    match config {
        Some(config)
            if config.is_valid()
                && &config.protocol_manager == manager
                && message.verify(&config).is_ok() =>
        {
            BoardConfigurationState::Trusted(config)
        }
        _ => BoardConfigurationState::Foreign,
    }
}

/// Verify every row before any row can contribute to ceremony state or results.
pub fn verify_board<C: Ctx>(
    messages: &[Message],
    manager: &StrandSignaturePk,
) -> Result<Configuration<C>> {
    let config = match configuration_state(messages, manager) {
        BoardConfigurationState::Trusted(config) => config,
        BoardConfigurationState::Missing => return Err(anyhow!("Board configuration is missing")),
        BoardConfigurationState::Foreign => {
            return Err(anyhow!(
                "Board configuration does not match the trusted manager"
            ))
        }
    };
    for message in messages {
        let verified = message.verify(&config)?;
        if !matches!(
            message.statement,
            Statement::Configuration(..) | Statement::Ballots(..)
        ) && verified.signer_position == PROTOCOL_MANAGER_INDEX
        {
            return Err(anyhow!(
                "Trustee statement must be signed by a configured trustee"
            ));
        }
    }
    Ok(config)
}

fn same_public_key(left: &Statement, right: &Statement) -> bool {
    match (left, right) {
        (
            Statement::PublicKey(_, c, p, s, h),
            Statement::PublicKey(_, c2, p2, s2, h2) | Statement::PublicKeySigned(_, c2, p2, s2, h2),
        ) => (c, p, s, h) == (c2, p2, s2, h2),
        _ => false,
    }
}

/// Require the producing trustee and every other trustee to agree on the complete key statement.
pub fn agreed_public_key<'a, C: Ctx>(
    messages: &'a [Message],
    config: &Configuration<C>,
) -> Option<&'a Message> {
    messages.iter().find(|candidate| {
        matches!(candidate.statement, Statement::PublicKey(..))
            && candidate.artifact.is_some()
            && config.trustees.first() == Some(&candidate.sender.pk)
            && config.trustees.iter().all(|pk| {
                messages.iter().any(|m| {
                    &m.sender.pk == pk && same_public_key(&candidate.statement, &m.statement)
                })
            })
    })
}

fn same_plaintexts(left: &Statement, right: &Statement) -> bool {
    match (left, right) {
        (
            Statement::Plaintexts(_, c, b, p, d, h, k),
            Statement::Plaintexts(_, c2, b2, p2, d2, h2, k2)
            | Statement::PlaintextsSigned(_, c2, b2, p2, d2, h2, k2),
        ) => (c, b, p, d, h, k) == (c2, b2, p2, d2, h2, k2),
        _ => false,
    }
}

/// A result is ready only when every selected decryption trustee agrees with its producer.
pub fn plaintexts_agreed<C: Ctx>(
    candidate: &Message,
    messages: &[Message],
    config: &Configuration<C>,
) -> bool {
    let Statement::Plaintexts(_, cfg, batch, _, _, _, pk) = &candidate.statement else {
        return false;
    };
    if candidate.artifact.is_none() {
        return false;
    }
    messages.iter().any(|ballots| {
        let Statement::Ballots(_, bc, bb, _, bk, selected) = &ballots.statement else {
            return false;
        };
        if (cfg, batch, pk) != (bc, bb, bk) || config.threshold > selected.len() {
            return false;
        }
        let active = &selected[..config.threshold];
        if selected[config.threshold..]
            .iter()
            .any(|p| *p != NULL_TRUSTEE)
            || active
                .iter()
                .enumerate()
                .any(|(i, p)| *p == 0 || *p > config.trustees.len() || active[..i].contains(p))
        {
            return false;
        }
        config.trustees.get(active[0] - 1) == Some(&candidate.sender.pk)
            && active.iter().all(|position| {
                messages.iter().any(|m| {
                    m.sender.pk == config.trustees[*position - 1]
                        && same_plaintexts(&candidate.statement, &m.statement)
                })
            })
    })
}
