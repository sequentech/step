// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::{distributions::Uniform, Rng};
use rocket::http::Status;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::{instrument, warn};

#[derive(Deserialize, Debug)]
pub struct VoterInformationBody {
    pub voter_id: String,
    pub ward: String,
    pub schoolboard: Option<String>,
    pub poll: Option<String>,
    pub birthdate: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MarkVotedBody {
    pub voter_id: String,
    pub channel: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatafixResponse {
    pub code: u16,
    pub message: String,
}

pub type JsonErrorResponse = Json<DatafixResponse>;

impl DatafixResponse {
    #[instrument]
    pub fn new(status: Status) -> JsonErrorResponse {
        Json(DatafixResponse {
            code: status.code,
            message: status.reason().unwrap_or_default().to_string(),
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct VoterviewRequest {
    pub url: String,
    pub usr: String,
    pub psw: String,
    pub county_mun: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DatafixAnnotations {
    pub id: String,
    pub password_policy: PasswordPolicy,
    pub voterview_request: VoterviewRequest,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum BasePolicy {
    #[strum(serialize = "id-password-concatenated")]
    #[serde(rename = "id-password-concatenated")]
    IdPswConcat,
    #[default]
    #[strum(serialize = "password-only")]
    #[serde(rename = "password-only")]
    PswOnly,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum CharactersPolicy {
    #[strum(serialize = "numeric")]
    #[serde(rename = "numeric")]
    Numeric,
    #[default]
    #[strum(serialize = "alphanumeric")]
    #[serde(rename = "alphanumeric")]
    Alphanumeric,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PasswordPolicy {
    base: BasePolicy,
    size: usize,
    characters: CharactersPolicy,
}

impl PasswordPolicy {
    #[instrument]
    pub fn generate_password(self, voter_id: &str) -> String {
        let pin = match self.characters {
            CharactersPolicy::Numeric => {
                let mut pass = String::new();
                let mut rng = rand::thread_rng();
                for _ in 0..self.size {
                    pass.push_str(rng.gen_range(0..10).to_string().as_str());
                }
                pass
            }
            CharactersPolicy::Alphanumeric => rand::thread_rng()
                .sample_iter(Uniform::new(char::from(32), char::from(126))) // In the range of the ascii characters
                .take(self.size)
                .map(char::from)
                .collect(),
        };
        match self.base {
            BasePolicy::IdPswConcat => format!("{}{}", voter_id, pin),
            BasePolicy::PswOnly => pin,
        }
    }
}
