// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {getOperationRole} from "./Permissions"
import {IPermissions} from "@/types/keycloak"
import type {GraphQLRequest} from "@apollo/client"

// Keep the real shared predicate without loading the browser component barrel.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/utils/typechecks"),
}))
const operation = (operationName?: string) => ({operationName}) as GraphQLRequest

it.each([
    ["sequent_backend_area", IPermissions.AREA_READ],
    ["insert_sequent_backend_candidate", IPermissions.CANDIDATE_CREATE],
    ["update_sequent_backend_candidate", IPermissions.CANDIDATE_WRITE],
    ["delete_sequent_backend_candidate", IPermissions.CANDIDATE_DELETE],
    ["sequent_backend_tally_results_publication", IPermissions.PUBLISH_RESULTS_READ],
    ["RevealVoterSecretAttribute", IPermissions.VOTER_READ],
    ["MonitoringGetDashboard", IPermissions.MONITORING_VIEW],
    ["MonitoringRenderWidget", IPermissions.MONITORING_VIEW],
    ["MonitoringExport", IPermissions.MONITORING_VIEW],
    ["MonitoringSaveConfig", IPermissions.MONITORING_CONFIGURE],
    ["GetApprovalMatrix", IPermissions.APPLICATION_READ],
    ["EvaluateApprovalMatrix", IPermissions.APPLICATION_READ],
    ["SaveApprovalMatrix", IPermissions.APPROVAL_MATRIX_WRITE],
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
it("asks whether a trustee's key step needs a signature with the trustee ceremony role", () => {
    expect(getOperationRole(operation("KeyShareSignatureStatus"), true)).toBe(
        IPermissions.TRUSTEE_CEREMONY
    )
    expect(getOperationRole(operation("KeyShareSignatureStatus"))).toBe(IPermissions.ADMIN_USER)
})
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

describe("staff without the admin-user role", () => {
    const nonAdmin = (name?: string) => getOperationRole(operation(name), false, false)

    it.each([
        ["sequent_backend_election_event", IPermissions.ELECTION_EVENT_READ],
        ["sequent_backend_area", IPermissions.AREA_READ],
        ["sequent_backend_tenant", IPermissions.ELECTION_EVENT_READ],
        ["election_events_tree", IPermissions.ELECTION_EVENT_READ],
        ["candidate_tree", IPermissions.ELECTION_EVENT_READ],
        ["getRoles", IPermissions.ROLE_READ],
        ["getPermissions", IPermissions.USER_PERMISSION_READ],
        ["SetRolePermission", IPermissions.ROLE_WRITE],
        ["DeleteRolePermission", IPermissions.ROLE_WRITE],
        ["getUsers", IPermissions.USER_READ],
        // Post > Publish: an SBEI starts Open and Close voting, and Initialize voting.
        ["UpdateElectionVotingStatus", IPermissions.ELECTION_STATE_WRITE],
        ["CreateTallyCeremony", IPermissions.ADMIN_CEREMONY],
    ])("query %s as %s", (name, role) => {
        expect(nonAdmin(name)).toBe(role)
    })

    it.each([undefined, "", "unknown", "IntrospectionQuery", "toString", "__proto__"])(
        "never borrow admin-user, even for %s",
        (name) => {
            expect(nonAdmin(name)).toBe(IPermissions.ELECTION_EVENT_READ)
        }
    )

    it("keep admins on their current roles", () => {
        expect(getOperationRole(operation("getRoles"))).toBe(IPermissions.ADMIN_USER)
        expect(getOperationRole(operation("election_events_tree"))).toBe(IPermissions.ADMIN_USER)
        expect(getOperationRole(operation("UpdateElectionVotingStatus"))).toBe(
            IPermissions.ADMIN_USER
        )
    })
})
