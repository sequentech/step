// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod cli;
mod config;
mod fixtures;
mod pipes;
mod utils;
use sequent_core::services::{pdf, reports};
use serde_json::Map;
use std::collections::HashMap;

use cli::console::ciccp_consolidation;
use pipes::{
    generate_reports::{GenerateReports, ReportData},
    mark_winners::MarkWinners,
};

use crate::pipes::error::{Error, Result};
use std::env;
/*
fn main() -> std::result::Result<(), Box<dyn std::error::Error + 'static>> {
    let cli = Cli::parse();
    init_log(true);

    match cli.command {
        Commands::Run(run) => {
            let config = run.validate()?;
            let mut state = State::new(&run, &config)?;

            while let Some(next_stage) = state.get_next() {
                let stage_name = next_stage.to_string();
                event!(Level::INFO, "Exec {}", stage_name);
                state.exec_next()?;
            }
        }
    }

    Ok(())
}*/
fn main() -> Result<()> {
    // Ensure correct number of arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <base_path> <folder_common>", args[0]);
        std::process::exit(1);
    }

    let base_path = &args[1];
    let folder_common = &args[2];

    let contest_result = ciccp_consolidation(base_path.as_str(), folder_common.as_str())?;
    let winners = MarkWinners::get_winners(&contest_result);

    let report = ReportData {
        election_name: "Consejeros por Razón de edad".to_string(),
        contest: contest_result.contest.clone(),
        contest_result: contest_result.clone(),
        area: None,
        winners,
        channel_type: None,
    };

    let computed_reports = GenerateReports::compute_reports(vec![report])?;
    let json_reports = serde_json::to_value(computed_reports)?;

    let mut variables_map = Map::new();

    variables_map.insert("reports".to_owned(), json_reports.clone());

    let mut template_map = HashMap::new();
    let report_base_html = include_str!("./resources/report_base_html.hbs");
    template_map.insert("report_base_html".to_string(), report_base_html.to_string());
    let report_base_pdf = include_str!("./resources/report_base_pdf.hbs");
    template_map.insert("report_base_pdf".to_string(), report_base_pdf.to_string());
    let report_content = include_str!("./resources/report_content.hbs").to_string();
    template_map.insert("report_content".to_string(), report_content);

    let render_html = reports::render_template(
        "report_base_html",
        template_map.clone(),
        variables_map.clone(),
    )
    .map_err(|e| {
        Error::UnexpectedError(format!(
            "Error during render_template_text from report.hbs template file: {}",
            e
        ))
    })?;

    //let aggregate_str = serde_json::to_string(&contest_result)?;
    println!("{}", render_html);

    Ok(())
}
