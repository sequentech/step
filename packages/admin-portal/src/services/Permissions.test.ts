// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {getOperationRole} from "./Permissions"
import {IPermissions} from "@/types/keycloak"
import type {GraphQLRequest} from "@apollo/client"

// Keep the real shared predicate without loading the browser component barrel.
jest.mock("@sequentech/ui-core", () => require("../../../ui-core/src/utils/typechecks"))
const operation = (operationName?: string) => ({operationName}) as GraphQLRequest

it.each([
    ["sequent_backend_area", IPermissions.AREA_READ],
    ["insert_sequent_backend_candidate", IPermissions.CANDIDATE_CREATE],
    ["update_sequent_backend_candidate", IPermissions.CANDIDATE_WRITE],
    ["delete_sequent_backend_candidate", IPermissions.CANDIDATE_DELETE],
    ["sequent_backend_tally_results_publication", IPermissions.PUBLISH_RESULTS_READ],
    ["RevealVoterSecretAttribute", IPermissions.VOTER_READ],
])("requires the role for %s in both modes", (name, role) => {
    expect(getOperationRole(operation(name))).toBe(role)
    expect(getOperationRole(operation(name), true)).toBe(role)
})

it.each(["sequent_backend_keys_ceremony", "sequent_backend_tally_session_execution"])(
    "uses the trustee ceremony permission only in trustee mode: %s",
    (name) => {
        expect(getOperationRole(operation(name))).toBe(IPermissions.ADMIN_CEREMONY)
        expect(getOperationRole(operation(name), true)).toBe(IPermissions.TRUSTEE_CEREMONY)
    }
)
it("keeps the trustee user lookup separate from the admin fallback", () => {
    expect(getOperationRole(operation("getUsers"), true)).toBe(IPermissions.VOTER_READ)
    expect(getOperationRole(operation("getUsers"))).toBe(IPermissions.ADMIN_USER)
})
it.each([undefined, "", "unknown", "toString", "constructor", "__proto__"])(
    "falls back to an explicit admin permission for unknown operation %s",
    (name) => {
        // Prototype property names are unknown operations too, not permission values.
        expect(getOperationRole(operation(name))).toBe(IPermissions.ADMIN_USER)
        expect(getOperationRole(operation(name), true)).toBe(IPermissions.ADMIN_USER)
    }
)
