// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {getOperationAST, Kind, print, valueFromASTUntyped, type DocumentNode} from "graphql"
import {
    GetLedgerMessageStatesDocument,
    GetMessagingAccountSummariesDocument,
    GetVoterUnsettledMessagesDocument,
} from "@/gql/graphql"
import {GET_MESSAGING_ACCOUNT_SUMMARIES} from "./GetMessagingAccounts"
import {GET_LEDGER_MESSAGE_STATES, GET_VOTER_UNSETTLED_MESSAGES} from "./GetLedgerMessages"

function rootField(document: DocumentNode) {
    const field = getOperationAST(document)?.selectionSet.selections[0]
    if (field?.kind !== Kind.FIELD) throw new Error("Expected a root query field")
    return field
}

function where(document: DocumentNode, variables: Record<string, string | string[]>) {
    const argument = rootField(document).arguments?.find(({name}) => name.value === "where")
    if (!argument) throw new Error("Expected a scoped query")
    return valueFromASTUntyped(argument.value, variables)
}

test("generated account summaries preserve the safe columns and tenant scope", () => {
    expect(print(GetMessagingAccountSummariesDocument)).toBe(print(GET_MESSAGING_ACCOUNT_SUMMARIES))
    expect(
        rootField(GetMessagingAccountSummariesDocument).selectionSet?.selections.map((field) =>
            field.kind === Kind.FIELD ? field.name.value : field.kind
        )
    ).toEqual([
        "id",
        "tenant_id",
        "channel",
        "provider",
        "name",
        "sender",
        "limits",
        "provider_approval",
        "readiness",
        "status",
        "is_default",
        "created_at",
        "updated_at",
    ])
    expect(where(GetMessagingAccountSummariesDocument, {tenantId: "tenant-one"})).toEqual({
        tenant_id: {_eq: "tenant-one"},
    })
})

test("generated unsettled messages stay scoped to the tenant, event and voter", () => {
    expect(print(GetVoterUnsettledMessagesDocument)).toBe(print(GET_VOTER_UNSETTLED_MESSAGES))
    expect(
        where(GetVoterUnsettledMessagesDocument, {
            tenantId: "tenant-one",
            electionEventId: "event-one",
            voterId: "voter-one",
        })
    ).toEqual({
        tenant_id: {_eq: "tenant-one"},
        election_event_id: {_eq: "event-one"},
        voter_id: {_eq: "voter-one"},
        direction: {_eq: "OUTBOUND"},
        state: {_in: ["FAILED", "UNKNOWN"]},
    })
})

test("generated ledger states constrain message IDs within the tenant", () => {
    expect(print(GetLedgerMessageStatesDocument)).toBe(print(GET_LEDGER_MESSAGE_STATES))
    expect(
        where(GetLedgerMessageStatesDocument, {tenantId: "tenant-one", ids: ["message-one"]})
    ).toEqual({
        tenant_id: {_eq: "tenant-one"},
        id: {_in: ["message-one"]},
    })
})
