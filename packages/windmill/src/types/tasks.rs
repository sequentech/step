// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
#[strum(serialize_all = "PascalCase")]
pub enum ETasksExecution {
    #[strum(serialize = "Export Election Event")]
    EXPORT_ELECTION_EVENT,
    #[strum(serialize = "Import Candidates")]
    IMPORT_CANDIDATES,
    #[strum(serialize = "Import Voters")]
    IMPORT_USERS,
    #[strum(serialize = "Import Election Event")]
    IMPORT_ELECTION_EVENT,
}
