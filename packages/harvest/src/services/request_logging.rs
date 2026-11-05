// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Rocket 0.5 logs requests outside the route handler. Scope the logging of
//! routine polls (trustee checks and board reads) to their request task so
//! shared authorization and SQL helpers keep their usual levels for every
//! other caller.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Method;
use rocket::{Data, Request, Response};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, Weak};
use tokio::task::{try_id, Id};
use tracing::{level_filters::LevelFilter, Level, Metadata};

const TRUSTEE_BOARDS_PATH: &str = "/trustee/boards";

static CHECKS: LazyLock<Mutex<HashMap<Id, Weak<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The request owns this guard through its local cache. Rocket handles each
/// network request in a separate Tokio task, and keeps the request alive until
/// the response is written. A weak entry also handles cancellation and panics.
struct Check {
    task: Id,
    marker: Arc<()>,
}

impl Drop for Check {
    fn drop(&mut self) {
        if let Ok(mut checks) = CHECKS.lock() {
            if checks.get(&self.task).is_some_and(|marker| {
                marker.ptr_eq(&Arc::downgrade(&self.marker))
            }) {
                checks.remove(&self.task);
            }
        }
    }
}

pub struct TrusteeCheckLogging;

pub fn fairing() -> TrusteeCheckLogging {
    TrusteeCheckLogging
}

/// The routine polls a trustee repeats while it works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoutineCheck {
    /// `GET /trustee/boards`: the boards the trustee has work on.
    TrusteeBoards,
    /// `GET /b4/boards/<board>/messages`: a board's messages.
    BoardMessages,
}

impl RoutineCheck {
    /// What a failed poll is logged as.
    fn failure(self) -> &'static str {
        match self {
            RoutineCheck::TrusteeBoards => "trustee board check failed",
            RoutineCheck::BoardMessages => "board messages read failed",
        }
    }
}

fn routine_check(request: &Request<'_>) -> Option<RoutineCheck> {
    if request.method() != Method::Get {
        return None;
    }
    let path = request.uri().path();
    if path.as_str() == TRUSTEE_BOARDS_PATH {
        return Some(RoutineCheck::TrusteeBoards);
    }
    let segments: Vec<&str> = path.segments().collect();
    matches!(segments.as_slice(), ["b4", "boards", _, "messages"])
        .then_some(RoutineCheck::BoardMessages)
}

#[rocket::async_trait]
impl Fairing for TrusteeCheckLogging {
    fn info(&self) -> Info {
        Info {
            name: "Trustee check logging",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        let Some(task) = try_id() else {
            return;
        };
        // Local clients can dispatch several requests from the same task.
        if let Ok(mut checks) = CHECKS.lock() {
            checks.remove(&task);
            if routine_check(request).is_some() {
                let check = request.local_cache(|| Check {
                    task,
                    marker: Arc::new(()),
                });
                checks.insert(task, Arc::downgrade(&check.marker));
            }
        }
    }

    async fn on_response<'r>(
        &self,
        request: &'r Request<'_>,
        response: &mut Response<'r>,
    ) {
        let Some(check) = routine_check(request) else {
            return;
        };
        if !response.status().class().is_success() {
            if let Some(task) = try_id() {
                if let Ok(mut checks) = CHECKS.lock() {
                    checks.remove(&task);
                }
            }
            tracing::warn!(status = %response.status(), "{}", check.failure());
        }
    }
}

/// Successful checks are visible at debug verbosity, including Rocket's
/// request/response logs and shared helper spans. Warnings and errors always
/// retain their normal visibility. Poisoned state falls back to normal logs.
pub fn enabled(metadata: &Metadata<'_>, level: LevelFilter) -> bool {
    if *metadata.level() <= Level::WARN || level >= LevelFilter::DEBUG {
        return true;
    }
    let Some(task) = try_id() else {
        return true;
    };
    CHECKS
        .lock()
        .map(|checks| checks.get(&task).and_then(Weak::upgrade).is_none())
        .unwrap_or(true)
}

#[cfg(test)]
#[path = "../../tests/support/request_logging.rs"]
mod tests;
