// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::pipes::error::{Error, Result};
use serde::Deserialize;
use serde_path_to_error;
use serde_path_to_error::Deserializer;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;

pub trait HasId {
    fn id(&self) -> &str;
}

pub fn parse_file<T: for<'a> Deserialize<'a>>(mut file: File) -> Result<T> {
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let jd = &mut serde_json::Deserializer::from_str(&contents);

    let result: Result<T, _> = serde_path_to_error::deserialize(jd);

    result.map_err(|err| {
        Error::UnexpectedError(format!("Parse error: {:?} . Contents {contents}", err))
    })
}
