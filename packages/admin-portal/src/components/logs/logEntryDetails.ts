// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the entry popup shows of a log row: the row's message JSON (the board
 * message as windmill serializes it) read into one typed view, whatever the
 * statement kind. Nothing here is translated; the popup labels it.
 */

export enum ELogOrigin {
    HASURA = "hasura",
    HARVEST = "harvest",
    WINDMILL = "windmill",
    KEYCLOAK_CONSOLE = "keycloak-console",
    SERVICE_ACCOUNT = "service-account",
}

export enum ELogChangeKind {
    VALUES = "values",
    /** A secret field: the entry only says that it changed. */
    SECRET = "secret",
    /** A text too long to keep: its SHA-256 before and after. */
    HASHES = "hashes",
}

/** A logged value as text, a yes/no, or null when there is none. */
export type LogValue = string | boolean | null

export type ILogChange =
    | {kind: ELogChangeKind.VALUES; path: string; old: LogValue; new: LogValue}
    | {kind: ELogChangeKind.SECRET; path: string}
    | {
          kind: ELogChangeKind.HASHES
          path: string
          oldSha256: string | null
          newSha256: string | null
      }

export interface ILogWho {
    username: string | null
    userId: string | null
    role: string | null
    /** A sign-in attempt at an account that doesn't exist: the name typed isn't recorded. */
    unknownAccount: boolean
}

export interface ILogObject {
    type: string
    name: string | null
    id: string | null
}

export interface ILogSignIn {
    /** Keycloak's event type, such as LOGIN_ERROR. */
    event: string | null
    error: string | null
    clientId: string | null
    ipAddress: string | null
    /** The method and type Keycloak names, such as "openid-connect · code". */
    authentication: string | null
    credential: string | null
    failures: string | null
    /** The rest of Keycloak's details, as it names them. */
    details: Array<{name: string; value: string}>
    /** The entry's text when it isn't the structured JSON (older entries). */
    text: string | null
}

export interface ILogKeycloakChange {
    origin: ELogOrigin.KEYCLOAK_CONSOLE | ELogOrigin.SERVICE_ACCOUNT
    username: string | null
    userId: string | null
    realm: string | null
    clientId: string | null
    ipAddress: string | null
    operation: string | null
    resourceType: string | null
    resourcePath: string | null
    appliesTo: string | null
    /** The new representation, as indented JSON. */
    representation: string | null
    error: string | null
}

export interface ILogDeletedEvent {
    lastEntryId: string | null
    stateHash: string | null
    logDatabase: string
    entries: string | null
    status: string | null
    taskId: string | null
    /** The retired log's system key: the event's keys are deleted with it. */
    systemPublicKey: string | null
}

export interface ILogExport {
    format: string | null
    scope: string | null
    username: string | null
    electionId: string | null
    areaId: string | null
    createdFrom: string | null
    createdTo: string | null
    timeZone: string | null
    entries: string | null
    sha256: string | null
    documentId: string | null
}

/** How the checksum an admin declared for an imported file compares with the file's. */
export enum EImportChecksum {
    UNDECLARED = "undeclared",
    MATCHES = "matches",
    MISMATCH = "mismatch",
}

/** The file an import read, as the task that read it hashed and counted it. */
export interface ILogImportedFile {
    documentId: string | null
    fileSha256: string
    declaredSha256: string | null
    /** Null when the entry doesn't say, as older imports don't. */
    declaredChecksum: EImportChecksum | null
    /** What the file held, by name: elections, candidates, rows (`count`)… */
    counts: Array<{name: string; value: string}>
}

export interface ILogWriter {
    release: string | null
    lastEntryId: string | null
}

export interface ILogSignatures {
    /** The admin whose key signed the entry; null when only the log's system key did. */
    signedBy: string | null
    senderKey: string | null
    senderSignature: string | null
    systemSignature: string | null
}

export interface ILogEntryView {
    /** The description; the raw message when it can't be read. */
    description: string
    /** The message isn't the JSON the popup knows: only `description` is shown. */
    unreadable: boolean
    statementKind: string | null
    eventType: string | null
    logType: string | null
    who: ILogWho
    origin: ELogOrigin | null
    object: ILogObject | null
    action: string | null
    changes: ILogChange[]
    signIn: ILogSignIn | null
    keycloak: ILogKeycloakChange | null
    deletedEvent: ILogDeletedEvent | null
    importedFile: ILogImportedFile | null
    exported: ILogExport | null
    writer: ILogWriter | null
    /** Free details of a change that the popup has no section for (import counts, hashes). */
    extra: Array<{name: string; value: string}>
    /** The admin key an AdminPublicKey entry publishes. */
    publicKey: string | null
    signatures: ILogSignatures
}

export const STATEMENT_KIND_KEYCLOAK_USER_EVENT = "KeycloakUserEvent"
export const STATEMENT_KIND_ADMIN_CHANGE = "AdminChange"
export const STATEMENT_KIND_KEYCLOAK_ADMIN_EVENT = "KeycloakAdminEvent"
export const STATEMENT_KIND_LOGS_EXPORTED = "LogsExported"
export const STATEMENT_KIND_LOG_WRITER_STARTED = "LogWriterStarted"
export const STATEMENT_KIND_LOG_WRITER_STOPPED = "LogWriterStopped"
export const STATEMENT_KIND_ADMIN_PUBLIC_KEY = "AdminPublicKey"
export const STATEMENT_KIND_TENANT_ADMIN_ACTION = "TenantAdminAction"

const MADE_BY_SERVICE_ACCOUNT = "SERVICE_ACCOUNT"
const FAILURE_DETAILS = ["num_failures", "numFailures"]

type Json = Record<string, unknown>

const isObject = (value: unknown): value is Json =>
    !!value && typeof value === "object" && !Array.isArray(value)

const parseObject = (text: unknown): Json | null => {
    if (typeof text !== "string") return null
    try {
        const value = JSON.parse(text)
        return isObject(value) ? value : null
    } catch {
        return null
    }
}

/** A scalar as text; null for nothing, "null" (as the board writes a missing user) and objects. */
const text = (value: unknown): string | null => {
    if (typeof value === "string") return value && value !== "null" ? value : null
    if (typeof value === "number" || typeof value === "boolean") return String(value)
    return null
}

const compact = (value: unknown): string =>
    typeof value === "string" ? value : JSON.stringify(value)

const listText = (values: unknown[]): string =>
    values
        .map((value) => (isObject(value) || Array.isArray(value) ? compact(value) : String(value)))
        .join(", ")

/**
 * A logged value as the popup shows it: text as written, a yes/no as a
 * boolean, a list joined by commas and an object as `key: value; key: value`.
 * Null stands for "no value" (absent, null or empty).
 */
export const logValue = (value: unknown): LogValue => {
    if (value === undefined || value === null || value === "") return null
    if (typeof value === "boolean") return value
    if (typeof value === "string") return value
    if (typeof value === "number") return String(value)
    if (Array.isArray(value)) return value.length ? listText(value) : null
    if (isObject(value)) {
        const parts = Object.entries(value).map(([key, inner]) => {
            const shown = logValue(inner)
            return `${key}: ${shown === null ? "-" : String(shown)}`
        })
        return parts.length ? parts.join("; ") : null
    }
    return String(value)
}

const changeOf = (value: unknown): ILogChange | null => {
    if (!isObject(value) || typeof value.path !== "string") return null
    const path = value.path
    if (value.secret === true) return {kind: ELogChangeKind.SECRET, path}
    if ("old_sha256" in value || "new_sha256" in value) {
        return {
            kind: ELogChangeKind.HASHES,
            path,
            oldSha256: text(value.old_sha256),
            newSha256: text(value.new_sha256),
        }
    }
    return {kind: ELogChangeKind.VALUES, path, old: logValue(value.old), new: logValue(value.new)}
}

const CHANGE_ORIGINS = [ELogOrigin.HASURA, ELogOrigin.HARVEST, ELogOrigin.WINDMILL]

const changeOrigin = (value: unknown): ELogOrigin | null =>
    CHANGE_ORIGINS.find((origin) => origin === value) ?? null

const rows = (value: Json, skip: string[] = []): Array<{name: string; value: string}> =>
    Object.entries(value)
        .filter(([name]) => !skip.includes(name))
        .map(([name, inner]) => ({name, value: logValue(inner)}))
        .filter((row): row is {name: string; value: string | boolean} => row.value !== null)
        .map(({name, value: shown}) => ({name, value: String(shown)}))

const DELETED_EVENT_KEYS = [
    "log_database",
    "last_entry_id",
    "state_hash",
    "entries",
    "status",
    "task_id",
    "system_public_key",
]

const IMPORTED_FILE_KEYS = [
    "document_id",
    "file_sha256",
    "declared_sha256",
    "declared_checksum",
    "counts",
    "count",
]

const importChecksum = (value: unknown): EImportChecksum | null =>
    (Object.values(EImportChecksum) as unknown[]).includes(value)
        ? (value as EImportChecksum)
        : null

const importedFileOf = (extra: Json | null): ILogImportedFile | null => {
    const fileSha256 = text(extra?.file_sha256)
    if (!extra || !fileSha256) return null
    const counts = isObject(extra.counts) ? rows(extra.counts) : []
    const count = text(extra.count)
    return {
        documentId: text(extra.document_id),
        fileSha256,
        declaredSha256: text(extra.declared_sha256),
        declaredChecksum: importChecksum(extra.declared_checksum),
        counts: count === null ? counts : [{name: "count", value: count}, ...counts],
    }
}

const deletedEventOf = (extra: Json | null): ILogDeletedEvent | null => {
    const logDatabase = text(extra?.log_database)
    if (!extra || !logDatabase) return null
    return {
        logDatabase,
        lastEntryId: text(extra.last_entry_id),
        stateHash: text(extra.state_hash),
        entries: text(extra.entries),
        status: text(extra.status),
        taskId: text(extra.task_id),
        systemPublicKey: text(extra.system_public_key),
    }
}

const signInOf = (body: unknown): ILogSignIn | null => {
    if (!isObject(body)) return null
    const variant = body[STATEMENT_KIND_KEYCLOAK_USER_EVENT]
    if (!Array.isArray(variant)) return null
    const [content, event] = variant
    const details = parseObject(content)
    if (!details) {
        return {
            event: text(event),
            error: null,
            clientId: null,
            ipAddress: null,
            authentication: null,
            credential: null,
            failures: null,
            details: [],
            text: text(content),
        }
    }
    const rest = isObject(details.details) ? details.details : {}
    const failureKey = FAILURE_DETAILS.find((key) => text(rest[key]) !== null)
    return {
        event: text(event),
        error: text(details.error),
        clientId: text(details.client_id),
        ipAddress: text(details.ip_address),
        authentication:
            [text(details.auth_method), text(details.auth_type)]
                .filter((part): part is string => part !== null)
                .join(" · ") || null,
        credential: text(details.credential_type),
        failures: failureKey ? text(rest[failureKey]) : null,
        details: rows(rest, FAILURE_DETAILS),
        text: null,
    }
}

const unknownAccount = (body: unknown): boolean => {
    if (!isObject(body)) return false
    const variant = body[STATEMENT_KIND_KEYCLOAK_USER_EVENT]
    return Array.isArray(variant) && parseObject(variant[0])?.unknown_account === true
}

/** What a Keycloak admin event applies to: the named thing, else the resource it addresses. */
const appliesTo = (representation: unknown, path: string | null): string | null => {
    if (isObject(representation)) {
        const named = text(representation.username) ?? text(representation.name)
        if (named) return named
    }
    return path ? path.split("/").slice(0, 2).join("/") : null
}

const keycloakOf = (details: Json): ILogKeycloakChange => {
    const actor = isObject(details.actor) ? details.actor : {}
    const resourcePath = text(details.resource_path)
    const representation = details.representation
    return {
        origin:
            details.made_by === MADE_BY_SERVICE_ACCOUNT
                ? ELogOrigin.SERVICE_ACCOUNT
                : ELogOrigin.KEYCLOAK_CONSOLE,
        username: text(actor.username),
        userId: text(actor.user_id),
        realm: text(actor.realm),
        clientId: text(actor.client_id),
        ipAddress: text(actor.ip_address),
        operation: text(details.operation),
        resourceType: text(details.resource_type),
        resourcePath,
        appliesTo: appliesTo(representation, resourcePath),
        representation:
            representation === null || representation === undefined
                ? null
                : typeof representation === "string"
                  ? representation
                  : JSON.stringify(representation, null, 2),
        error: text(details.error),
    }
}

const exportOf = (details: Json): ILogExport => ({
    format: text(details.format),
    scope: text(details.scope),
    username: text(details.username),
    electionId: text(details.election_id),
    areaId: text(details.area_id),
    createdFrom: text(details.created_from),
    createdTo: text(details.created_to),
    timeZone: text(details.time_zone),
    entries: text(details.entries),
    sha256: text(details.sha256),
    documentId: text(details.document_id),
})

/** The details an Audit body carries, whatever its kind. */
const auditDetails = (body: unknown): Json | null => {
    if (!isObject(body) || !isObject(body.Audit)) return null
    return parseObject(body.Audit.details_json)
}

/**
 * The details of a TenantActivity body: a request Harvest recorded outside an
 * election event, or the opening of a tenant log.
 */
const tenantActivityDetails = (body: unknown): Json | null => {
    if (!isObject(body) || !isObject(body.TenantActivity)) return null
    return parseObject(body.TenantActivity.details_json)
}

const keyText = (value: unknown): string | null =>
    value === null || value === undefined || value === "" ? null : compact(value)

const emptyView = (description: string, statementKind: string | null): ILogEntryView => ({
    description,
    unreadable: true,
    statementKind,
    eventType: null,
    logType: null,
    who: {username: null, userId: null, role: null, unknownAccount: false},
    origin: null,
    object: null,
    action: null,
    changes: [],
    signIn: null,
    keycloak: null,
    deletedEvent: null,
    importedFile: null,
    exported: null,
    writer: null,
    extra: [],
    publicKey: null,
    signatures: {signedBy: null, senderKey: null, senderSignature: null, systemSignature: null},
})

/** The popup's view of a log row. A message that isn't JSON keeps only its text. */
export const logEntryView = (
    record:
        | {message?: string | null; statement_kind?: string | null; user_id?: string | null}
        | null
        | undefined
): ILogEntryView => {
    const statementKind = text(record?.statement_kind)
    const message = parseObject(record?.message)
    if (!message) return emptyView(record?.message ?? "", statementKind)

    const statement = isObject(message.statement) ? message.statement : {}
    const head = isObject(statement.head) ? statement.head : {}
    const body = statement.body
    const kind = statementKind ?? text(head.kind)
    const details = auditDetails(body)
    const sender = isObject(message.sender) ? message.sender : {}

    const view: ILogEntryView = {
        ...emptyView(text(head.description) ?? "", kind),
        unreadable: false,
        eventType: text(head.event_type),
        logType: text(head.log_type),
        who: {
            username: text(message.username),
            userId: text(message.user_id) ?? text(record?.user_id),
            role: null,
            unknownAccount: unknownAccount(body),
        },
        signIn: kind === STATEMENT_KIND_KEYCLOAK_USER_EVENT ? signInOf(body) : null,
        signatures: {
            signedBy: text(sender.name),
            senderKey: keyText(sender.pk),
            senderSignature: keyText(message.sender_signature),
            systemSignature: keyText(message.system_signature),
        },
    }

    const activity = tenantActivityDetails(body)
    if (activity && kind === STATEMENT_KIND_TENANT_ADMIN_ACTION) {
        // The action, its outcome and what it named are StatementExplanation's lines.
        view.origin = ELogOrigin.HARVEST
        view.who = {
            ...view.who,
            username: view.who.username ?? text(activity.username),
            userId: view.who.userId ?? text(activity.user_id),
        }
    }

    if (isObject(body) && Array.isArray(body[STATEMENT_KIND_ADMIN_PUBLIC_KEY])) {
        view.publicKey = keyText(body[STATEMENT_KIND_ADMIN_PUBLIC_KEY][2])
    }
    if (!details) return view

    switch (kind) {
        case STATEMENT_KIND_ADMIN_CHANGE: {
            const actor = isObject(details.actor) ? details.actor : {}
            const object = isObject(details.object) ? details.object : null
            const extra = isObject(details.extra) ? details.extra : null
            view.who = {
                ...view.who,
                username: view.who.username ?? text(actor.username),
                userId: view.who.userId ?? text(actor.user_id),
                role: text(actor.role),
            }
            view.origin = changeOrigin(details.origin)
            view.object =
                object && text(object.type)
                    ? {type: String(object.type), name: text(object.name), id: text(object.id)}
                    : null
            view.action = text(details.action)
            view.changes = (Array.isArray(details.changes) ? details.changes : [])
                .map(changeOf)
                .filter((change): change is ILogChange => change !== null)
            view.deletedEvent = deletedEventOf(extra)
            view.importedFile = importedFileOf(extra)
            view.extra = extra
                ? rows(extra, [
                      ...(view.deletedEvent ? DELETED_EVENT_KEYS : []),
                      ...(view.importedFile ? IMPORTED_FILE_KEYS : []),
                  ])
                : []
            break
        }
        case STATEMENT_KIND_KEYCLOAK_ADMIN_EVENT: {
            view.keycloak = keycloakOf(details)
            view.origin = view.keycloak.origin
            view.who = {
                ...view.who,
                username: view.who.username ?? view.keycloak.username,
                userId: view.who.userId ?? view.keycloak.userId,
            }
            break
        }
        case STATEMENT_KIND_LOGS_EXPORTED:
            view.exported = exportOf(details)
            break
        case STATEMENT_KIND_LOG_WRITER_STARTED:
        case STATEMENT_KIND_LOG_WRITER_STOPPED:
            view.writer = {
                release: text(details.release),
                lastEntryId: text(details.last_entry_id),
            }
            break
    }
    return view
}

/** A key or signature short enough for a line: its start and end around an ellipsis. */
export const shortened = (value: string, head = 20, tail = 5): string =>
    value.length <= head + tail + 1 ? value : `${value.slice(0, head)}…${value.slice(-tail)}`
