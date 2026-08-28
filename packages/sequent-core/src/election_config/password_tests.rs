// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use std::collections::BTreeSet;

fn recipe() -> PasswordRecipe {
    PasswordRecipe {
        seed: "a-seed-the-wizard-minted".to_string(),
        ..Default::default()
    }
}

#[test]
fn the_two_halves_of_each_class_are_the_whole_class() {
    // The classes are spelled as "always safe" and "left out when avoiding
    // confusable", and a character in neither half — or in both — is a typo that
    // would quietly shrink or bias the alphabet.
    for (safe, confusable, whole) in [
        (
            LOWERCASE,
            LOWERCASE_CONFUSABLE,
            "abcdefghijklmnopqrstuvwxyz",
        ),
        (
            UPPERCASE,
            UPPERCASE_CONFUSABLE,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ),
        (DIGITS, DIGITS_CONFUSABLE, "0123456789"),
    ] {
        let halves: BTreeSet<char> =
            safe.chars().chain(confusable.chars()).collect();
        let expected: BTreeSet<char> = whole.chars().collect();
        assert_eq!(halves, expected, "'{safe}' + '{confusable}'");
        assert_eq!(
            safe.chars().count() + confusable.chars().count(),
            expected.len(),
            "'{safe}' and '{confusable}' share a character"
        );
    }
}

#[test]
fn the_same_seed_and_voter_give_the_same_password() {
    // The whole point. A delivery reopened from its own zip rebuilds the
    // passwords it sent out, so a client who lost the CSV is handed it again
    // rather than having every credential reissued.
    let one = recipe().password_for("m-1001").expect("a password");
    let again = recipe().password_for("m-1001").expect("a password");
    assert_eq!(one, again);
}

#[test]
fn two_voters_under_one_seed_get_different_passwords() {
    let recipe = recipe();
    assert_ne!(recipe.password_for("m-1001"), recipe.password_for("m-1002"));
}

#[test]
fn a_new_seed_reissues_every_password() {
    // Regenerating the seed is the one button that changes everybody's password,
    // and it has to actually do that.
    let before = recipe().password_for("m-1001");
    let after = PasswordRecipe {
        seed: "a-different-seed".to_string(),
        ..Default::default()
    }
    .password_for("m-1001");
    assert_ne!(before, after);
}

#[test]
fn a_password_is_as_long_as_it_was_asked_for() {
    for length in [MIN_LENGTH, 12, 20, 64, MAX_LENGTH] {
        let made = PasswordRecipe { length, ..recipe() }
            .password_for("m-1001")
            .expect("a password");
        assert_eq!(made.chars().count(), length, "asked for {length}");
    }
}

#[test]
fn a_length_outside_the_bounds_is_clamped_rather_than_refused() {
    // A recipe arrives from a JSON document somebody may have edited by hand, so
    // the bounds hold here as well as in the wizard's stepper.
    for (asked, expected) in
        [(0, MIN_LENGTH), (1, MIN_LENGTH), (9_000, MAX_LENGTH)]
    {
        let made = PasswordRecipe {
            length: asked,
            ..recipe()
        }
        .password_for("m-1001")
        .expect("a password");
        assert_eq!(made.chars().count(), expected, "asked for {asked}");
    }
}

#[test]
fn every_character_comes_from_the_alphabet_the_recipe_names() {
    // Over four class combinations and a hundred voters, because the failure this
    // catches is a character class leaking in — which one voter's password would
    // show only by luck.
    for (lowercase, uppercase, digits, symbols) in [
        (true, true, true, false),
        (true, false, false, false),
        (false, false, true, false),
        (true, true, true, true),
    ] {
        let recipe = PasswordRecipe {
            lowercase,
            uppercase,
            digits,
            symbols,
            ..recipe()
        };
        let allowed: BTreeSet<char> = recipe.alphabet().into_iter().collect();
        for voter in 0..100 {
            let made = recipe
                .password_for(&format!("m-{voter}"))
                .expect("a password");
            for character in made.chars() {
                assert!(
                    allowed.contains(&character),
                    "'{character}' is not in the alphabet for \
                     {lowercase}/{uppercase}/{digits}/{symbols}"
                );
            }
        }
    }
}

#[test]
fn avoiding_confusable_characters_leaves_them_out() {
    let recipe = PasswordRecipe {
        symbols: false,
        ..recipe()
    };
    let alphabet: BTreeSet<char> = recipe.alphabet().into_iter().collect();
    // Every confusable character, and only those: 3, 4, 6, 7 and 9 stay.
    for character in "0Oo1lIi258SZB".chars() {
        assert!(
            !alphabet.contains(&character),
            "'{character}' should be left out"
        );
    }

    for character in "34679".chars() {
        assert!(
            alphabet.contains(&character),
            "'{character}' is not confusable and should stay"
        );
    }

    // And they come back when the setting is off, or it would not be a setting.
    let loose = PasswordRecipe {
        avoid_confusable: false,
        ..recipe
    };
    let wider: BTreeSet<char> = loose.alphabet().into_iter().collect();
    for character in "0O1lIiZ".chars() {
        assert!(
            wider.contains(&character),
            "'{character}' should be offered"
        );
    }
}

#[test]
fn a_recipe_with_no_classes_or_no_seed_produces_nothing() {
    // Rather than a column of empty strings, which the platform's importer reads
    // as "give every one of these voters an empty credential".
    let no_classes = PasswordRecipe {
        lowercase: false,
        uppercase: false,
        digits: false,
        symbols: false,
        ..recipe()
    };
    assert!(no_classes.alphabet().is_empty());
    assert!(!no_classes.ready());
    assert_eq!(no_classes.password_for("m-1001"), None);

    let no_seed = PasswordRecipe {
        seed: "   ".to_string(),
        ..Default::default()
    };
    assert!(!no_seed.ready());
    assert_eq!(no_seed.password_for("m-1001"), None);
}

#[test]
fn the_alphabet_is_the_same_whatever_order_the_classes_were_added() {
    // The order is part of the derivation, so it is sorted rather than
    // concatenated: an edit that reorders the class constants must not change
    // anybody's password.
    let recipe = recipe();
    let mut sorted = recipe.alphabet();
    sorted.sort_unstable();
    assert_eq!(recipe.alphabet(), sorted);
}

#[test]
fn the_characters_are_spread_evenly_across_the_alphabet() {
    /// **A bias here is the defect nobody sees.** `byte % len` favours the first
    /// `256 % len` characters, which for a 49-character alphabet means the first
    /// eleven turn up about 28% more often than the rest — invisible in one
    /// password and a real weakening across ninety thousand.
    ///
    /// Measured over enough characters that the bound is comfortable rather than
    /// tight: a chi-squared test would be better statistics and worse as a
    /// regression test, because it fails occasionally by design.
    fn spread(recipe: &PasswordRecipe, voters: usize) -> Vec<usize> {
        let alphabet = recipe.alphabet();
        let mut counts = vec![0usize; alphabet.len()];
        for voter in 0..voters {
            let made = recipe
                .password_for(&format!("m-{voter}"))
                .expect("a password");
            for character in made.chars() {
                let at = alphabet
                    .iter()
                    .position(|each| *each == character)
                    .expect("from the alphabet");
                counts[at] += 1;
            }
        }
        counts
    }

    let recipe = PasswordRecipe {
        length: 20,
        ..recipe()
    };
    let counts = spread(&recipe, 2_000);
    let total: usize = counts.iter().sum();
    let expected = total as f64 / counts.len() as f64;
    let (low, high) = (expected * 0.75, expected * 1.25);
    for (at, count) in counts.iter().enumerate() {
        assert!(
            (*count as f64) > low && (*count as f64) < high,
            "character {at} turned up {count} times, expected about {expected:.0}"
        );
    }
}

#[test]
fn a_password_does_not_change_when_the_rest_of_the_row_does() {
    // Derived from the username alone, on purpose: a voter whose surname is
    // corrected, or who gains an email address, must not have their credential
    // change under them between one build and the next.
    let recipe = recipe();
    assert_eq!(recipe.password_for("m-1001"), recipe.password_for("m-1001"),);
    // The username *is* the identity, so a different one is a different voter.
    assert_ne!(recipe.password_for("m-1001"), recipe.password_for("M-1001"),);
}
