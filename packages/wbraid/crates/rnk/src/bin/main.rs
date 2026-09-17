// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use cryptography::context::RistrettoCtx as RCtx;
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair};
use cryptography::groups::Ristretto255Group;
use cryptography::groups::ristretto255::RistrettoElement;
use cryptography::traits::groups::CryptographicGroup;
use rnk::*;

type Ctx = RCtx;
type RGroup = Ristretto255Group;

fn main() {
    // Set up ballot encoding using combinatorial ranking
    println!("Setting up ballot structure...");

    // Create ice cream contest
    let ice_cream_set = CombinationSet::new(
        "ice_cream".to_string(),
        &[
            "vanilla".to_string(),
            "chocolate".to_string(),
            "strawberry".to_string(),
            "mint".to_string(),
        ],
        1,
    );

    // Create color contest
    let color_set = CombinationSet::new(
        "color".to_string(),
        &[
            "red".to_string(),
            "blue".to_string(),
            "green".to_string(),
            "purple".to_string(),
        ],
        1,
    );

    // Combine into a product set (clone sets to keep references for later use)
    let ballot_set = ProductSet::new(
        "ballot",
        vec![ice_cream_set.clone().into(), color_set.clone().into()],
    );

    // Generate keypair for the election
    let keypair = KeyPair::<Ctx>::generate();

    // Cast 20 ballots with a mix of choices
    println!("Casting 20 encrypted ballots...");

    // Vote data: (ice_cream_index, color_index) where indices are 0-3
    let votes = [
        (0, 1),
        (1, 2),
        (2, 0),
        (3, 3),
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 0),
        (0, 3),
        (1, 0),
        (2, 1),
        (3, 2),
        (0, 2),
        (1, 3),
        (2, 0),
        (3, 1),
        (0, 2),
        (1, 0),
        (2, 3),
        (3, 2),
    ];

    let ice_cream_flavors = ["vanilla", "chocolate", "strawberry", "mint"];
    let colors = ["red", "blue", "green", "purple"];

    let mut encrypted_ballots = Vec::new();

    for (i, &(ice_cream_idx, color_idx)) in votes.iter().enumerate() {
        // Create individual choice objects for each contest
        let ice_cream_choice = ice_cream_flavors[ice_cream_idx];
        let color_choice = colors[color_idx];

        // Create Combination objects for each contest
        let ice_cream_combination =
            Combination::from_set(&ice_cream_set, vec![ice_cream_choice.to_string()]).unwrap();
        let color_combination =
            Combination::from_set(&color_set, vec![color_choice.to_string()]).unwrap();

        // Create product ballot object using RankedValue
        let ballot_product = Product::from_set(
            &ballot_set,
            vec![
                // We use conversions from a variant struct to the
                // enum provided by the derive_more crate
                ice_cream_combination.into(),
                color_combination.into(),
            ],
        )
        .unwrap();

        // Rank the combined ballot
        let rank = ballot_set.rank_product(&ballot_product).unwrap();

        // Convert rank to bytes
        let rank_bytes = rank.to_be_bytes();

        // Encode bytes into crypto group element
        let encoded_vote: [RistrettoElement; 1] = RGroup::encode_bytes(&rank_bytes).unwrap();

        // Encrypt
        let ciphertext: Ciphertext<Ctx, 1> = keypair.encrypt(&encoded_vote);

        encrypted_ballots.push(ciphertext);

        println!(
            "   Ballot {}: ({}, {}) -> rank {} -> encrypted",
            i + 1,
            ice_cream_choice,
            color_choice,
            rank
        );
    }

    println!(
        "   {} ballots encrypted successfully!",
        encrypted_ballots.len()
    );
    println!();

    // Decrypt all ballots and reverse the encoding process
    println!("Decrypting and tallying ballots...");

    // Initialize counters for each choice
    let mut ice_cream_counts = [0; 4]; // vanilla, chocolate, strawberry, mint
    let mut color_counts = [0; 4]; // red, blue, green, purple

    for (i, ciphertext) in encrypted_ballots.iter().enumerate() {
        // Decrypt the ciphertext to get group element
        let decrypted_elements = keypair.decrypt(ciphertext);

        // Decode group element back to bytes
        let decoded_bytes: [u8; 8] = RGroup::decode_bytes(&decrypted_elements).unwrap();

        // Convert bytes back to rank
        let rank = u64::from_be_bytes(decoded_bytes);

        // Unrank to get original ballot choice
        let ballot_product = ballot_set.unrank_product(rank);

        // Extract individual choices from RankedValue
        let ice_cream_combination = ballot_product.get_values()[0].as_combination().unwrap();
        let color_combination = ballot_product.get_values()[1].as_combination().unwrap();

        let ice_cream_choice = &ice_cream_combination.get_values()[0];
        let color_choice = &color_combination.get_values()[0];

        // Update counters
        match ice_cream_choice.as_str() {
            "vanilla" => ice_cream_counts[0] += 1,
            "chocolate" => ice_cream_counts[1] += 1,
            "strawberry" => ice_cream_counts[2] += 1,
            "mint" => ice_cream_counts[3] += 1,
            _ => {}
        }

        match color_choice.as_str() {
            "red" => color_counts[0] += 1,
            "blue" => color_counts[1] += 1,
            "green" => color_counts[2] += 1,
            "purple" => color_counts[3] += 1,
            _ => {}
        }

        println!(
            "   Ballot {}: decrypted -> rank {} -> ({}, {})",
            i + 1,
            rank,
            ice_cream_choice,
            color_choice
        );
    }

    println!();

    println!("Final Election Results:");
    println!("   Ice Cream Contest:");
    println!("     Vanilla:    {}", ice_cream_counts[0]);
    println!("     Chocolate:  {}", ice_cream_counts[1]);
    println!("     Strawberry: {}", ice_cream_counts[2]);
    println!("     Mint:       {}", ice_cream_counts[3]);
    println!();
    println!("   Color Contest:");
    println!("     Red:    {}", color_counts[0]);
    println!("     Blue:   {}", color_counts[1]);
    println!("     Green:  {}", color_counts[2]);
    println!("     Purple: {}", color_counts[3]);
    println!();

    let total_votes = ice_cream_counts.iter().sum::<i32>();
    println!("   Total ballots: {}", total_votes);
    println!();

    // Verify against original votes
    let mut expected_ice_cream = [0; 4];
    let mut expected_color = [0; 4];

    for &(ice_cream_idx, color_idx) in votes.iter() {
        expected_ice_cream[ice_cream_idx] += 1;
        expected_color[color_idx] += 1;
    }

    println!("   Ice Cream Contest:");
    for i in 0..4 {
        let flavor = ice_cream_flavors[i];
        println!(
            "     {}: Expected {}, Got {} ✓",
            flavor, expected_ice_cream[i], ice_cream_counts[i]
        );
    }
    println!("   Color Contest:");
    for i in 0..4 {
        let color = colors[i];
        println!(
            "     {}: Expected {}, Got {} ✓",
            color, expected_color[i], color_counts[i]
        );
    }

    let ice_cream_success = ice_cream_counts == expected_ice_cream;
    let color_success = color_counts == expected_color;
    assert!(ice_cream_success && color_success);
}
