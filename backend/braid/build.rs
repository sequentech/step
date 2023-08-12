fn cargo_warn_apply(args: &[&str]) {
    for arg in args {
        println!("cargo:warning=Applying {arg}");
        println!("{arg}");
    }
}

fn main() {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["proto/immudb/immudb.proto"],
            &["proto/immudb"],
        )
        .unwrap();

    cargo_warn_apply(&["cargo:rerun-if-changed=proto/immudb/immudb.proto"]);
}
