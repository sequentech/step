// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's keys file: written by `trustee generate`, read by `trustee
//! start`. Its content is never logged; its public keys are.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use anyhow::{Context as _, Result};
use protocol_board::{TrusteePublicKeys, TrusteeSecrets};
use tracing::info;

/// Readable and writable by its owner only.
const KEYS_FILE_MODE: u32 = 0o600;

/// Fresh secrets for a new trustee. With a path, they are written to a new
/// keys file there and the public keys printed to standard output; without
/// one, the keys file goes to standard output and the public keys to standard
/// error, so that the output can be stored as it is.
pub(crate) fn generate(path: Option<&Path>) -> Result<()> {
    let secrets = TrusteeSecrets::generate();
    let keys_file = secrets.to_toml()?;
    let public_keys = secrets.public_keys()?;
    match path {
        Some(path) => {
            write_new(path, &keys_file)?;
            print_public_keys(&mut io::stdout(), &public_keys)
                .context("printing the public keys")
        }
        None => {
            io::stdout()
                .write_all(keys_file.as_bytes())
                .context("printing the keys file")?;
            print_public_keys(&mut io::stderr(), &public_keys)
                .context("printing the public keys")
        }
    }
}

/// The secrets of the keys file at `path`, which must exist: a trustee never
/// runs on secrets it made up at start.
pub(crate) fn load(path: &Path) -> Result<TrusteeSecrets> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("reading the keys file {}", path.display()))?;
    let secrets = TrusteeSecrets::parse(&contents)
        .with_context(|| format!("the keys file {}", path.display()))?;
    let public_keys = secrets.public_keys()?;
    info!(
        public_key = %public_keys.signing_public_key,
        share_encryption_public_key = %public_keys.share_encryption_public_key,
        "loaded the keys file {}",
        path.display()
    );
    Ok(secrets)
}

/// Write a keys file that did not exist.
pub(crate) fn write_new(path: &Path, contents: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(KEYS_FILE_MODE)
        .open(path)
        .with_context(|| {
            format!("creating the keys file {}", path.display())
        })?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
        .with_context(|| format!("writing the keys file {}", path.display()))
}

/// Issue one independent random password per named trustee, without
/// enrolling or activating anyone. Never write to the medium or overwrite.
pub(crate) fn bootstrap_passwords(
    path: &Path,
    trustees: &[String],
) -> Result<()> {
    let checked: BTreeSet<_> = trustees.iter().collect();
    if checked.len() != trustees.len()
        || checked.len() < 6
        || checked.len() > 64
    {
        anyhow::bail!("provide between six and 64 distinct trustee names");
    }
    for name in &checked {
        let bytes = name.as_bytes();
        if !(1..=42).contains(&bytes.len())
            || !bytes[0].is_ascii_alphanumeric()
            || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
            || !bytes.iter().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-'
            })
        {
            anyhow::bail!("invalid trustee name");
        }
    }
    let mut random = File::open("/dev/urandom")
        .context("opening the system random source")?;
    DirBuilder::new()
        .mode(0o700)
        .create(path)
        .context("creating a new private trustee output directory")?;
    for name in checked {
        let mut bytes = [0_u8; 32];
        random
            .read_exact(&mut bytes)
            .context("reading the system random source")?;
        let mut password = String::with_capacity(64);
        for byte in bytes {
            write!(&mut password, "{byte:02x}")?;
        }
        write_new(&path.join(format!("{name}.password")), &password)?;
    }
    Ok(())
}

/// The public keys under the names of the `trustee` columns that store them.
fn print_public_keys(
    out: &mut impl Write,
    public_keys: &TrusteePublicKeys,
) -> io::Result<()> {
    writeln!(out, "public_key: {}", public_keys.signing_public_key)?;
    writeln!(
        out,
        "share_encryption_public_key: {}",
        public_keys.share_encryption_public_key
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn bootstrap_creates_six_distinct_private_files_without_reissuing() {
        let tmp = tempfile::tempdir().unwrap();
        let names: Vec<_> = (1..=6).map(|n| format!("trustee-{n}")).collect();
        let path = tmp.path().join("issued");
        bootstrap_passwords(&path, &names).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let mut unique = BTreeSet::new();
        for name in &names {
            let file = path.join(format!("{name}.password"));
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let value = fs::read_to_string(&file).unwrap();
            assert_eq!(value.len(), 64);
            assert!(value.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(unique.insert(value));
        }
        assert!(bootstrap_passwords(&path, &names).is_err());
        assert!(
            bootstrap_passwords(&tmp.path().join("too-few"), &names[..5])
                .is_err()
        );
        let mut duplicate = names.clone();
        duplicate[0] = duplicate[1].clone();
        assert!(
            bootstrap_passwords(&tmp.path().join("duplicate"), &duplicate)
                .is_err()
        );
        let mut invalid = names;
        invalid[0] = "../escape".into();
        assert!(
            bootstrap_passwords(&tmp.path().join("invalid"), &invalid).is_err()
        );
    }

    /// What `generate` writes is readable by its owner only, and is what
    /// `start` loads.
    #[test]
    fn a_generated_keys_file_is_private_to_its_owner_and_starts_the_trustee() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trustee.toml");
        generate(Some(path.as_path())).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(load(&path).unwrap().to_toml().unwrap(), written);
    }

    /// A trustee's keys are never lost to a second `generate`.
    #[test]
    fn generating_over_an_existing_keys_file_is_refused_and_keeps_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trustee.toml");
        generate(Some(path.as_path())).unwrap();
        let first = fs::read_to_string(&path).unwrap();

        let refused = generate(Some(path.as_path())).unwrap_err();
        let error = format!("{refused:#}");
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), first);
    }

    /// `start` never runs on secrets of its own making.
    #[test]
    fn start_refuses_to_run_without_its_keys_file_and_makes_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trustee.toml");

        let error = format!("{:#}", load(&path).unwrap_err());
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert!(!path.exists());
    }

    /// A keys file that does not parse is named in the error.
    #[test]
    fn start_refuses_a_keys_file_it_cannot_read_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trustee.toml");
        fs::write(&path, "signing_key = \"\"\n").unwrap();

        let error = format!("{:#}", load(&path).unwrap_err());
        assert!(error.contains(&path.display().to_string()), "{error}");
    }
}
