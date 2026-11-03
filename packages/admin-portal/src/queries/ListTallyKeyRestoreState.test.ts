// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {FieldNode, Kind, OperationDefinitionNode, print} from "graphql"
import {LIST_TALLY_KEY_RESTORE_STATE} from "./ListTallyKeyRestoreState"

const TALLY_SESSION = "sequent_backend_tally_session"
const TALLY_SESSION_EXECUTION = "sequent_backend_tally_session_execution"

const rootField = (name: string): FieldNode | undefined =>
    LIST_TALLY_KEY_RESTORE_STATE.definitions
        .filter(
            (definition): definition is OperationDefinitionNode =>
                definition.kind === Kind.OPERATION_DEFINITION
        )
        .flatMap((operation) => operation.selectionSet.selections)
        .find(
            (selection): selection is FieldNode =>
                selection.kind === Kind.FIELD && selection.name.value === name
        )

const printedArgument = (fieldName: string, argumentName: string): string | undefined => {
    const value = rootField(fieldName)?.arguments?.find(
        (argument) => argument.name.value === argumentName
    )?.value
    return value && print(value)
}

const selectedFields = (fieldName: string): Array<string> | undefined =>
    rootField(fieldName)?.selectionSet?.selections.map((selection) =>
        selection.kind === Kind.FIELD ? selection.name.value : selection.kind
    )

describe("LIST_TALLY_KEY_RESTORE_STATE", () => {
    it("selects only the tally status and the trustee statuses", () => {
        expect(selectedFields(TALLY_SESSION)).toEqual(["id", "execution_status"])
        expect(selectedFields(TALLY_SESSION_EXECUTION)).toEqual(["tally_session_id", "status"])
    })

    it.each([TALLY_SESSION, TALLY_SESSION_EXECUTION])(
        "loads %s for the whole election event, with no page limit",
        (fieldName) => {
            expect(printedArgument(fieldName, "where")).toBe(
                "{tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $electionEventId}}"
            )
            expect(printedArgument(fieldName, "limit")).toBeUndefined()
            expect(printedArgument(fieldName, "offset")).toBeUndefined()
        }
    )

    it("keeps the latest execution of each tally session, ordering null dates last", () => {
        expect(printedArgument(TALLY_SESSION_EXECUTION, "distinct_on")).toBe("tally_session_id")
        expect(printedArgument(TALLY_SESSION_EXECUTION, "order_by")).toBe(
            "[{tally_session_id: asc}, {created_at: desc_nulls_last}, {id: desc}]"
        )
    })
})
