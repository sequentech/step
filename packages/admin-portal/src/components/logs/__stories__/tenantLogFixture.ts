// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic entries of a tenant log: sign-ins and a lockout, a settings
// change, a change made in Keycloak's console, a deleted election event, an
// export and the log's own entries.
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import type {ILogRow} from "../LogEntryDialog"

/** The tenant's log zone in these stories. */
export const TENANT_LOG_ZONE = "Europe/Madrid"

interface IAccount {
    id: string
    username: string
    name: string
}

export const SECURITY_OFFICER: IAccount = {
    id: storyId(8, 3),
    username: "rosa.marin",
    name: "Rosa Marin",
}
export const AUDITOR: IAccount = {id: storyId(8, 4), username: "ana.vega", name: "Ana Vega"}
export const CONFIGURATION_MANAGER: IAccount = {
    id: storyId(8, 5),
    username: "carl.duarte",
    name: "Carl Duarte",
}
export const NEW_STAFF: IAccount = {id: storyId(8, 6), username: "lena.ortiz", name: "Lena Ortiz"}
const OPERATOR: IAccount = {id: storyId(8, 7), username: "ops.lund", name: "Ops Lund"}

export const SYSTEM_SIGNATURE = "c3lzdGVtLXNpZ25hdHVyZS1vZi10aGUtdGVuYW50LWxvZy1ZYjQ9"
const SENDER_SIGNATURE = "c2VuZGVyLXNpZ25hdHVyZS1vZi10aGUtYWRtaW4ta2V5LVc1Yz0="
export const ADMIN_KEY = "MCowBQYDK2VwAyEA9sLwQk3mVh1xT7uY2bN8cR5dF6gH0jK4W5c="
export const STATE_HASH = `9f3c52e8${"4b".repeat(24)}d17e1a0b`
export const DELETED_EVENT_DATABASE = `tenant${TENANT_ID.replaceAll("-", "")}event${EVENT_ID.replaceAll("-", "")}`
export const EXPORT_SHA256 = `5d1e${"a7".repeat(28)}c09b`

/** 1 February 2028, 00:00 UTC. */
const START = Date.UTC(2028, 1, 1) / 1000
const at = (day: number, time: string) => {
    const [hours, minutes, seconds] = time.split(":").map(Number)
    return START + (day - 1) * 86400 + hours * 3600 + minutes * 60 + (seconds ?? 0)
}

export type TenantLogRow = ILogRow & {
    id: number
    created: number
    statement_timestamp: number
    statement_kind: string
    message: string
    user_id: string
    // The story's data provider filters on the row's own columns.
    username: string | null
    event_type: string
    log_type: string
}

interface IEntry {
    id: number
    kind: string
    timestamp: number
    account: IAccount | null
    description: string
    eventType?: "USER" | "SYSTEM"
    logType?: "INFO" | "ERROR"
    body: Record<string, unknown>
}

const entry = ({
    id,
    kind,
    timestamp,
    account,
    description,
    eventType = "USER",
    logType = "INFO",
    body,
}: IEntry): TenantLogRow => ({
    id,
    created: timestamp + 1,
    statement_timestamp: timestamp,
    statement_kind: kind,
    user_id: account?.id ?? "",
    username: account?.username ?? null,
    event_type: eventType,
    log_type: logType,
    message: JSON.stringify({
        sender: {name: account && eventType === "USER" ? account.name : "", pk: ADMIN_KEY},
        sender_signature: SENDER_SIGNATURE,
        system_signature: SYSTEM_SIGNATURE,
        user_id: account?.id ?? null,
        username: account?.username ?? null,
        election_id: null,
        area_id: null,
        statement: {
            head: {
                event: TENANT_ID,
                kind,
                timestamp,
                event_type: eventType,
                log_type: logType,
                description,
            },
            body,
        },
    }),
})

const audit = (
    options: Omit<IEntry, "body"> & {details: Record<string, unknown>}
): TenantLogRow => {
    const {details, ...rest} = options
    return entry({
        ...rest,
        body: {
            Audit: {
                kind: options.kind,
                event_type: options.eventType ?? "USER",
                log_type: options.logType ?? "INFO",
                description: options.description,
                details_json: JSON.stringify(details),
                entry_id: storyId(9, options.id),
            },
        },
    })
}

const signIn = (
    id: number,
    timestamp: number,
    account: IAccount | null,
    description: string,
    event: string,
    details: Record<string, unknown> = {},
    logType: "INFO" | "ERROR" = "INFO"
): TenantLogRow =>
    entry({
        id,
        kind: "KeycloakUserEvent",
        timestamp,
        account,
        description,
        logType,
        body: {
            KeycloakUserEvent: [
                JSON.stringify({
                    error: null,
                    client_id: "admin-portal",
                    ip_address: "203.0.113.77",
                    auth_method: "openid-connect",
                    auth_type: "code",
                    credential_type: null,
                    ...details,
                }),
                event,
            ],
        },
    })

const actor = (account: IAccount) => ({
    user_id: account.id,
    username: account.username,
    role: "admin-user",
})

export const EXPORT_ENTRY = audit({
    id: 5262,
    kind: "LogsExported",
    timestamp: at(3, "10:02:41"),
    account: AUDITOR,
    description: `Exported (CSV): ${SECURITY_OFFICER.username} with her election event entries, 14 entries`,
    details: {
        format: "CSV",
        scope: "ACCOUNT_WITH_EVENTS",
        username: SECURITY_OFFICER.username,
        election_id: null,
        area_id: null,
        created_from: "2028-01-31T23:00:00Z",
        created_to: "2028-02-03T22:59:00Z",
        time_zone: TENANT_LOG_ZONE,
        entries: 14,
        sha256: EXPORT_SHA256,
        document_id: storyId(5, 2),
    },
})

export const SIGNED_IN_ENTRY = signIn(
    5261,
    at(3, "09:58:03"),
    AUDITOR,
    "Signed in: password and authenticator code",
    "LOGIN",
    {credential_type: "otp"}
)

export const KEYCLOAK_CONSOLE_ENTRY = audit({
    id: 5260,
    kind: "KeycloakAdminEvent",
    timestamp: at(3, "09:44:19"),
    account: OPERATOR,
    description: `Changed in Keycloak's admin console: ${CONFIGURATION_MANAGER.username} added to the role Security Officer`,
    details: {
        operation: "CREATE",
        resource_type: "GROUP_MEMBERSHIP",
        resource_path: `users/${CONFIGURATION_MANAGER.id}/groups/${storyId(6, 4)}`,
        representation: {name: "Security Officer", path: "/Security Officer"},
        error: null,
        actor: {
            user_id: OPERATOR.id,
            username: OPERATOR.username,
            client_id: "security-admin-console",
            realm: "master",
            ip_address: "203.0.113.41",
        },
        made_by: "OPERATOR",
    },
})

export const SETTINGS_ENTRY = audit({
    id: 5259,
    kind: "AdminChange",
    timestamp: at(3, "09:21:37"),
    account: SECURITY_OFFICER,
    description: "Settings changed: Languages, Integrations and Voting Channels",
    details: {
        object: {type: "tenant", id: TENANT_ID, name: "Northwind Staff Association"},
        action: "update",
        actor: actor(SECURITY_OFFICER),
        origin: "hasura",
        changes: [
            {
                path: "settings.language_conf",
                old: {enabled_language_codes: ["en"], default_language_code: "en"},
                new: {enabled_language_codes: ["en", "es"], default_language_code: "en"},
            },
            {path: "settings.gapi_key", secret: true},
            {path: "voting_channels.kiosk", old: false, new: true},
            {path: "settings.logs_timezone", old: "UTC", new: TENANT_LOG_ZONE},
        ],
    },
})

export const WRITER_STARTED_ENTRY = audit({
    id: 5255,
    kind: "LogWriterStarted",
    timestamp: at(3, "01:00:06"),
    account: null,
    eventType: "SYSTEM",
    description: "Log writer started (release 10.1.0); 3 waiting entries posted",
    details: {release: "10.1.0", last_entry_id: 5254},
})

const WRITER_STOPPED_ENTRY = audit({
    id: 5254,
    kind: "LogWriterStopped",
    timestamp: at(3, "00:58:40"),
    account: null,
    eventType: "SYSTEM",
    description: "Log writer stopped for an upgrade; last entry 5253",
    details: {release: "10.0.4", last_entry_id: 5253},
})

export const DELETED_EVENT_ENTRY = audit({
    id: 5253,
    kind: "AdminChange",
    timestamp: at(2, "16:55:10"),
    account: CONFIGURATION_MANAGER,
    description: "Election event deleted: Training 2027. Its log ended at entry 18,442",
    details: {
        object: {type: "election_event", id: EVENT_ID, name: "Training 2027"},
        action: "delete",
        actor: actor(CONFIGURATION_MANAGER),
        origin: "windmill",
        changes: [],
        extra: {
            log_database: DELETED_EVENT_DATABASE,
            last_entry_id: 18442,
            state_hash: STATE_HASH,
            entries: 18442,
            status: "RETIRED",
            task_id: storyId(7, 3),
        },
    },
})

const SERVICE_ACCOUNT_ENTRY = audit({
    id: 5221,
    kind: "KeycloakAdminEvent",
    timestamp: at(1, "14:47:20"),
    account: null,
    eventType: "SYSTEM",
    description: `Keycloak: group Staff given to ${NEW_STAFF.username} by Step's service account`,
    details: {
        operation: "CREATE",
        resource_type: "GROUP_MEMBERSHIP",
        resource_path: `users/${NEW_STAFF.id}/groups/${storyId(6, 5)}`,
        representation: {name: "Staff", path: "/Staff"},
        error: null,
        actor: {
            user_id: storyId(8, 9),
            username: "service-account-service-account",
            client_id: "service-account",
            realm: `tenant-${TENANT_ID}`,
            ip_address: "10.0.0.7",
        },
        made_by: "SERVICE_ACCOUNT",
    },
})

const STAFF_ROLE_ENTRY = audit({
    id: 5222,
    kind: "AdminChange",
    timestamp: at(1, "14:47:20"),
    account: SECURITY_OFFICER,
    description: `Staff account ${NEW_STAFF.username}: role Staff given`,
    details: {
        object: {type: "staff_account", id: NEW_STAFF.id, name: NEW_STAFF.username},
        action: "grant",
        actor: actor(SECURITY_OFFICER),
        origin: "harvest",
        changes: [{path: "groups", old: [], new: ["Staff"]}],
    },
})

const TEMPLATE_ENTRY = audit({
    id: 5220,
    kind: "AdminChange",
    timestamp: at(1, "11:31:18"),
    account: CONFIGURATION_MANAGER,
    description: "Template changed: Enrollment approved (email)",
    details: {
        object: {type: "template", id: storyId(2, 6), name: "Enrollment approved"},
        action: "update",
        actor: actor(CONFIGURATION_MANAGER),
        origin: "hasura",
        changes: [
            {
                path: "template.email.html_body",
                old_sha256: `1c7e${"3d".repeat(28)}aa01`,
                new_sha256: `84b2${"f0".repeat(28)}6e9d`,
            },
            {path: "template.email.subject", old: "Approved", new: "Your enrollment is approved"},
        ],
    },
})

export const UNKNOWN_ACCOUNT_ENTRY = signIn(
    5238,
    at(2, "05:44:15"),
    null,
    "Sign-in failed: unknown account",
    "LOGIN_ERROR",
    {error: "user_not_found", unknown_account: true},
    "ERROR"
)

export const LOCKOUT_ENTRY = signIn(
    5236,
    at(2, "05:43:57"),
    NEW_STAFF,
    "Account locked for 1 minute after 30 failed sign-ins in a row",
    "USER_DISABLED_BY_TEMPORARY_LOCKOUT",
    {error: "user_temporarily_disabled", details: {num_failures: "30"}},
    "ERROR"
)

const failedSignIn = (id: number, timestamp: number, attempt: string): TenantLogRow =>
    signIn(
        id,
        timestamp,
        NEW_STAFF,
        `Sign-in failed: wrong password (${attempt} in a row)`,
        "LOGIN_ERROR",
        {error: "invalid_user_credentials", credential_type: "password"},
        "ERROR"
    )

const PUBLIC_KEY_ENTRY = entry({
    id: 5200,
    kind: "AdminPublicKey",
    timestamp: at(1, "08:12:04"),
    account: SECURITY_OFFICER,
    description: `Admin public key of ${SECURITY_OFFICER.username}`,
    body: {AdminPublicKey: [TENANT_ID, SECURITY_OFFICER.id, ADMIN_KEY]},
})

/** The tenant log, newest first, as listElectoralLog answers it. */
export const tenantLogRows = (): TenantLogRow[] => [
    EXPORT_ENTRY,
    SIGNED_IN_ENTRY,
    KEYCLOAK_CONSOLE_ENTRY,
    SETTINGS_ENTRY,
    WRITER_STARTED_ENTRY,
    WRITER_STOPPED_ENTRY,
    DELETED_EVENT_ENTRY,
    UNKNOWN_ACCOUNT_ENTRY,
    LOCKOUT_ENTRY,
    failedSignIn(5235, at(2, "05:43:52"), "30th"),
    failedSignIn(5234, at(2, "05:43:47"), "29th"),
    STAFF_ROLE_ENTRY,
    SERVICE_ACCOUNT_ENTRY,
    TEMPLATE_ENTRY,
    PUBLIC_KEY_ENTRY,
]

/** The entries a page of the list shows. */
export const TENANT_LOG_PAGE = 10
