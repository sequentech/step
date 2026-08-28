// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Passwords a delivery generates, rather than a client typing ninety of them.
//!
//! A census can always carry a `password` column — the platform's importer hashes
//! it and the Census screen has offered it all along — but somebody had to produce
//! the values, which in practice meant a spreadsheet formula nobody could
//! reproduce a week later.
//!
//! **One random thing, and it is not the passwords.** A [`PasswordRecipe`] carries
//! a *seed*, and every password is derived from that seed and the voter's own
//! username. So the same plan built twice gives the same passwords, a delivery
//! reopened from its own zip regenerates exactly what was sent out, and a client
//! who has lost the CSV can be handed it again without re-issuing credentials.
//! Regenerating the seed — one button in the wizard — reissues every password at
//! once, deliberately.
//!
//! The seed itself comes from the wizard, `crypto.getRandomValues`, which is why
//! there is no randomness in this module and none anywhere in `election_config`:
//! three comments in this crate already explain that a `getrandom` in the
//! WebAssembly build is a cost with no benefit, and a derivation needs a hash
//! rather than an RNG.
//!
//! **What this is not.** These are initial credentials distributed with a link,
//! not a password store: the value travels in clear text in
//! `export_voters-<id>.csv`, because the platform is what hashes it on import.
//! `carriesCredentials` in the wizard warns about exactly that file for exactly
//! that reason, and the warning fires whether the column was typed or derived.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// The column the platform's importer hashes.
///
/// `get_copy_from_query` in `import_users` treats the *presence* of this header as
/// "hash a password for each of these voters", which is why a recipe that is on
/// must fill every row and one that is off must not write the column at all.
pub const COLUMN: &str = "password";

/// Long enough to be worth having, short enough to read off a letter.
pub const DEFAULT_LENGTH: usize = 12;

/// Below this a derived password is not worth the trouble of deriving.
pub const MIN_LENGTH: usize = 6;

/// Keycloak's own ceiling, mirrored from `realm_password_policy`.
pub const MAX_LENGTH: usize = 128;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";

/// The characters a person reads back wrong, unless somebody says otherwise.
///
/// **A default rather than a rule.** It used to be three pairs of constants — each
/// class split into safe and confusable — which meant the list was a fact about
/// this file and not a thing a delivery could look at. It is the set most people
/// mean by "no look-alikes": `O`/`0`, `I`/`l`/`1`/`i`, `S`/`5`, `Z`/`2`, `B`/`8`.
///
/// A client whose members read their credentials off a screen in a good font may
/// want fewer left out; one reading them down a telephone may want more. So the
/// recipe carries the set, seeded with this.
pub const DEFAULT_EXCLUDED: &str = "01258BILOSZilo";

/// Punctuation a phone keypad and a handwritten note both survive.
///
/// Deliberately narrower than `realm_password_policy`'s set: no quotes, no
/// backslash, nothing a CSV, a shell or a spreadsheet formula treats specially.
/// A password that arrives mangled is worse than a shorter alphabet.
const SYMBOLS: &str = "!#$%&*+-=?@_";

/// How a recipe turns into the passwords a build carries.
///
/// Every field is `#[serde(default)]`, so a plan written before this existed
/// deserialises to the shape below and `skip_serializing_if` on the plan's own
/// field keeps such a document byte-identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasswordRecipe {
    /// How many characters. Clamped by [`MIN_LENGTH`] and [`MAX_LENGTH`].
    #[serde(default = "default_length")]
    pub length: usize,

    #[serde(default = "yes")]
    pub lowercase: bool,

    #[serde(default = "yes")]
    pub uppercase: bool,

    #[serde(default = "yes")]
    pub digits: bool,

    /// Off by default: a symbol is the character somebody mistypes reading it off
    /// a letter, and these are credentials people are read down a telephone.
    #[serde(default)]
    pub symbols: bool,

    /// Whether to leave out the characters in [`excluded`](Self::excluded).
    ///
    /// On by default, and it is the setting that pays for itself: the cost is
    /// about a fifth of the alphabet, and what it buys is not having to tell
    /// somebody on the telephone which kind of O it was.
    #[serde(default = "yes")]
    pub avoid_confusable: bool,

    /// Which characters those are, when the switch above is on.
    ///
    /// Seeded with [`DEFAULT_EXCLUDED`] and editable, because what looks alike
    /// depends on where a credential is read: off a screen in a good font, down a
    /// telephone, or from a letter somebody printed. Characters not in any chosen
    /// class are simply never drawn, so a stray one here costs nothing.
    ///
    /// Part of the recipe, so changing it changes the passwords — the same as
    /// changing the length or a class. That is the point of the seed being the
    /// only *random* input: everything else about a password is a decision
    /// somebody can see.
    #[serde(default = "default_excluded")]
    pub excluded: String,

    /// The one random thing. Generated by the wizard with
    /// `crypto.getRandomValues` and carried in the plan, which is what makes a
    /// rebuild reproduce the passwords it sent out.
    #[serde(default)]
    pub seed: String,
}

fn default_length() -> usize {
    DEFAULT_LENGTH
}

fn yes() -> bool {
    true
}

fn default_excluded() -> String {
    DEFAULT_EXCLUDED.to_string()
}

impl Default for PasswordRecipe {
    fn default() -> Self {
        PasswordRecipe {
            length: DEFAULT_LENGTH,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: false,
            avoid_confusable: true,
            excluded: default_excluded(),
            seed: String::new(),
        }
    }
}

impl PasswordRecipe {
    /// The characters a password may be made of, in a stable order.
    ///
    /// Stable because the order is part of the derivation: the same recipe has to
    /// give the same alphabet on every build, or reproducibility is a coin toss.
    /// Sorted rather than concatenated for the same reason — a future edit that
    /// reorders the class constants must not change anybody's passwords.
    pub fn alphabet(&self) -> Vec<char> {
        let mut characters: Vec<char> = Vec::new();
        for (chosen, class) in [
            (self.lowercase, LOWERCASE),
            (self.uppercase, UPPERCASE),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ] {
            if chosen {
                characters.extend(class.chars());
            }
        }
        if self.avoid_confusable {
            characters.retain(|character| !self.excluded.contains(*character));
        }
        characters.sort_unstable();
        characters.dedup();
        characters
    }

    /// Whether this recipe can produce anything at all.
    ///
    /// Two ways it cannot: no character class chosen, or no seed. Both are
    /// reported against the screen rather than guessed at, because a password of
    /// zero possible characters and a password derived from an empty seed are
    /// each a credential nobody should be issued.
    pub fn ready(&self) -> bool {
        !self.alphabet().is_empty() && !self.seed.trim().is_empty()
    }

    /// The password for one voter.
    ///
    /// `None` when the recipe is not [`ready`](Self::ready) — a caller that
    /// writes a column of empty strings would be handing every voter an empty
    /// credential, which is the failure `drop_empty_voter_columns` exists to
    /// prevent one layer down.
    ///
    /// **Derived, not drawn.** `HMAC-SHA256` keyed by the seed over a domain
    /// string and the username, in 32-byte blocks with a counter for as many
    /// bytes as the rejection sampling below needs. The username rather than the
    /// whole row because it is the one field the platform treats as identity: a
    /// voter whose surname is corrected must not have their password change with
    /// it.
    pub fn password_for(&self, username: &str) -> Option<String> {
        let alphabet = self.alphabet();
        if alphabet.is_empty() || self.seed.trim().is_empty() {
            return None;
        }
        let length = self.length.clamp(MIN_LENGTH, MAX_LENGTH);

        // Rejection sampling, so every character is equally likely. `byte %
        // alphabet.len()` would quietly favour the first `256 % len` characters —
        // and a bias in a credential generator is the kind of defect that is
        // invisible until somebody measures it.
        let size = alphabet.len();
        let limit = 256 - (256 % size.min(256));
        let mut out = String::with_capacity(length);
        let mut block: u32 = 0;
        while out.len() < length {
            for byte in self.bytes(username, block) {
                if (byte as usize) < limit {
                    out.push(alphabet[(byte as usize) % size]);
                    if out.len() == length {
                        break;
                    }
                }
            }
            block += 1;
            // A seed and a username can only be unlucky so many times: with the
            // narrowest alphabet this crate offers, thirty-two bytes a block, the
            // odds of a hundred blocks all being rejected are not a thing that
            // happens. The bound is here so a future alphabet of length 255 still
            // terminates rather than hanging a build.
            if block > 1_000 {
                return None;
            }
        }
        Some(out)
    }

    /// One 32-byte block of the derivation.
    fn bytes(&self, username: &str, block: u32) -> [u8; 32] {
        // The seed is the key, not part of the message: a secret belongs in the
        // key of a MAC, and `new_from_slice` accepts any length so a seed of any
        // shape the wizard mints is usable.
        let mut mac = Hmac::<Sha256>::new_from_slice(self.seed.as_bytes())
            .expect("HMAC accepts a key of any length");
        // A domain string, so the same seed used for something else later cannot
        // produce the same bytes. Versioned, so a change to the derivation is a
        // decision somebody makes rather than a surprise: bumping this reissues
        // every password, and that is exactly what it should mean.
        mac.update(b"sequent-voter-password-v1\0");
        mac.update(username.as_bytes());
        mac.update(b"\0");
        mac.update(&block.to_be_bytes());
        mac.finalize().into_bytes().into()
    }
}

#[cfg(test)]
#[path = "password_tests.rs"]
mod password_tests;
