// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only
//! # CLI Tool for Managing Sequent Tasks

/// Commands
mod commands;
/// Tests
mod tests;
/// Types
mod types;
/// Utils
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cli",
    version = "1.0",
    about = "CLI tool for managing Sequent tasks"
)]
/// CLI struct
struct Cli {
    #[command(subcommand)]
    /// Main subcommands
    command: MainCommand,
}

#[derive(Subcommand)]
/// Main subcommands
enum MainCommand {
    #[command(subcommand)]
    /// All step subcommands
    Step(StepCommands),
}

#[derive(Subcommand)]
/// Step subcommands
enum StepCommands {
    /// Config command
    Config(commands::configure::Config),
    /// Create election event command
    CreateElectionEvent(commands::create_election_event::CreateElectionEventCLI),
    /// Create election command
    CreateElection(commands::create_election::CreateElection),
    /// Create contest command
    CreateContest(commands::create_contest::CreateContest),
    /// Create candidate command
    CreateCandidate(commands::create_candidate::CreateCandidate),
    /// Create area command
    CreateArea(commands::create_area::CreateArea),
    /// Create area contest command
    CreateAreaContest(commands::create_area_contest::CreateAreaContest),
    /// Create voter command
    CreateVoter(commands::create_voter::CreateVoter),
    /// Export cast votes command
    ExportCastVotes(commands::export_cast_votes::ExportCastVotes),
    /// Export election event command
    ExportElectionEvent(commands::export_election_event::ExportElectionEventCommand),
    /// Update voter command
    UpdateVoter(commands::update_voter::UpdateVoter),
    /// Import election event command
    ImportElection(commands::import_election_event::ImportElectionEventFile),
    /// Publish changes command
    Publish(commands::publish_changes::PublishChanges),
    /// Refresh token command
    RefreshToken(commands::refresh_token::Refresh),
    /// Start key ceremony command
    StartKeyCeremony(commands::start_key_ceremony::StartKeyCeremony),
    /// Complete key ceremony command
    CompleteKeyCeremony(commands::complete_key_ceremony::Complete),
    /// Start tally ceremony command
    StartTally(commands::start_tally::StartTallyCeremony),
    /// Update tally status command
    UpdateTally(commands::update_tally_status::UpdateTallyStatus),
    /// Submit tally resolution command
    SubmitTallyResolution(commands::submit_tally_resolution::SubmitTallyResolution),
    /// Confirm key tally command
    ConfirmKeyTally(commands::confirm_tally_ceremoney_key::ConfirmKeyForTally),
    /// Render template command
    RenderTemplate(commands::render_template::RenderTemplate),
    /// Generate voters command
    GenerateVoters(commands::generate_voters::GenerateVoters),
    /// Duplicate votes command
    DuplicateVotes(commands::duplicate_votes::DuplicateVotes),
    /// Create applications command
    CreateApplications(commands::create_applications::CreateApplications),
    /// Create electoral logs command
    CreateElectoralLogs(commands::create_electoral_logs::CreateElectoralLogs),
    /// Hash password command
    HashPassword(commands::hash_passwords::HashPasswords),
    /// Update event voting status command
    UpdateEventVotingStatus(commands::update_event_voting_status::UpdateElectionEventVotingStatus),
    /// Update election voting status command
    UpdateElectionVotingStatus(
        commands::update_election_voting_status::UpdateElectionVotingStatusCommand,
    ),
    /// Download tally results command
    DownloadTallyResults(commands::download_tally_results::DownloadTallyResults),
    /// Generate preview url command
    GeneratePreviewUrl(commands::generate_preview::GeneratePreview),
}
fn main() {
    let cli = Cli::parse();

    match &cli.command {
        MainCommand::Step(step_cmd) => match step_cmd {
            StepCommands::Config(cmd) => cmd.run(),
            StepCommands::CreateElectionEvent(create_event) => create_event.run(),
            StepCommands::CreateElection(create_election) => create_election.run(),
            StepCommands::CreateContest(create_contest) => create_contest.run(),
            StepCommands::CreateCandidate(create_candidate) => create_candidate.run(),
            StepCommands::CreateArea(create_area) => create_area.run(),
            StepCommands::CreateAreaContest(create_area_contest) => create_area_contest.run(),
            StepCommands::ExportCastVotes(export_cast_votes) => export_cast_votes.run(),
            StepCommands::ExportElectionEvent(export_election_event) => export_election_event.run(),
            StepCommands::ImportElection(import) => import.run(),
            StepCommands::CreateVoter(create_voter) => create_voter.run(),
            StepCommands::UpdateVoter(update_voter) => update_voter.run(),
            StepCommands::Publish(publish_ballot) => publish_ballot.run(),
            StepCommands::RefreshToken(refresh) => refresh.run(),
            StepCommands::StartKeyCeremony(start) => start.run(),
            StepCommands::CompleteKeyCeremony(complete) => complete.run(),
            StepCommands::StartTally(start) => start.run(),
            StepCommands::UpdateTally(update) => update.run(),
            StepCommands::SubmitTallyResolution(submit) => submit.run(),
            StepCommands::ConfirmKeyTally(confirm) => confirm.run(),
            StepCommands::RenderTemplate(render) => render.run(),
            StepCommands::GenerateVoters(render) => render.run(),
            StepCommands::DuplicateVotes(render) => render.run(),
            StepCommands::CreateApplications(render) => render.run(),
            StepCommands::CreateElectoralLogs(render) => render.run(),
            StepCommands::HashPassword(render) => render.run(),
            StepCommands::UpdateEventVotingStatus(update_event_voting_status) => {
                update_event_voting_status.run();
            }
            StepCommands::UpdateElectionVotingStatus(update_election_voting_status) => {
                update_election_voting_status.run();
            }
            StepCommands::DownloadTallyResults(download) => download.run(),
            StepCommands::GeneratePreviewUrl(render) => render.run(),
        },
    }
}
