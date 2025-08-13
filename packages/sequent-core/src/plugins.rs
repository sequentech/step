// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum Plugins {
    #[strum(serialize = "miru")]
    MIRU,
}

pub fn get_plugin_shared_dir(plugin: &Plugins) -> String {
    match plugin {
        Plugins::MIRU => format!("/temp/{}", Plugins::MIRU),
    }
}
