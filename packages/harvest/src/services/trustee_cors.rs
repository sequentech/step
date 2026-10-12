// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! CORS for authenticated browser trustees, restricted to configured portals.
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};

pub struct TrusteeCors {
    origins: Vec<String>,
}
impl TrusteeCors {
    pub fn from_environment() -> Self {
        let origins = std::env::var("HARVEST_TRUSTEE_ALLOWED_ORIGINS")
            .or_else(|_| std::env::var("ADMIN_PORTAL_URL"))
            .unwrap_or_default();
        Self {
            origins: origins
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        }
    }
}
#[rocket::async_trait]
impl Fairing for TrusteeCors {
    fn info(&self) -> Info {
        Info {
            name: "Trustee portal CORS",
            kind: Kind::Response,
        }
    }
    async fn on_response<'r>(
        &self,
        request: &'r Request<'_>,
        response: &mut Response<'r>,
    ) {
        if !matches!(
            request.uri().path().as_str(),
            "/trustee/boards" | "/trustee/boards/report"
        ) {
            return;
        }
        response.adjoin_raw_header("Vary", "Origin");
        if let Some(origin) = request.headers().get_one("Origin") {
            if self.origins.iter().any(|allowed| allowed == origin) {
                response.set_raw_header(
                    "Access-Control-Allow-Origin",
                    origin.to_owned(),
                );
                response.set_raw_header(
                    "Access-Control-Allow-Methods",
                    "GET, POST, OPTIONS",
                );
                response.set_raw_header(
                    "Access-Control-Allow-Headers",
                    "Authorization, Content-Type",
                );
                response.set_raw_header("Access-Control-Max-Age", "600");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::{Header, Status};
    use rocket::local::asynchronous::Client;
    #[get("/trustee/boards")]
    fn protected() -> Status {
        Status::Unauthorized
    }
    #[get("/other")]
    fn other() -> Status {
        Status::Ok
    }
    #[rocket::async_test]
    async fn only_configured_origins_and_trustee_routes_get_cors_without_bypassing_auth(
    ) {
        let client = Client::tracked(
            rocket::build()
                .attach(TrusteeCors {
                    origins: vec!["https://admin.example".into()],
                })
                .mount(
                    "/",
                    routes![
                        protected,
                        other,
                        crate::routes::trustee_boards::trustee_boards_preflight,
                        crate::routes::trustee_boards::trustee_report_preflight
                    ],
                ),
        )
        .await
        .unwrap();
        let preflight = client
            .options("/trustee/boards")
            .header(Header::new("Origin", "https://admin.example"))
            .header(Header::new("Access-Control-Request-Method", "GET"))
            .header(Header::new(
                "Access-Control-Request-Headers",
                "authorization",
            ))
            .dispatch()
            .await;
        assert_eq!(preflight.status(), Status::NoContent);
        assert_eq!(
            preflight.headers().get_one("Access-Control-Allow-Origin"),
            Some("https://admin.example")
        );
        assert_eq!(
            preflight.headers().get_one("Access-Control-Allow-Headers"),
            Some("Authorization, Content-Type")
        );
        let protected = client
            .get("/trustee/boards")
            .header(Header::new("Origin", "https://admin.example"))
            .dispatch()
            .await;
        assert_eq!(protected.status(), Status::Unauthorized);
        assert_eq!(
            protected.headers().get_one("Access-Control-Allow-Origin"),
            Some("https://admin.example")
        );
        for origin in [
            "https://admin.example.attacker",
            "https://unrelated.example",
            "null",
        ] {
            let response = client
                .options("/trustee/boards/report")
                .header(Header::new("Origin", origin))
                .dispatch()
                .await;
            assert_eq!(
                response.headers().get_one("Access-Control-Allow-Origin"),
                None
            );
        }
        let response = client
            .get("/other")
            .header(Header::new("Origin", "https://admin.example"))
            .dispatch()
            .await;
        assert_eq!(
            response.headers().get_one("Access-Control-Allow-Origin"),
            None
        );
    }
}
