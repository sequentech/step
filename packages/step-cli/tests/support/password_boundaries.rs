// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! CSV credential conversion is an operator-facing file operation. Invalid
//! input must preserve both the source and any previously generated output.

use super::*;
use std::fs;
use tempfile::TempDir;

struct Files {
    directory: TempDir,
    command: HashPasswords,
}

impl Files {
    fn new(csv: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("voters.csv");
        let output = directory.path().join("credentials.csv");
        fs::write(&input, csv).unwrap();
        Self {
            command: HashPasswords {
                input_file: input.to_str().unwrap().into(),
                output_file: output.to_str().unwrap().into(),
                iterations: NonZeroU32::new(2).unwrap(),
            },
            directory,
        }
    }
}

#[test]
fn password_derivation_matches_independent_sha256_pbkdf2_vectors() {
    // Calculated independently with Python hashlib.pbkdf2_hmac, not with the
    // ring implementation under test. These pin algorithm, iterations and encoding.
    for (iterations, expected) in [
        (1, "Eg+2z/z4syxD5yJSVsT4N6hlSMkszDVICAWYfLcL4Xs="),
        (2, "rk0Mla9rRtMtCt/5KPBt0CowP47zwlHf1uLYWpVHTEM="),
        (4096, "xeR41ZKIyEGqUw22hFxMjZYok6ABzk4RpJY4c6qYE0o="),
    ] {
        assert_eq!(
            hash_password(
                &"password".into(),
                b"salt",
                &NonZeroU32::new(iterations).unwrap()
            )
            .unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn conversion_removes_plaintext_and_preserves_row_order_and_quoted_fields() {
    let fixture = Files::new(
        "username,password,note\nfirst,secret-one,\"Lee, Ada\"\nsecond,é-秘密,\"two\nlines\"\n",
    );
    fixture.command.run_hash_password().await.unwrap();
    let mut reader = csv::Reader::from_path(&fixture.command.output_file).unwrap();
    assert_eq!(
        reader.headers().unwrap().iter().collect::<Vec<_>>(),
        [
            "username",
            "note",
            "password_salt",
            "hashed_password",
            "num_of_iterations"
        ]
    );
    let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 2);
    for (row, (name, note, password)) in records.iter().zip([
        ("first", "Lee, Ada", "secret-one"),
        ("second", "two\nlines", "é-秘密"),
    ]) {
        assert_eq!(&row[0], name);
        assert_eq!(&row[1], note);
        assert_eq!(&row[4], "2");
        let salt = BASE64_STANDARD.decode(&row[2]).unwrap();
        let hash = BASE64_STANDARD.decode(&row[3]).unwrap();
        assert_eq!(salt.len(), 32);
        assert_eq!(hash.len(), 32);
        assert!(pbkdf2::verify(
            PBKDF2_ALGORITHM,
            fixture.command.iterations,
            &salt,
            password.as_bytes(),
            &hash
        )
        .is_ok());
    }
    let output = fs::read_to_string(&fixture.command.output_file).unwrap();
    assert!(!output.contains("secret-one"));
    assert!(!output.contains("é-秘密"));
}

#[tokio::test]
async fn malformed_input_preserves_a_previous_output_and_does_not_publish_partial_rows() {
    for csv in [
        "username,note\nfirst,missing-password\n",
        "username,password\nfirst,one\nsecond,two,extra\n",
    ] {
        let fixture = Files::new(csv);
        fs::write(&fixture.command.output_file, "previous successful export").unwrap();
        assert!(fixture.command.run_hash_password().await.is_err());
        assert_eq!(
            fs::read_to_string(&fixture.command.output_file).unwrap(),
            "previous successful export"
        );
        assert_eq!(
            fs::read_to_string(&fixture.command.input_file).unwrap(),
            csv
        );
    }
}

#[tokio::test]
async fn duplicate_password_columns_cannot_leave_a_plaintext_password_in_the_output() {
    let fixture = Files::new("username,password,password\nfirst,secret-one,secret-two\n");
    assert!(fixture.command.run_hash_password().await.is_err());
    assert!(!std::path::Path::new(&fixture.command.output_file).exists());
}

#[tokio::test]
async fn existing_credential_columns_are_rejected_instead_of_creating_ambiguous_headers() {
    for reserved in ["password_salt", "hashed_password", "num_of_iterations"] {
        let fixture = Files::new(&format!(
            "username,password,{reserved}\nfirst,secret-one,old-value\n"
        ));
        assert!(
            fixture.command.run_hash_password().await.is_err(),
            "{reserved}"
        );
        assert!(!std::path::Path::new(&fixture.command.output_file).exists());
    }
}

#[tokio::test]
async fn using_the_input_as_the_output_is_rejected_without_truncating_it() {
    const INPUT: &str = "username,password\nfirst,secret-one\n";
    let mut fixture = Files::new(INPUT);
    fixture.command.output_file = fixture.command.input_file.clone();
    assert!(fixture.command.run_hash_password().await.is_err());
    assert_eq!(
        fs::read_to_string(&fixture.command.input_file).unwrap(),
        INPUT
    );
}

#[tokio::test]
async fn a_header_only_csv_produces_a_valid_empty_export() {
    let fixture = Files::new("username,password\n");
    fixture.command.run_hash_password().await.unwrap();
    let output = fs::read_to_string(&fixture.command.output_file).unwrap();
    assert_eq!(
        output,
        "username,password_salt,hashed_password,num_of_iterations\n"
    );
    assert_eq!(
        fs::read_dir(fixture.directory.path()).unwrap().count(),
        2,
        "no abandoned temporary output"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn an_output_symlink_to_the_source_cannot_truncate_plaintext_input() {
    use std::os::unix::fs::symlink;

    let fixture = Files::new("username,password\nfirst,synthetic-secret\n");
    symlink(&fixture.command.input_file, &fixture.command.output_file).unwrap();
    let before = fs::read(&fixture.command.input_file).unwrap();
    assert!(fixture.command.run_hash_password().await.is_err());
    assert_eq!(fs::read(&fixture.command.input_file).unwrap(), before);
    assert!(fs::symlink_metadata(&fixture.command.output_file)
        .unwrap()
        .is_symlink());
}

#[tokio::test]
async fn a_destination_that_is_a_directory_is_preserved_without_temporary_files() {
    let fixture = Files::new("username,password\nfirst,synthetic-secret\n");
    fs::create_dir(&fixture.command.output_file).unwrap();
    let marker = std::path::Path::new(&fixture.command.output_file).join("keep.txt");
    fs::write(&marker, "keep this directory").unwrap();

    assert!(fixture.command.run_hash_password().await.is_err());
    assert_eq!(fs::read_to_string(marker).unwrap(), "keep this directory");
    assert_eq!(fs::read_dir(fixture.directory.path()).unwrap().count(), 2);
}
