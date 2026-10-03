// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_EVENT_MESSAGING_CONFIG = gql`
    mutation UpdateEventMessagingConfig($electionEventId: uuid!, $config: jsonb!) {
        update_event_messaging_config(election_event_id: $electionEventId, config: $config) {
            errors
        }
    }
`
