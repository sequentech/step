// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// Outbound message attempts of an event, counted per channel and state and
// aliased `{CHANNEL}_{STATE}`.
export const GET_MESSAGE_DELIVERY_STATS = gql`
    query GetMessageDeliveryStats($tenantId: uuid!, $electionEventId: uuid!) {
        EMAIL_QUEUED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "EMAIL"}
                state: {_eq: "QUEUED"}
            }
        ) {
            aggregate {
                count
            }
        }
        EMAIL_ACCEPTED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "EMAIL"}
                state: {_eq: "ACCEPTED"}
            }
        ) {
            aggregate {
                count
            }
        }
        EMAIL_DELIVERED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "EMAIL"}
                state: {_eq: "DELIVERED"}
            }
        ) {
            aggregate {
                count
            }
        }
        EMAIL_FAILED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "EMAIL"}
                state: {_eq: "FAILED"}
            }
        ) {
            aggregate {
                count
            }
        }
        EMAIL_UNKNOWN: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "EMAIL"}
                state: {_eq: "UNKNOWN"}
            }
        ) {
            aggregate {
                count
            }
        }
        SMS_QUEUED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "SMS"}
                state: {_eq: "QUEUED"}
            }
        ) {
            aggregate {
                count
            }
        }
        SMS_ACCEPTED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "SMS"}
                state: {_eq: "ACCEPTED"}
            }
        ) {
            aggregate {
                count
            }
        }
        SMS_DELIVERED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "SMS"}
                state: {_eq: "DELIVERED"}
            }
        ) {
            aggregate {
                count
            }
        }
        SMS_FAILED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "SMS"}
                state: {_eq: "FAILED"}
            }
        ) {
            aggregate {
                count
            }
        }
        SMS_UNKNOWN: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "SMS"}
                state: {_eq: "UNKNOWN"}
            }
        ) {
            aggregate {
                count
            }
        }
        WHATSAPP_QUEUED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "WHATSAPP"}
                state: {_eq: "QUEUED"}
            }
        ) {
            aggregate {
                count
            }
        }
        WHATSAPP_ACCEPTED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "WHATSAPP"}
                state: {_eq: "ACCEPTED"}
            }
        ) {
            aggregate {
                count
            }
        }
        WHATSAPP_DELIVERED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "WHATSAPP"}
                state: {_eq: "DELIVERED"}
            }
        ) {
            aggregate {
                count
            }
        }
        WHATSAPP_FAILED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "WHATSAPP"}
                state: {_eq: "FAILED"}
            }
        ) {
            aggregate {
                count
            }
        }
        WHATSAPP_UNKNOWN: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "WHATSAPP"}
                state: {_eq: "UNKNOWN"}
            }
        ) {
            aggregate {
                count
            }
        }
        VIBER_QUEUED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "VIBER"}
                state: {_eq: "QUEUED"}
            }
        ) {
            aggregate {
                count
            }
        }
        VIBER_ACCEPTED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "VIBER"}
                state: {_eq: "ACCEPTED"}
            }
        ) {
            aggregate {
                count
            }
        }
        VIBER_DELIVERED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "VIBER"}
                state: {_eq: "DELIVERED"}
            }
        ) {
            aggregate {
                count
            }
        }
        VIBER_FAILED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "VIBER"}
                state: {_eq: "FAILED"}
            }
        ) {
            aggregate {
                count
            }
        }
        VIBER_UNKNOWN: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "VIBER"}
                state: {_eq: "UNKNOWN"}
            }
        ) {
            aggregate {
                count
            }
        }
        MESSENGER_QUEUED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "MESSENGER"}
                state: {_eq: "QUEUED"}
            }
        ) {
            aggregate {
                count
            }
        }
        MESSENGER_ACCEPTED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "MESSENGER"}
                state: {_eq: "ACCEPTED"}
            }
        ) {
            aggregate {
                count
            }
        }
        MESSENGER_DELIVERED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "MESSENGER"}
                state: {_eq: "DELIVERED"}
            }
        ) {
            aggregate {
                count
            }
        }
        MESSENGER_FAILED: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "MESSENGER"}
                state: {_eq: "FAILED"}
            }
        ) {
            aggregate {
                count
            }
        }
        MESSENGER_UNKNOWN: sequent_backend_message_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                direction: {_eq: "OUTBOUND"}
                channel: {_eq: "MESSENGER"}
                state: {_eq: "UNKNOWN"}
            }
        ) {
            aggregate {
                count
            }
        }
    }
`
