// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

/// Signs the Configuration.
///
/// The placeholder logic in is_config_approved will be checks
/// in addition to general validation already performed in
/// Configuration::is_valid, which is called when
/// the trustee's LocalBoard is bootstrapped.
pub(super) fn sign_config<C: Ctx>(
    configuration_h: &ConfigurationHash,
    trustee: &Trustee<C>,
) -> Result<Vec<Message>, ProtocolError> {
    let cfg = trustee.get_configuration(configuration_h)?;
    // FIXME assert
    assert!(trustee.is_config_approved(cfg));
    trace!("Configuration is valid");

    let self_index = cfg.get_trustee_position(&trustee.get_pk()?);
    // FIXME assert
    assert!(self_index.is_some());

    let m = Message::configuration_msg(cfg, trustee)?;
    Ok(vec![m])
}
