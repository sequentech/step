// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::env;
use std::path::Path;
use std::process::Command;

fn run_command(command: &str, args: &[&str]) {
    let args_joined = args.join(" ");
    println!("cargo:warning=Running command: command={command} {args_joined}");
    let command_output = Command::new(command)
        .current_dir("./trillian-board/")
        .args(args)
        .output()
        .expect("failed to execute process");
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    let status = command_output.status;

    println!("cargo:warning=Running command: exit_status={status:?}");
    println!("cargo:warning=Running command: stdout={stdout}");
    println!("cargo:warning=Running command: stderr={stderr}");
    assert!(
        status.success(),
        "{command} {args_joined} command was not successful"
    );
}

fn cargo_warn_apply(args: &[&str]) {
    for arg in args {
        println!("cargo:warning=Applying {arg}");
        println!("{arg}");
    }
}

fn main() {
    // compile the grpc code structs with tonic
    tonic_build::configure()
        // Use bcrypt & padded base64 serialization in serde for EntryData.data,
        // to be compatible with grpcurl in the `src/bin/sign.rs` binary. Please
        // note:
        // - That this requires that the file adding the
        // `tonic::include_proto!("bulletin_board");` macro, contains a `use
        // serde_with::serde_as;` to work.
        // - That in order to work, the `#[serde_as]` for any struct needs to be
        //   appended before `#[derive(..)]`. 
        //
        // More info:
        // https://docs.rs/serde_with/latest/serde_with/base64/struct.Base64.html
        .type_attribute(
            ".bulletin_board.NewDataEntry",
            "#[serde_as]",
        )
        .field_attribute(
            ".bulletin_board.NewDataEntry.data",
            "#[serde_as(as = \"::serde_with::base64::Base64\")]",
        )
        // Implement serialization traits for all input message types
        .type_attribute(
            ".bulletin_board.Board",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.BoardEntryData",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.BoardEntry",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.ListBoardsRequest",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.User",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.Role",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.UserRole",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.Permissions",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.CreateBoardRequest",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.ListEntriesRequest",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.NewDataEntry",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .type_attribute(
            ".bulletin_board.AddEntriesRequest",
            "#[derive(::serde::Serialize, ::serde::Deserialize, ::borsh::BorshDeserialize, ::borsh::BorshSerialize)]",
        )
        .compile(&["proto/bulletin_board.proto"], &["proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));

    let out_path = env::var("out").unwrap_or_else(|_| String::from("."));

    // Ensure that the rpc protocol file triggers a rebuild
    cargo_warn_apply(&["cargo:rerun-if-changed=proto/bulletin_board.proto"]);

    // If we are not building the server code stop here
    if env::var("CARGO_FEATURE_BUILD_SERVER").is_err() {
        return;
    }
    // Ensure that the go-code triggers a rebuild
    cargo_warn_apply(&[
        "cargo:rerun-if-changed=trillian-board/storage/fs/fs.go",
        "cargo:rerun-if-changed=trillian-board/main.go",
        "cargo:rerun-if-changed=trillian-board/go.sum",
    ]);

    // detect and apply the debug vs release mode from rust to go
    let env_profile =
        std::env::var("PROFILE").unwrap_or_else(|_| String::from("debug"));
    let go_build_profile = match env_profile.as_str() {
        "release" => vec!["-ldflags", "-s -w"],
        _ => vec![],
    };

    // build the go wrapping code around trillian as a static library
    let lib_path = Path::new(&out_path).join("libtrillian_board.a");
    let lib_path_str = lib_path.to_string_lossy();
    let go_build_command = [
        vec!["build"],
        go_build_profile,
        vec!["-buildmode=c-archive", "-o", &lib_path_str, "main.go"],
    ]
    .concat();
    run_command("go", &go_build_command);

    // indicate -L and -l to the compiler to link against the just-built
    // trillian go library
    cargo_warn_apply(&[
        format!("cargo:rustc-link-search=native={}", &out_path).as_str(),
        "cargo:rustc-link-lib=static=trillian_board",
    ]);
}
