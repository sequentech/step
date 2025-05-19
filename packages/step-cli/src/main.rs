// // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only

mod commands;
mod tests;
mod types;
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "seq",
    version = "1.0",
    about = "CLI tool for managing Sequent tasks"
)]
struct Cli {
    #[command(subcommand)]
    command: MainCommand,
}

#[derive(Subcommand)]
enum MainCommand {
    #[command(subcommand)]
    Step(StepCommands),
}

#[derive(Subcommand)]
enum StepCommands {
    Config(commands::configure::Config),
    CreateElectionEvent(commands::create_election_event::CreateElectionEventCLI),
    CreateElection(commands::create_election::CreateElection),
    CreateContest(commands::create_contest::CreateContest),
    CreateCandidate(commands::create_candidate::CreateCandidate),
    CreateArea(commands::create_area::CreateArea),
    CreateAreaContest(commands::create_area_contest::CreateAreaContest),
    CreateVoter(commands::create_voter::CreateVoter),
    ExportCastVotes(commands::export_cast_votes::ExportCastVotes),
    UpdateVoter(commands::update_voter::UpdateVoter),
    ImportElection(commands::import_election_event::ImportElectionEventFile),
    Publish(commands::publish_changes::PublishChanges),
    RefreshToken(commands::refresh_token::Refresh),
    StartKeyCeremony(commands::start_key_ceremony::StartKeyCeremony),
    CompleteKeyCeremony(commands::complete_key_ceremony::Complete),
    StartTally(commands::start_tally::StartTallyCeremony),
    UpdateTally(commands::update_tally_status::UpdateTallyStatus),
    ConfirmKeyTally(commands::confirm_tally_ceremoney_key::ConfirmKeyForTally),
    RenderTemplate(commands::render_template::RenderTemplate),
    GenerateVoters(commands::generate_voters::GenerateVoters),
    DuplicateVotes(commands::duplicate_votes::DuplicateVotes),
    CreateApplications(commands::create_applications::CreateApplications),
    CreateElectoralLogs(commands::create_electoral_logs::CreateElectoralLogs),
    HashPassword(commands::hash_passwords::HashPasswords),
    UpdateEventVotingStatus(commands::update_event_voting_status::UpdateElectionEventVotingStatus),
    UpdateElectionVotingStatus(commands::update_election_voting_status::UpdateElectionVotingStatusCommand),
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
            StepCommands::ImportElection(import) => import.run(),
            StepCommands::CreateVoter(create_voter) => create_voter.run(),
            StepCommands::UpdateVoter(update_voter) => update_voter.run(),
            StepCommands::Publish(publish_ballot) => publish_ballot.run(),
            StepCommands::RefreshToken(refresh) => refresh.run(),
            StepCommands::StartKeyCeremony(start) => start.run(),
            StepCommands::CompleteKeyCeremony(complete) => complete.run(),
            StepCommands::StartTally(start) => start.run(),
            StepCommands::UpdateTally(update) => update.run(),
            StepCommands::ConfirmKeyTally(confirm) => confirm.run(),
            StepCommands::RenderTemplate(render) => render.run(),
            StepCommands::GenerateVoters(render) => render.run(),
            StepCommands::DuplicateVotes(render) => render.run(),
            StepCommands::CreateApplications(render) => render.run(),
            StepCommands::CreateElectoralLogs(render) => render.run(),
            StepCommands::HashPassword(render) => render.run(),
            StepCommands::UpdateEventVotingStatus(update_event_voting_status) => update_event_voting_status.run(),
            StepCommands::UpdateElectionVotingStatus(update_election_voting_status) => update_election_voting_status.run(),
        },
    }
}
