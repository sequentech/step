// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import {getOperationRole} from "./Permissions"
import {IPermissions} from "@/types/keycloak"
import type {GraphQLRequest} from "@apollo/client"

const operation = (operationName?: string) => ({operationName}) as GraphQLRequest

const roleOf = (name: string | undefined, isTrustee = false, isAdminUser = true): IPermissions =>
    getOperationRole(operation(name), (role) =>
        role === IPermissions.TRUSTEE_CEREMONY
            ? isTrustee
            : role === IPermissions.ADMIN_USER && isAdminUser
    )

it.each([
    "ConfirmKeyCustody",
    "AuthorizeTallyCustody",
    "GetCustodyRequest",
    "ListTrusteeBoards",
    "ReportTrusteeBoard",
])("uses the trustee ceremony permission for the public custody action %s", (name) => {
    expect(roleOf(name)).toBe(IPermissions.TRUSTEE_CEREMONY)
})

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
    // Messaging (R4-20): admin-user can read none of it.
    ["GetEventMessagingConfig", IPermissions.MESSAGING_CONFIG_READ],
    ["GetMessagingAccountSummaries", IPermissions.MESSAGING_CONFIG_READ],
    ["GetVoterUnsettledMessages", IPermissions.MESSAGING_CONFIG_READ],
    ["GetVoterMessageAttempts", IPermissions.MESSAGING_CONFIG_READ],
    ["GetLedgerMessageStates", IPermissions.MESSAGING_CONFIG_READ],
    ["GetMessagingEventStats", IPermissions.MESSAGING_CONFIG_READ],
    ["UpdateEventMessagingConfig", IPermissions.MESSAGING_CONFIG_WRITE],
    ["GetMessagingAccounts", IPermissions.MESSAGING_ACCOUNT_READ],
    ["MessagingAccountCallbackUrl", IPermissions.MESSAGING_ACCOUNT_WRITE],
    ["UpsertMessagingAccount", IPermissions.MESSAGING_ACCOUNT_WRITE],
    // Second pass: the role each Hasura action grants.
    ["GetMessagingAudiencePreview", IPermissions.MESSAGING_CONFIG_READ],
    ["GetMessagingVoterCounts", IPermissions.MESSAGING_CONFIG_READ],
    ["RecountMessagingVoters", IPermissions.MESSAGING_CONFIG_READ],
    ["GetMessagingMonthlyVolumes", IPermissions.MESSAGING_ACCOUNT_READ],
    ["MessagingAccountChangeImpact", IPermissions.MESSAGING_ACCOUNT_WRITE],
    ["MessagingFakeProviderMessages", IPermissions.MESSAGING_ACCOUNT_WRITE],
    ["MessagingFakeProviderMessage", IPermissions.MESSAGING_ACCOUNT_WRITE],
    ["MessagingFakeProviderGetStarted", IPermissions.MESSAGING_ACCOUNT_WRITE],
    ["FindApplicationsBySupportReference", IPermissions.APPLICATION_READ],
])("requires the role for %s in both modes", (name, role) => {
    expect(roleOf(name)).toBe(role)
    expect(roleOf(name, true)).toBe(role)
})

it.each(["sequent_backend_keys_ceremony", "sequent_backend_tally_session_execution"])(
    "uses the trustee ceremony permission only in trustee mode: %s",
    (name) => {
        expect(roleOf(name)).toBe(IPermissions.ADMIN_CEREMONY)
        expect(roleOf(name, true)).toBe(IPermissions.TRUSTEE_CEREMONY)
    }
)
it("asks whether a trustee's key step needs a signature with the trustee ceremony role", () => {
    expect(roleOf("CustodyProofSignatureStatus", true)).toBe(IPermissions.TRUSTEE_CEREMONY)
    expect(roleOf("CustodyProofSignatureStatus")).toBe(IPermissions.ADMIN_USER)
})
it("keeps the trustee user lookup separate from the admin fallback", () => {
    expect(roleOf("getUsers", true)).toBe(IPermissions.VOTER_READ)
    expect(roleOf("getUsers")).toBe(IPermissions.ADMIN_USER)
})
it.each([undefined, "", "unknown", "toString", "constructor", "__proto__"])(
    "falls back to an explicit admin permission for unknown operation %s",
    (name) => {
        // Prototype property names are unknown operations too, not permission values.
        expect(roleOf(name)).toBe(IPermissions.ADMIN_USER)
        expect(roleOf(name, true)).toBe(IPermissions.ADMIN_USER)
    }
)

describe("staff without the admin-user role", () => {
    const nonAdmin = (name?: string) => roleOf(name, false, false)

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
        expect(roleOf("getRoles")).toBe(IPermissions.ADMIN_USER)
        expect(roleOf("election_events_tree")).toBe(IPermissions.ADMIN_USER)
        expect(roleOf("UpdateElectionVotingStatus")).toBe(IPermissions.ADMIN_USER)
    })
})

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
    "GetCustodyRequest",
    "ConfirmKeyCustody",
    "AuthorizeTallyCustody",
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
    getOperationRole(operation(operationName), holding(roles))

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

// The board holds publication read access as well as account/role read access.
// Its trustees also hold the shipped ceremony roles. Neither is an admin-user.
const electionBoardRoles: Array<string> = [
    "admin-ceremony",
    "area-read",
    "create-ceremony",
    "document-read",
    "document-download",
    "election-dashboard-tab",
    "election-data-tab",
    "election-event-archive",
    "election-event-create",
    "election-event-data-tab",
    "election-event-delete",
    "election-event-keys-tab",
    "election-event-logs-tab",
    "election-event-publish-tab",
    "election-event-read",
    "election-event-scheduled-event-columns",
    "election-event-scheduled-tab",
    "election-event-signatures-tab",
    "election-event-tally-tab",
    "election-event-tasks-tab",
    "election-event-voters-tab",
    "election-publish-tab",
    "election-state-write",
    "electoral-log-audit",
    "electoral-log-console-query",
    "electoral-log-console-read",
    "export-ceremony",
    "keys-read",
    "logs-export",
    "logs-read",
    "monitoring-view",
    "publish-changes",
    "publish-create",
    "publish-pause-voting",
    "publish-read",
    "publish-regenerate",
    "publish-start-voting",
    "publish-stop-voting",
    "publish-write",
    "report-write",
    "report-read",
    "role-read",
    "scheduled-event-create",
    "scheduled-event-delete",
    "scheduled-event-write",
    "sign-approve-configuration",
    "sign-approve-voter",
    "sign-close-voting",
    "sign-generate-election-returns",
    "sign-generate-reports",
    "sign-initialize-voting",
    "sign-open-voting",
    "sign-transmit-results",
    "signing-certificates-read",
    "signing-requests-cancel",
    "signing-requests-export",
    "signing-requests-read",
    "signing-rules-read",
    "tally-read",
    "tally-results-read",
    "tally-start",
    "tally-write",
    "tasks-read",
    "user-permission-read",
    "user-read",
    "users-menu",
    "voter-export",
    "voter-read",
]
const boardTrusteeRoles = [
    ...electionBoardRoles,
    "trustee-ceremony",
    "trustee-read",
    "election-read",
    "document-write",
    "document-read",
    "document-upload",
    "document-download",
    "sign-key-ceremony",
    "sign-tally-key",
]

describe.each([
    ["election board", electionBoardRoles],
    ["board trustee", boardTrusteeRoles],
])("fine-grained account reads for %s", (_name, roles) => {
    it.each([
        ["getUsers", IPermissions.USER_READ],
        ["GetUserProfileConfiguration", IPermissions.USER_READ],
        ["getRoles", IPermissions.ROLE_READ],
        ["getPermissions", IPermissions.USER_PERMISSION_READ],
    ])(
        "uses the held permission for %s instead of the publication/ceremony default",
        (name, expected) => {
            expect(roles).not.toContain(IPermissions.ADMIN_USER)
            expect(roles).toContain(expected)
            expect(roleFor(name, roles as Array<string>)).toBe(expected)
        }
    )

    it.each([
        ["getUsers", {election_event_id: "event-a"}],
        ["GetUserProfileConfiguration", {electionEventId: "event-a"}],
    ])("keeps election-event %s on voter-read", (name, variables) => {
        expect(
            getOperationRole({...operation(name), variables}, holding(roles as Array<string>))
        ).toBe(IPermissions.VOTER_READ)
    })
})

it("does not borrow an account-read permission for a publisher who lacks it", () => {
    for (const name of ["getUsers", "GetUserProfileConfiguration", "getRoles", "getPermissions"]) {
        expect(roleFor(name, [IPermissions.PUBLISH_READ])).toBe(IPermissions.PUBLISH_READ)
    }
})

it("keeps the legacy administrator profile-read fallback", () => {
    expect(roleOf("GetUserProfileConfiguration")).toBe(IPermissions.ADMIN_USER)
})

describe("scoped configuration archive operations", () => {
    it.each([
        ["election board", electionBoardRoles],
        ["board trustee", boardTrusteeRoles],
    ])(
        "imports signed election packages using the held create permission for %s",
        (_name, roles) => {
            expect(roleFor("ImportElectionEvent", roles as Array<string>)).toBe(
                IPermissions.ELECTION_EVENT_CREATE
            )
        }
    )

    it("uploads a board's private configuration archive without borrowing document-upload", () => {
        expect(electionBoardRoles).not.toContain(IPermissions.DOCUMENT_UPLOAD)
        expect(roleFor("GetUploadUrl", electionBoardRoles)).toBe(IPermissions.ELECTION_EVENT_CREATE)
    })

    it("uploads a tenant restore archive using the administrator's held tenant-write", () => {
        const roles = [IPermissions.ADMIN_USER, IPermissions.TENANT_READ, IPermissions.TENANT_WRITE]
        expect(roleFor("GetUploadUrl", roles)).toBe(IPermissions.TENANT_WRITE)
    })

    it("preserves document-upload for a caller holding the generic permission", () => {
        expect(roleFor("GetUploadUrl", boardTrusteeRoles)).toBe(IPermissions.DOCUMENT_UPLOAD)
        expect(
            roleFor("GetUploadUrl", [IPermissions.ADMIN_USER, IPermissions.DOCUMENT_UPLOAD])
        ).toBe(IPermissions.DOCUMENT_UPLOAD)
    })

    it("does not select archive permissions that the caller does not hold", () => {
        const roles = [IPermissions.PUBLISH_READ]
        expect(roleFor("GetUploadUrl", roles)).toBe(IPermissions.PUBLISH_READ)
        expect(roleFor("ImportElectionEvent", roles)).toBe(IPermissions.PUBLISH_READ)
    })
})

describe("board document and task reads", () => {
    it.each([
        ["FetchDocument", IPermissions.DOCUMENT_DOWNLOAD],
        ["GetDocument", IPermissions.DOCUMENT_READ],
        ["GetTaskById", IPermissions.TASKS_READ],
    ])("uses only the held permission for %s", (name, permission) => {
        for (const roles of [electionBoardRoles, boardTrusteeRoles]) {
            expect(roleFor(name, roles)).toBe(permission)
        }
        expect(roleFor(name, [IPermissions.PUBLISH_READ])).toBe(IPermissions.PUBLISH_READ)
        expect(roleFor(name, [IPermissions.TRUSTEE_CEREMONY])).toBe(IPermissions.TRUSTEE_CEREMONY)
    })

    it("preserves the administrator's scoped tenant-backup download path", () => {
        expect(roleFor("FetchDocument", [IPermissions.ADMIN_USER, IPermissions.TENANT_READ])).toBe(
            IPermissions.ADMIN_USER
        )
        expect(roleFor("GetDocument", [IPermissions.ADMIN_USER])).toBe(IPermissions.ADMIN_USER)
        expect(roleFor("GetTaskById", [IPermissions.ADMIN_USER])).toBe(IPermissions.ADMIN_USER)
    })
})

describe("board publication and ceremony operations", () => {
    it.each([
        ["PublishBallot", IPermissions.PUBLISH_WRITE],
        ["GenerateBallotPublication", IPermissions.PUBLISH_CREATE],
        ["UpdateEventVotingStatus", IPermissions.ELECTION_STATE_WRITE],
        ["UpdateElectionVotingStatus", IPermissions.ELECTION_STATE_WRITE],
        ["CreateTallyCeremony", IPermissions.ADMIN_CEREMONY],
        ["UpdateTallyCeremony", IPermissions.ADMIN_CEREMONY],
        ["GetLifecycleSnapshots", IPermissions.ELECTION_EVENT_READ],
        ["GetTallyData", IPermissions.TALLY_RESULTS_READ],
    ])("uses the existing held role for %s", (name, permission) => {
        for (const roles of [electionBoardRoles, boardTrusteeRoles]) {
            expect(roleFor(name, roles)).toBe(permission)
        }
        expect(roleFor(name, [IPermissions.PUBLISH_READ])).toBe(IPermissions.PUBLISH_READ)
        expect(roleFor(name, [IPermissions.ADMIN_USER])).toBe(IPermissions.ADMIN_USER)
    })

    it("uses an available publication role without inventing another grant", () => {
        expect(roleFor("GenerateBallotPublication", [IPermissions.PUBLISH_REGENERATE])).toBe(
            IPermissions.PUBLISH_REGENERATE
        )
        expect(roleFor("GenerateBallotPublication", [IPermissions.PUBLISH_WRITE])).toBe(
            IPermissions.PUBLISH_WRITE
        )
    })

    it("does not turn ordinary tally access into permission to recount", () => {
        expect(roleFor("RecountTallySession", electionBoardRoles)).toBe(IPermissions.PUBLISH_READ)
        expect(
            roleFor("RecountTallySession", [
                ...electionBoardRoles,
                IPermissions.TALLY_RECOUNT_EXECUTE,
            ])
        ).toBe(IPermissions.TALLY_RECOUNT_EXECUTE)
    })
})

it("keeps trustee execution reads on the ceremony role and board reads on admin-ceremony", () => {
    expect(roleFor("TrusteeNames", electionBoardRoles)).toBe(IPermissions.ADMIN_CEREMONY)
    expect(roleFor("TrusteeNames", boardTrusteeRoles)).toBe(IPermissions.TRUSTEE_CEREMONY)
    expect(roleFor("GetTallySessionExecution", electionBoardRoles)).toBe(
        IPermissions.ADMIN_CEREMONY
    )
    expect(roleFor("GetTallySessionExecution", boardTrusteeRoles)).toBe(
        IPermissions.TRUSTEE_CEREMONY
    )
    expect(roleFor("TrusteeNames", [IPermissions.TRUSTEE_CEREMONY])).toBe(
        IPermissions.TRUSTEE_CEREMONY
    )
    expect(roleFor("TrusteeNames", [IPermissions.TRUSTEE_READ])).toBe(IPermissions.TRUSTEE_READ)
})
