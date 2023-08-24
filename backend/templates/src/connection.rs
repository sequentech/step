use crate::rocket::outcome::IntoOutcome;
use rocket::http::ext::IntoCollection;
use rocket::http::{HeaderMap, Status};
use rocket::outcome::{try_outcome, Outcome::*};
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::serde::json::{json, Value};
use rocket::State;
use rocket::{Build, Rocket};
use std::convert::From;
use std::env;

#[derive(Debug, Clone)]
pub struct AuthHeaders {
    pub key: String,
    pub value: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthHeaders {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let mut headers = request.headers().clone();
        if headers.contains("X-Hasura-Admin-Secret") {
            Outcome::Success(AuthHeaders {
                key: "X-Hasura-Admin-Secret".to_string(),
                value: headers
                    .get_one("X-Hasura-Admin-Secret")
                    .unwrap()
                    .to_string(),
            })
        } else if headers.contains("authorization") {
            Outcome::Success(AuthHeaders {
                key: "authorization".to_string(),
                value: headers.get_one("authorization").unwrap().to_string(),
            })
        } else {
            Outcome::Failure((Status::Unauthorized, ()))
        }
    }
}
