// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const UPDATE_ELECTION_INITIALIZATION_REPORT = gql`
    mutation UpdateElectionInitializationReport(
        $id: uuid!
        $initializationReportGenerated: Boolean!
    ) {
        update_sequent_backend_election(
            where: {id: {_eq: $id}}
            _set: {initialization_report_generated: $initializationReportGenerated}
        ) {
            returning {
                id
                initialization_report_generated
            }
        }
    }
`
