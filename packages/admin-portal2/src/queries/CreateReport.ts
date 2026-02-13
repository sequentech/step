// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CREATE_REPORT = gql`
    mutation InsertReport($object: sequent_backend_report_insert_input!) {
        insert_sequent_backend_report(objects: [$object]) {
            returning {
                id
                election_event_id
                tenant_id
                election_id
                report_type
                template_alias
                cron_config
                encryption_policy
            }
            affected_rows
        }
    }
`
