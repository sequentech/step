// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Kind} from "graphql"
import {SigningAction} from "@/lib/signing/types"
import {GET_SIGNING_RULE_CAPACITIES} from "./SigningSettings"

it("loads signer capacity and role choices for every configurable protected action", () => {
    const operation = GET_SIGNING_RULE_CAPACITIES.definitions.find(
        (definition) => definition.kind === Kind.OPERATION_DEFINITION
    )
    if (!operation || operation.kind !== Kind.OPERATION_DEFINITION) {
        throw new Error("A real capacity query is required")
    }
    const actions: string[] = []
    for (const field of operation.selectionSet.selections) {
        if (field.kind !== Kind.FIELD) throw new Error("Expected an aliased capacity field")
        const action = field.arguments?.find((argument) => argument.name.value === "action")?.value
        if (action?.kind !== Kind.STRING) throw new Error("Expected a concrete protected action")
        actions.push(action.value)
        expect(field.name.value).toBe("signingRuleCapacity")
        expect(field.alias?.value).toBe(action.value.replace(/-/g, "_"))
        expect(
            field.selectionSet?.selections.map((selection) => {
                if (selection.kind !== Kind.FIELD) throw new Error("Expected a capacity property")
                return selection.name.value
            })
        ).toEqual(
            expect.arrayContaining([
                "max",
                "posts",
                "posts_short",
                "posts_short_without_requester",
                "roles",
                "waiting",
                "config_version",
            ])
        )
    }
    expect(actions.sort()).toEqual(Object.values(SigningAction).sort())
})
