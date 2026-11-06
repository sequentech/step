// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "fs"
import {resolve} from "path"
import {IPermissions} from "@/types/keycloak"
import {getOperationRole, getSchemaRoles} from "./Permissions"

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
    "GetDocument",
    "FetchDocument",
]

const LOCKDOWN_OPERATIONS = [
    ...SHELL_OPERATIONS,
    "sequent_backend_ballot_publication",
    "GenerateBallotPublication",
    "PublishBallot",
    "GetDocument",
    "FetchDocument",
]

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

describe("getSchemaRoles", () => {
    const holding = (roles: Array<IPermissions>) => (role: IPermissions) => roles.includes(role)

    it("uses admin-user alone for admins", () => {
        const roles = [
            IPermissions.ADMIN_USER,
            IPermissions.TRUSTEE_CEREMONY,
            IPermissions.PUBLISH_READ,
        ]
        expect(getSchemaRoles(holding(roles))).toEqual([IPermissions.ADMIN_USER])
    })

    it("uses every profile a restricted user holds", () => {
        const roles = [IPermissions.PUBLISH_READ, IPermissions.TRUSTEE_CEREMONY]
        expect(getSchemaRoles(holding(roles))).toEqual([
            IPermissions.TRUSTEE_CEREMONY,
            IPermissions.PUBLISH_READ,
        ])
    })

    it("adds the document schema to a user who can read documents", () => {
        const roles = [IPermissions.PUBLISH_READ, IPermissions.DOCUMENT_READ]
        expect(getSchemaRoles(holding(roles))).toEqual(roles)
    })

    it("has no role for a user without a profile", () => {
        expect(getSchemaRoles(holding([IPermissions.ELECTION_EVENT_READ]))).toEqual([])
    })
})

describe("operation names that match object members", () => {
    it.each(["toString", "constructor", "__proto__"])("treats %s as unmapped", (operation) => {
        expect(roleFor(operation, [IPermissions.PUBLISH_READ])).toBe(IPermissions.PUBLISH_READ)
    })
})

describe("document reads", () => {
    it("runs them under the document roles for a restricted user", () => {
        const roles = [
            IPermissions.PUBLISH_READ,
            IPermissions.DOCUMENT_READ,
            IPermissions.DOCUMENT_DOWNLOAD,
        ]
        expect(roleFor("GetDocument", roles)).toBe(IPermissions.DOCUMENT_READ)
        expect(roleFor("FetchDocument", roles)).toBe(IPermissions.DOCUMENT_DOWNLOAD)
    })

    it("keeps them on admin-user for admins", () => {
        const roles = [IPermissions.ADMIN_USER, IPermissions.DOCUMENT_DOWNLOAD]
        expect(roleFor("FetchDocument", roles)).toBe(IPermissions.ADMIN_USER)
    })
})

describe("task reads", () => {
    it("prefers tasks-read for a restricted user who holds it", () => {
        const roles = [IPermissions.PUBLISH_READ, IPermissions.TASKS_READ]
        expect(roleFor("GetTaskById", roles)).toBe(IPermissions.TASKS_READ)
    })
})

describe("publication reads", () => {
    const reads = ["GetTaskById", "GetBallotPublicationChange"]

    it.each(reads)("runs %s as publish-read for a user with both profiles", (operation) => {
        const roles = [IPermissions.TRUSTEE_CEREMONY, IPermissions.PUBLISH_READ]
        expect(roleFor(operation, roles)).toBe(IPermissions.PUBLISH_READ)
    })

    it.each(reads)("keeps %s on admin-user for admins", (operation) => {
        const roles = [IPermissions.ADMIN_USER, IPermissions.PUBLISH_READ]
        expect(roleFor(operation, roles)).toBe(IPermissions.ADMIN_USER)
    })

    it.each(reads)("keeps %s on the ceremony role for a trustee", (operation) => {
        expect(roleFor(operation, [IPermissions.TRUSTEE_CEREMONY])).toBe(
            IPermissions.TRUSTEE_CEREMONY
        )
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

    it.each(["GenerateBallotPublication", "PublishBallot"])(
        "runs %s as publish-write for publishers without admin-user",
        (operation) => {
            const roles = [IPermissions.PUBLISH_READ, IPermissions.PUBLISH_WRITE]
            expect(roleFor(operation, roles)).toBe(IPermissions.PUBLISH_WRITE)
        }
    )

    it.each(["GenerateBallotPublication", "PublishBallot"])(
        "keeps %s on admin-user for admins",
        (operation) => {
            const roles = [IPermissions.ADMIN_USER, IPermissions.PUBLISH_WRITE]
            expect(roleFor(operation, roles)).toBe(IPermissions.ADMIN_USER)
        }
    )

    it("uses a mapped role only when a trustee holds it", () => {
        const roles = [IPermissions.TRUSTEE_CEREMONY, IPermissions.ELECTION_READ]
        expect(roleFor("sequent_backend_election", roles)).toBe(IPermissions.ELECTION_READ)
        expect(roleFor("sequent_backend_contest", roles)).toBe(IPermissions.TRUSTEE_CEREMONY)
    })
})
