// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic audit rows of the main and the IAM databases.
import type {PgAuditRow} from "@/gql/graphql"

type AuditRecord = Omit<PgAuditRow, "__typename">

/** The audit service reports `server_timestamp` in microseconds since the epoch. */
export const hasuraAuditRecords = (): AuditRecord[] => [
    {
        id: 1,
        audit_type: "SESSION",
        class: "WRITE",
        command: "UPDATE",
        dbname: "hasura",
        server_timestamp: 1768478400000000,
        session_id: "session-0001",
        statement: "UPDATE sequent_backend.election SET status = $1",
        user: "hasura",
    },
]

export const keycloakAuditRecords = (): AuditRecord[] => [
    {
        id: 2,
        audit_type: "SESSION",
        class: "WRITE",
        command: "INSERT",
        dbname: "keycloak",
        server_timestamp: 1768478730000000,
        session_id: "session-0002",
        statement: "INSERT INTO user_entity (id, username) VALUES ($1, $2)",
        user: "keycloak",
    },
]
