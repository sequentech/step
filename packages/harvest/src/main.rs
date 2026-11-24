// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[rocket::launch]
async fn rocket() -> _ {
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    if command.as_deref() == Some(std::ffi::OsStr::new("serve-operational-product-setup-ca")) {
        #[cfg(all(unix, feature = "operational-signing-product", not(feature = "operational-signing-demo-fixture")))]
        {
            if args.next().as_deref() != Some(std::ffi::OsStr::new("--installation")) {
                panic!("private enrollment CA requires --installation <immutable installation root>");
            }
            let root = args.next().filter(|_| args.next().is_none()).map(std::path::PathBuf::from)
                .expect("private enrollment CA requires one installation root");
            return harvest::operational_setup_ca_runtime::rocket(&root).await
                .expect("private enrollment CA installation refused");
        }
        #[cfg(not(all(unix, feature = "operational-signing-product", not(feature = "operational-signing-demo-fixture"))))]
        panic!("private enrollment CA is not supported by this binary");
    }
    if command.as_deref() == Some(std::ffi::OsStr::new("serve-operational-product-ca")) {
        #[cfg(all(unix, feature = "operational-signing-private-ca-candidate", not(feature = "operational-signing-demo-fixture")))]
        {
            if args.next().as_deref() != Some(std::ffi::OsStr::new("--installation")) {
                panic!("private positive CA requires --installation <immutable installation root>");
            }
            let root = args.next().filter(|_| args.next().is_none()).map(std::path::PathBuf::from)
                .expect("private positive CA requires a single installation root");
            return harvest::operational_gold_ca_runtime::rocket(&root).await
                .expect("private product CA installation refused");
        }
        #[cfg(not(all(unix, feature = "operational-signing-private-ca-candidate", not(feature = "operational-signing-demo-fixture"))))]
        panic!("private positive CA is not supported by this binary");
    }
    harvest::rocket().await
}
