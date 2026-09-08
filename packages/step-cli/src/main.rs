// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only

mod commands;
mod load;
mod tests;
mod types;
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cli",
    version = "1.0",
    about = "CLI tool for managing Sequent tasks"
)]
struct Cli {
    #[command(subcommand)]
    command: MainCommand,
}

#[derive(Subcommand)]
enum MainCommand {
    /// Prepare and measure complete synthetic voting journeys.
    #[command(subcommand)]
    Load(load::Command),
    #[command(subcommand)]
    Step(StepCommands),
}

#[derive(Subcommand)]
enum StepCommands {
    Config(commands::configure::Config),
    CreateTenant(commands::create_tenant::CreateTenant),
    CreateElectionEvent(commands::create_election_event::CreateElectionEventCLI),
    CreateElection(commands::create_election::CreateElection),
    CreateContest(commands::create_contest::CreateContest),
    CreateCandidate(commands::create_candidate::CreateCandidate),
    CreateArea(commands::create_area::CreateArea),
    CreateAreaContest(commands::create_area_contest::CreateAreaContest),
    CreateVoter(commands::create_voter::CreateVoter),
    DeleteElectionEvent(commands::delete_election_event::DeleteElectionEventCLI),
    DeleteTenant(commands::delete_tenant::DeleteTenantCLI),
    ExportCastVotes(commands::export_cast_votes::ExportCastVotes),
    ExportElectionEvent(commands::export_election_event::ExportElectionEventCommand),
    UpdateVoter(commands::update_voter::UpdateVoter),
    ImportElection(commands::import_election_event::ImportElectionEventFile),
    ImportVoters(commands::import_voters::ImportVoters),
    Publish(commands::publish_changes::PublishChanges),
    RefreshToken(commands::refresh_token::Refresh),
    StartKeyCeremony(commands::start_key_ceremony::StartKeyCeremony),
    CompleteKeyCeremony(commands::complete_key_ceremony::Complete),
    GetKeyCeremonyStatus(commands::get_key_ceremony_status::GetKeyCeremonyStatus),
    StartTally(commands::start_tally::StartTallyCeremony),
    UpdateTally(commands::update_tally_status::UpdateTallyStatus),
    SubmitTallyResolution(commands::submit_tally_resolution::SubmitTallyResolution),
    TallySheet(commands::tally_sheet::TallySheetCommand),
    ConfirmKeyTally(commands::confirm_tally_ceremoney_key::ConfirmKeyForTally),
    RenderTemplate(commands::render_template::RenderTemplate),
    GenerateVoters(commands::generate_voters::GenerateVoters),
    DuplicateVotes(commands::duplicate_votes::DuplicateVotes),
    CreateApplications(commands::create_applications::CreateApplications),
    CreateElectoralLogs(commands::create_electoral_logs::CreateElectoralLogs),
    HashPassword(commands::hash_passwords::HashPasswords),
    UpdateEventVotingStatus(commands::update_event_voting_status::UpdateElectionEventVotingStatus),
    UpdateElectionVotingStatus(
        commands::update_election_voting_status::UpdateElectionVotingStatusCommand,
    ),
    DownloadTallyResults(commands::download_tally_results::DownloadTallyResults),
    GeneratePreviewUrl(commands::generate_preview::GeneratePreview),
    ConfigureResultsWebsite(commands::results_publication::ConfigureResultsWebsite),
    PublishResults(commands::results_publication::PublishResults),
    RevokeResultsPublication(commands::results_publication::RevokeResultsPublication),
    ExportTenantConfig(commands::export_tenant_config::ExportTenantConfig),
    ImportTenantConfig(commands::import_tenant_config::ImportTenantConfig),
    ListTrustees(commands::get_trustees::ListTrustees),
    CreateTrustee(commands::create_trustee::CreateTrustee),
    DownloadDocument(commands::download_document::DownloadDocument),
    UploadDocument(commands::upload_document::UploadDocument),
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        MainCommand::Load(command) => {
            if let Err(error) = command.run() {
                eprintln!("{error:#}");
                std::process::exit(1);
            }
        }
        MainCommand::Step(step_cmd) => match step_cmd {
            StepCommands::Config(cmd) => cmd.run(),
            StepCommands::CreateTenant(create_tenant) => exit_on_error(create_tenant.run()),
            StepCommands::CreateElectionEvent(create_event) => create_event.run(),
            StepCommands::CreateElection(create_election) => create_election.run(),
            StepCommands::CreateContest(create_contest) => create_contest.run(),
            StepCommands::CreateCandidate(create_candidate) => create_candidate.run(),
            StepCommands::CreateArea(create_area) => create_area.run(),
            StepCommands::CreateAreaContest(create_area_contest) => create_area_contest.run(),
            StepCommands::ExportCastVotes(export_cast_votes) => export_cast_votes.run(),
            StepCommands::ExportElectionEvent(export_election_event) => export_election_event.run(),
            StepCommands::ImportElection(import) => import.run(),
            StepCommands::ImportVoters(import_voters) => exit_on_error(import_voters.run()),
            StepCommands::CreateVoter(create_voter) => create_voter.run(),
            StepCommands::DeleteElectionEvent(delete_event) => exit_on_error(delete_event.run()),
            StepCommands::DeleteTenant(delete_tenant) => exit_on_error(delete_tenant.run()),
            StepCommands::UpdateVoter(update_voter) => update_voter.run(),
            StepCommands::Publish(publish_ballot) => publish_ballot.run(),
            StepCommands::RefreshToken(refresh) => refresh.run(),
            StepCommands::StartKeyCeremony(start) => start.run(),
            StepCommands::CompleteKeyCeremony(complete) => complete.run(),
            StepCommands::GetKeyCeremonyStatus(status) => exit_on_error(status.run()),
            StepCommands::StartTally(start) => start.run(),
            StepCommands::UpdateTally(update) => update.run(),
            StepCommands::SubmitTallyResolution(submit) => submit.run(),
            StepCommands::TallySheet(command) => command.run(),
            StepCommands::ConfirmKeyTally(confirm) => confirm.run(),
            StepCommands::RenderTemplate(render) => render.run(),
            StepCommands::GenerateVoters(render) => render.run(),
            StepCommands::DuplicateVotes(render) => render.run(),
            StepCommands::CreateApplications(render) => render.run(),
            StepCommands::CreateElectoralLogs(render) => render.run(),
            StepCommands::HashPassword(render) => render.run(),
            StepCommands::UpdateEventVotingStatus(update_event_voting_status) => {
                update_event_voting_status.run()
            }
            StepCommands::UpdateElectionVotingStatus(update_election_voting_status) => {
                update_election_voting_status.run()
            }
            StepCommands::DownloadTallyResults(download) => download.run(),
            StepCommands::GeneratePreviewUrl(render) => render.run(),
            StepCommands::ConfigureResultsWebsite(configure_results_website) => {
                configure_results_website.run()
            }
            StepCommands::PublishResults(publish_results) => publish_results.run(),
            StepCommands::RevokeResultsPublication(revoke_results_publication) => {
                revoke_results_publication.run()
            }
            StepCommands::ExportTenantConfig(export) => exit_on_error(export.run()),
            StepCommands::ImportTenantConfig(import) => exit_on_error(import.run()),
            StepCommands::ListTrustees(list) => exit_on_error(list.run()),
            StepCommands::CreateTrustee(create) => exit_on_error(create.run()),
            StepCommands::DownloadDocument(download) => exit_on_error(download.run()),
            StepCommands::UploadDocument(upload) => exit_on_error(upload.run()),
        },
    }
}

/// Give scripts a nonzero status while leaving the detailed error on stderr.
fn exit_on_error(result: Result<(), impl std::fmt::Display>) {
    if let Err(error) = result {
        eprintln!("Error! {error}");
        std::process::exit(1);
    }
}
