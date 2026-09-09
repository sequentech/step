// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
macro_rules! dispatch_db {
    ($self:expr, $method:ident ($($arg:expr),* $(,)?)) => {{
        #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
        compile_error!("enable at least one db backend");

        match $self {
            #[cfg(feature="sqlite")]
            $crate::db::common::BoardDb::Sqlite(inner) => inner.$method($($arg),*).await,

            #[cfg(feature="postgres")]
            $crate::db::common::BoardDb::Postgres(inner) => inner.$method($($arg),*).await,
        }}
    }
}
pub(super) use dispatch_db;
