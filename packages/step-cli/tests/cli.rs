// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Every step-cli integration test, in one test binary. Each binary links the
//! whole CLI and its dependencies, so fourteen of them spent minutes linking
//! (sequentech/meta#14622). A new test file needs a `mod` line here; run one
//! file with `cargo test -p step-cli --test cli <module>::`.

#[path = "support/http_cli.rs"]
mod http_cli;

mod acceptance_cli;
mod command_failures;
mod complete_key_ceremony;
mod import_cli;
mod kubernetes_cli;
mod load_kubernetes;
mod password_cli;
mod readiness_cli;
mod refresh_token_cli;
mod tally_sheet_cli;
mod upload_document_cli;
mod verify_ballot_box_seal_cli;
mod verify_transmission_package_cli;
mod voter_progress_cli;
