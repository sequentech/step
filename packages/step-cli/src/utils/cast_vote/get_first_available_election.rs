use crate::types::config::ConfigData;
use crate::utils::cast_vote::{get_elections};
use crate::utils::cast_vote::get_ballot_styles::{get_ballot_styles_fn, BallotStyle};
use crate::utils::cast_vote::get_elections::{Election};

pub fn get_first_available_election(config: &ConfigData) -> Result<(BallotStyle, Election), Box<dyn std::error::Error>> {
    // Get all ballot styles
    let ballot_styles = get_ballot_styles_fn(config)?;
    
    println!("ballot_styles: {:?}", ballot_styles.len());
    // Get election IDs from ballot styles
    let election_ids: Vec<String> = ballot_styles.iter()
        .map(|style| style.election_id.clone())
        .collect();

    println!("election_ids: {:?}", election_ids);
    // Get elections
    let elections = get_elections::get_elections(config, election_ids)?;

    // Find the first election that is published and open for voting
    let first_election = elections
        .iter()
        .find(|e| !e.name.starts_with("Test"))
        .map(|e| e.clone())
        .ok_or("No non-test elections found")?;

    println!("open_election: {:?}", first_election);
    // Get the corresponding ballot style
    let ballot_style = ballot_styles.into_iter()
        .find(|s| s.election_id == first_election.id)
        .ok_or("No ballot style found for open election")?;
    println!("ballot_style: {:?}", ballot_style.id);
    Ok((ballot_style, first_election.clone()))
}