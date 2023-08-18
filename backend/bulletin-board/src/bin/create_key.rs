// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use bulletin_board::util::KeyPairConfig;
use strand::rnd::StrandRng as Rng;
use strand::signature::StrandSignatureSk as SecretKey;
use tracing::instrument;

/// Simple utility that generates a public and a private key to be used to
/// connect and sign requests to the bulletin board service.
#[instrument]
fn main() -> Result<()> {
    let key_pair_config = generate_keys()?;
    let key_pair_config_toml = toml::to_string(&key_pair_config)?;
    println!("{}", key_pair_config_toml);
    Ok(())
}

fn generate_keys() -> Result<KeyPairConfig> {
    let mut generator = Rng;
    let secret_key = SecretKey::new(&mut generator);
    Ok(KeyPairConfig::try_from(secret_key)?)
}
