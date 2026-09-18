// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

fn cargo_warn_apply(args: &[&str]) {
    for arg in args {
        println!("cargo:warning=Applying {arg}");
        println!("{arg}");
    }
}

fn main() {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &["proto/immudb/immudb.proto"],
            // Imported Google protos already live below this root. Nonexistent
            // include paths make Cargo rerun this build script on every build,
            // invalidating electoral-log and the entire Windmill dependency tree.
            &["proto/immudb"],
        )
        .unwrap();

    cargo_warn_apply(&["cargo:rerun-if-changed=proto/immudb/immudb.proto"]);
}
