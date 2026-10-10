// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "fs"
import {resolve} from "path"
import {IPermissions} from "@/types/keycloak"
import {getOperationRole} from "./Permissions"

interface RealmGroup {
    name: string
    realmRoles?: Array<string>
}

interface RealmTemplate {
    groups: Array<RealmGroup>
}

const REPO_ROOT = resolve(__dirname, "../../../..")

const REALM_TEMPLATES = [
    ".devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json",
    "packages/windmill/external-bin/janitor/templates/COMELEC/keycloakAdmin.hbs",
]

const TRUSTEE_GROUP = "trustee"
const LOCKDOWN_GROUP = "admin-lockdown"

const SHELL_OPERATIONS = [
    "IntrospectionQuery",
    "sequent_backend_tenant",
    "election_events_tree",
    "election_tree",
    "contest_tree",
    "candidate_tree",
    "sequent_backend_election_event",
    "sequent_backend_election",
    "sequent_backend_contest",
    "sequent_backend_candidate",
]

const TRUSTEE_OPERATIONS = [
    ...SHELL_OPERATIONS,
    "sequent_backend_document",
    "sequent_backend_keys_ceremony",
    "sequent_backend_tally_session",
    "sequent_backend_tally_session_execution",
    "sequent_backend_trustee",
    "TrusteeNames",
    "GetPrivateKey",
    "CheckPrivateKey",
    "RestorePrivateKey",
]

const LOCKDOWN_OPERATIONS = [...SHELL_OPERATIONS, "sequent_backend_ballot_publication"]

const groupRoles = (template: string, groupName: string): Array<string> => {
    const realm: RealmTemplate = JSON.parse(readFileSync(resolve(REPO_ROOT, template), "utf8"))
    const group = realm.groups.find(({name}) => name === groupName)
    if (!group) {
        throw new Error(`group ${groupName} is missing from ${template}`)
    }
    return group.realmRoles ?? []
}

const holding =
    (roles: Array<string>) =>
    (role: IPermissions): boolean =>
        roles.includes(role)

const roleFor = (operationName: string | undefined, roles: Array<string>): IPermissions =>
    getOperationRole({operationName, query: undefined as never}, holding(roles))

describe.each(REALM_TEMPLATES)("realm template %s", (template) => {
    it.each([TRUSTEE_GROUP, LOCKDOWN_GROUP])("does not give admin-user to %s", (groupName) => {
        expect(groupRoles(template, groupName)).not.toContain(IPermissions.ADMIN_USER)
    })

    it.each(TRUSTEE_OPERATIONS)("runs trustee operation %s as a trustee role", (operation) => {
        const roles = groupRoles(template, TRUSTEE_GROUP)
        const role = roleFor(operation, roles)
        expect(role).not.toBe(IPermissions.ADMIN_USER)
        expect(roles).toContain(role)
    })

    it.each(LOCKDOWN_OPERATIONS)("runs lockdown operation %s as a lockdown role", (operation) => {
        const roles = groupRoles(template, LOCKDOWN_GROUP)
        const role = roleFor(operation, roles)
        expect(role).not.toBe(IPermissions.ADMIN_USER)
        expect(roles).toContain(role)
    })
})

describe("getOperationRole", () => {
    it("keeps unnamed and unmapped operations on admin-user for admins", () => {
        const roles = [IPermissions.ADMIN_USER, IPermissions.ELECTION_EVENT_READ]
        expect(roleFor(undefined, roles)).toBe(IPermissions.ADMIN_USER)
        expect(roleFor("sequent_backend_tenant", roles)).toBe(IPermissions.ADMIN_USER)
        expect(roleFor("sequent_backend_unmapped_table", roles)).toBe(IPermissions.ADMIN_USER)
        expect(roleFor("sequent_backend_election_event", roles)).toBe(
            IPermissions.ELECTION_EVENT_READ
        )
    })

    it("keeps the ceremony mapping for admins who also run ceremonies", () => {
        const roles = [IPermissions.ADMIN_USER, IPermissions.TRUSTEE_CEREMONY]
        expect(roleFor("sequent_backend_keys_ceremony", roles)).toBe(IPermissions.TRUSTEE_CEREMONY)
        expect(roleFor("getUsers", roles)).toBe(IPermissions.VOTER_READ)
        expect(roleFor("sequent_backend_tenant", roles)).toBe(IPermissions.ADMIN_USER)
    })

    it("uses the ceremony role for unnamed operations of trustees", () => {
        expect(roleFor(undefined, [IPermissions.TRUSTEE_CEREMONY])).toBe(
            IPermissions.TRUSTEE_CEREMONY
        )
    })

    it("uses the publication read role for unmapped operations of publishers", () => {
        expect(roleFor("sequent_backend_contest", [IPermissions.PUBLISH_READ])).toBe(
            IPermissions.PUBLISH_READ
        )
    })

    it("uses a mapped role only when a trustee holds it", () => {
        const roles = [IPermissions.TRUSTEE_CEREMONY, IPermissions.ELECTION_READ]
        expect(roleFor("sequent_backend_election", roles)).toBe(IPermissions.ELECTION_READ)
        expect(roleFor("sequent_backend_contest", roles)).toBe(IPermissions.TRUSTEE_CEREMONY)
    })
})
