// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import scanovateContract from "../../../../keycloak-extensions/custom-event-listener/src/test/resources/scanovate-failure-contract.json"

import {
    EImportChecksum,
    ELogChangeKind,
    ELogOrigin,
    logEntryView,
    logValue,
    shortened,
} from "./logEntryDetails"
import {logFieldLabelKey, logObjectTypeKey} from "./logFieldLabels"

const USER_ID = "2e9b6d14-8a3f-4c57-b0e2-9d4f1a7c3e58"
const EVENT_ID = "e2b7c9d4-5f1a-4e83-9b62-7d0c4a8e1f35"

interface IOptions {
    username?: string | null
    userId?: string | null
    sender?: string
    head?: Record<string, unknown>
}

const row = (kind: string, body: unknown, options: IOptions = {}) => ({
    id: 5259,
    statement_kind: kind,
    user_id: options.userId === undefined ? USER_ID : (options.userId ?? ""),
    message: JSON.stringify({
        sender: {name: options.sender ?? "", pk: "MCowBQYDK2VwAyEA9sLw"},
        sender_signature: "c2VuZGVy",
        system_signature: "c3lzdGVt",
        user_id: options.userId === undefined ? USER_ID : options.userId,
        username: options.username === undefined ? "rosa.marin" : options.username,
        election_id: null,
        area_id: null,
        statement: {
            head: {
                event: EVENT_ID,
                kind,
                timestamp: 1833186097,
                event_type: "USER",
                log_type: "INFO",
                description: "Something happened",
                ...options.head,
            },
            body,
        },
    }),
})

const audit = (kind: string, details: unknown, options: IOptions = {}) =>
    row(
        kind,
        {
            Audit: {
                kind,
                event_type: "USER",
                log_type: "INFO",
                description: "Something happened",
                details_json: typeof details === "string" ? details : JSON.stringify(details),
                entry_id: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a00",
            },
        },
        options
    )

describe("the head and who of any entry", () => {
    it.each(["ForensicRead", "PseudonymResolved"])(
        "uses a backfilled display name for historical %s without changing its message",
        (kind) => {
            const original = row(kind, {}, {username: null})
            const record = {...original, username: "election-reviewer"}
            expect(logEntryView(record).who.username).toBe("election-reviewer")
            expect(record.message).toBe(original.message)
            expect(JSON.parse(record.message).username).toBeNull()
        }
    )

    it("prefers the recorded username over a later display-name projection", () => {
        const record = {...row("PseudonymResolved", {}), username: "renamed-reviewer"}
        expect(logEntryView(record).who.username).toBe("rosa.marin")
    })

    it("reads the description, the types and who from the message", () => {
        const view = logEntryView(row("CastVote", {CastVote: []}, {sender: "Rosa Marin"}))
        expect(view).toMatchObject({
            unreadable: false,
            description: "Something happened",
            statementKind: "CastVote",
            eventType: "USER",
            logType: "INFO",
            who: {username: "rosa.marin", userId: USER_ID, role: null, unknownAccount: false},
            origin: null,
            object: null,
            changes: [],
            signIn: null,
            keycloak: null,
            deletedEvent: null,
            importedFile: null,
            exported: null,
            writer: null,
        })
        expect(view.signatures).toEqual({
            signedBy: "Rosa Marin",
            senderKey: "MCowBQYDK2VwAyEA9sLw",
            senderSignature: "c2VuZGVy",
            systemSignature: "c3lzdGVt",
        })
    })

    it("has no signer when only the log's system key signed", () => {
        const view = logEntryView(row("LogWriterStarted", {}, {username: null, userId: null}))
        expect(view.signatures.signedBy).toBeNull()
        expect(view.who).toMatchObject({username: null, userId: null})
    })

    it("reads a board user written as the text null as nobody", () => {
        const view = logEntryView(row("CastVote", {}, {username: "null", userId: "null"}))
        expect(view.who).toMatchObject({username: null, userId: null})
    })

    it("keeps keys and signatures that are not text as compact JSON", () => {
        const record = row("CastVote", {})
        const message = JSON.parse(record.message)
        message.sender.pk = [1, 2, 3]
        message.system_signature = {r: 1}
        const view = logEntryView({...record, message: JSON.stringify(message)})
        expect(view.signatures.senderKey).toBe("[1,2,3]")
        expect(view.signatures.systemSignature).toBe('{"r":1}')
    })

    it.each([
        ["text that is not JSON", "plain text"],
        ["a JSON value that is not an object", "[1,2]"],
        ["broken JSON", '{"statement": '],
    ])("falls back to the raw message for %s", (_, message) => {
        const view = logEntryView({statement_kind: "AdminChange", message})
        expect(view).toMatchObject({
            unreadable: true,
            description: message,
            statementKind: "AdminChange",
            changes: [],
        })
    })

    it("shows nothing for a missing row or message", () => {
        expect(logEntryView(null)).toMatchObject({unreadable: true, description: ""})
        expect(logEntryView({message: null})).toMatchObject({unreadable: true, description: ""})
    })

    it("keeps the head when the details are not readable", () => {
        const view = logEntryView(audit("AdminChange", "{not json"))
        expect(view).toMatchObject({
            unreadable: false,
            description: "Something happened",
            changes: [],
            object: null,
        })
    })

    it("reads the kind from the head when the row has none", () => {
        const record = audit("LogWriterStopped", {release: "10.1.0", last_entry_id: 5253})
        const view = logEntryView({message: record.message})
        expect(view.statementKind).toBe("LogWriterStopped")
        expect(view.writer).toEqual({release: "10.1.0", lastEntryId: "5253"})
    })
})

describe("AdminChange", () => {
    const details = {
        object: {type: "tenant", id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5", name: "Northwind"},
        action: "update",
        actor: {user_id: USER_ID, username: "rosa.marin", role: "admin-user"},
        origin: "hasura",
        changes: [
            {path: "settings.logs_timezone", old: "UTC", new: "Europe/Madrid"},
            {path: "settings.gapi_key", secret: true},
            {path: "template.email.html_body", old_sha256: "aa11", new_sha256: "bb22"},
            {path: "voting_channels.kiosk", old: false, new: true},
            {path: "settings.display_name", new: "Northwind Staff Association"},
            {
                path: "settings.language_conf",
                old: {enabled_language_codes: ["en"], default_language_code: "en"},
                new: {enabled_language_codes: ["en", "es"], default_language_code: "en"},
            },
        ],
    }

    it("reads the object, the action, the actor and where it was made", () => {
        const view = logEntryView(audit("AdminChange", details))
        expect(view.object).toEqual({
            type: "tenant",
            id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            name: "Northwind",
        })
        expect(view.action).toBe("update")
        expect(view.origin).toBe(ELogOrigin.HASURA)
        expect(view.who).toEqual({
            username: "rosa.marin",
            userId: USER_ID,
            role: "admin-user",
            unknownAccount: false,
        })
    })

    it("reads each changed field with its old and new value", () => {
        const view = logEntryView(audit("AdminChange", details))
        expect(view.changes).toEqual([
            {
                kind: ELogChangeKind.VALUES,
                path: "settings.logs_timezone",
                old: "UTC",
                new: "Europe/Madrid",
            },
            {kind: ELogChangeKind.SECRET, path: "settings.gapi_key"},
            {
                kind: ELogChangeKind.HASHES,
                path: "template.email.html_body",
                oldSha256: "aa11",
                newSha256: "bb22",
            },
            {kind: ELogChangeKind.VALUES, path: "voting_channels.kiosk", old: false, new: true},
            {
                kind: ELogChangeKind.VALUES,
                path: "settings.display_name",
                old: null,
                new: "Northwind Staff Association",
            },
            {
                kind: ELogChangeKind.VALUES,
                path: "settings.language_conf",
                old: "enabled_language_codes: en; default_language_code: en",
                new: "enabled_language_codes: en, es; default_language_code: en",
            },
        ])
    })

    it("never shows a value for a secret, even when one is sent", () => {
        const view = logEntryView(
            audit("AdminChange", {
                ...details,
                changes: [{path: "settings.gapi_key", secret: true, old: "a", new: "b"}],
            })
        )
        expect(view.changes).toEqual([{kind: ELogChangeKind.SECRET, path: "settings.gapi_key"}])
        expect(JSON.stringify(view.changes)).not.toContain('"b"')
    })

    it("keeps one hash when the text is new", () => {
        const view = logEntryView(
            audit("AdminChange", {...details, changes: [{path: "body", new_sha256: "bb22"}]})
        )
        expect(view.changes).toEqual([
            {kind: ELogChangeKind.HASHES, path: "body", oldSha256: null, newSha256: "bb22"},
        ])
    })

    it.each(["harvest", "windmill"])("names %s as where it was made", (origin) => {
        expect(logEntryView(audit("AdminChange", {...details, origin})).origin).toBe(origin)
    })

    it("ignores an origin it doesn't know and changes without a path", () => {
        const view = logEntryView(
            audit("AdminChange", {
                ...details,
                origin: "elsewhere",
                changes: [{old: 1}, "x", null],
                object: {id: "1"},
                actor: null,
            })
        )
        expect(view.origin).toBeNull()
        expect(view.changes).toEqual([])
        expect(view.object).toBeNull()
        expect(view.who.role).toBeNull()
    })

    it("takes who from the actor when the message names nobody", () => {
        const view = logEntryView(
            audit("AdminChange", details, {username: null, userId: null, sender: ""})
        )
        expect(view.who).toMatchObject({username: "rosa.marin", userId: USER_ID})
    })

    it("reads a deleted election event's log from the extra details", () => {
        const view = logEntryView(
            audit("AdminChange", {
                object: {type: "election_event", id: EVENT_ID, name: "Training 2027"},
                action: "delete",
                actor: {user_id: USER_ID, username: "rosa.marin", role: "admin-user"},
                origin: "windmill",
                changes: [],
                extra: {
                    log_database: "tenant90505c8aevente2b7c9d4",
                    last_entry_id: 18442,
                    state_hash: "9f3c52e8d17e1a0b",
                    entries: 18442,
                    status: "RETIRED",
                    task_id: "7c1d81ab-0000-4000-8000-000000000001",
                    system_public_key: "bBSuL4bXEkBrqtmLXKFHmhxsZGJGCNG=",
                },
            })
        )
        expect(view.deletedEvent).toEqual({
            logDatabase: "tenant90505c8aevente2b7c9d4",
            lastEntryId: "18442",
            stateHash: "9f3c52e8d17e1a0b",
            entries: "18442",
            status: "RETIRED",
            taskId: "7c1d81ab-0000-4000-8000-000000000001",
            systemPublicKey: "bBSuL4bXEkBrqtmLXKFHmhxsZGJGCNG=",
        })
        expect(view.extra).toEqual([])
        expect(view.origin).toBe(ELogOrigin.WINDMILL)
    })

    it("reads the file an import read, with its count, and keeps other details", () => {
        const view = logEntryView(
            audit("AdminChange", {
                ...details,
                extra: {
                    count: 12,
                    file_sha256: "cc33",
                    document_id: "doc-1",
                    note: "x",
                    gone: null,
                },
            })
        )
        expect(view.deletedEvent).toBeNull()
        expect(view.importedFile).toEqual({
            documentId: "doc-1",
            fileSha256: "cc33",
            declaredSha256: null,
            declaredChecksum: null,
            counts: [{name: "count", value: "12"}],
        })
        expect(view.extra).toEqual([{name: "note", value: "x"}])
    })

    it("reads an import's counts and how its declared checksum compares", () => {
        const view = logEntryView(
            audit("AdminChange", {
                ...details,
                action: "import",
                extra: {
                    document_id: "doc-2",
                    file_sha256: "aa11",
                    declared_sha256: "bb22",
                    declared_checksum: "mismatch",
                    counts: {elections: 2, candidates: 7, areas: 0},
                },
            })
        )
        expect(view.importedFile).toEqual({
            documentId: "doc-2",
            fileSha256: "aa11",
            declaredSha256: "bb22",
            declaredChecksum: EImportChecksum.MISMATCH,
            counts: [
                {name: "elections", value: "2"},
                {name: "candidates", value: "7"},
                {name: "areas", value: "0"},
            ],
        })
        expect(view.extra).toEqual([])
    })

    it("ignores a checksum verdict it doesn't know", () => {
        const view = logEntryView(
            audit("AdminChange", {
                ...details,
                extra: {file_sha256: "aa11", declared_checksum: "maybe", counts: "many"},
            })
        )
        expect(view.importedFile).toMatchObject({declaredChecksum: null, counts: []})
    })

    it("has no imported file without a computed SHA-256", () => {
        const view = logEntryView(
            audit("AdminChange", {...details, extra: {declared_sha256: "bb22", count: 3}})
        )
        expect(view.importedFile).toBeNull()
        expect(view.extra).toEqual([
            {name: "declared_sha256", value: "bb22"},
            {name: "count", value: "3"},
        ])
    })
})

describe("KeycloakUserEvent", () => {
    const event = (content: unknown, type = "LOGIN_ERROR", options: IOptions = {}) =>
        row(
            "KeycloakUserEvent",
            {
                KeycloakUserEvent: [
                    typeof content === "string" ? content : JSON.stringify(content),
                    type,
                ],
            },
            options
        )

    it("reads the sign-in details of the tenant realm", () => {
        const view = logEntryView(
            event({
                error: "invalid_user_credentials",
                client_id: "admin-portal",
                ip_address: "203.0.113.77",
                auth_method: "openid-connect",
                auth_type: "code",
                credential_type: "otp",
                details: {redirect_uri: "https://admin.example/", empty: null},
            })
        )
        expect(view.signIn).toEqual({
            event: "LOGIN_ERROR",
            error: "invalid_user_credentials",
            clientId: "admin-portal",
            ipAddress: "203.0.113.77",
            authentication: "openid-connect · code",
            credential: "otp",
            failures: null,
            details: [{name: "redirect_uri", value: "https://admin.example/"}],
            text: null,
        })
        expect(view.who.unknownAccount).toBe(false)
    })

    it("marks an unknown account, which has no username", () => {
        const view = logEntryView(
            event(
                {error: "user_not_found", client_id: "admin-portal", unknown_account: true},
                "LOGIN_ERROR",
                {username: null, userId: null}
            )
        )
        expect(view.who).toEqual({
            username: null,
            userId: null,
            role: null,
            unknownAccount: true,
        })
        expect(view.signIn).toMatchObject({error: "user_not_found", authentication: null})
    })

    it.each(["num_failures", "numFailures"])("reads a lockout's failures from %s", (key) => {
        const view = logEntryView(
            event(
                {error: null, client_id: "admin-portal", details: {[key]: "30", reason: "locked"}},
                "USER_DISABLED_BY_TEMPORARY_LOCKOUT"
            )
        )
        expect(view.signIn).toMatchObject({
            event: "USER_DISABLED_BY_TEMPORARY_LOCKOUT",
            error: null,
            failures: "30",
            details: [{name: "reason", value: "locked"}],
        })
    })

    it("keeps the text of an entry that isn't structured", () => {
        const view = logEntryView(event("Voter signed in", "LOGIN"))
        expect(view.signIn).toMatchObject({event: "LOGIN", text: "Voter signed in", clientId: null})
        expect(view.who.unknownAccount).toBe(false)
    })

    it("has no sign-in section without the body", () => {
        expect(logEntryView(row("KeycloakUserEvent", null)).signIn).toBeNull()
        expect(logEntryView(row("KeycloakUserEvent", {KeycloakUserEvent: "x"})).signIn).toBeNull()
    })
})

describe("KeycloakAdminEvent", () => {
    const details = {
        operation: "CREATE",
        resource_type: "REALM_ROLE_MAPPING",
        resource_path: "users/b4d8e2a6-1c9f-4e37-8a5b-3f6c0d2e9b71/role-mappings/realm",
        representation: [{name: "security-officer"}],
        error: null,
        actor: {
            user_id: "5a3c7e91-6b2d-4f08-9c4e-1d8b5f3a6e27",
            username: "ops.lund",
            client_id: "security-admin-console",
            realm: "master",
            ip_address: "203.0.113.41",
        },
        made_by: "OPERATOR",
    }

    it("reads a change made in Keycloak's console", () => {
        const view = logEntryView(
            audit("KeycloakAdminEvent", details, {username: null, userId: null})
        )
        expect(view.origin).toBe(ELogOrigin.KEYCLOAK_CONSOLE)
        expect(view.keycloak).toEqual({
            origin: ELogOrigin.KEYCLOAK_CONSOLE,
            username: "ops.lund",
            userId: "5a3c7e91-6b2d-4f08-9c4e-1d8b5f3a6e27",
            realm: "master",
            clientId: "security-admin-console",
            ipAddress: "203.0.113.41",
            operation: "CREATE",
            resourceType: "REALM_ROLE_MAPPING",
            resourcePath: "users/b4d8e2a6-1c9f-4e37-8a5b-3f6c0d2e9b71/role-mappings/realm",
            appliesTo: "users/b4d8e2a6-1c9f-4e37-8a5b-3f6c0d2e9b71",
            representation: JSON.stringify([{name: "security-officer"}], null, 2),
            error: null,
        })
        expect(view.who).toMatchObject({
            username: "ops.lund",
            userId: "5a3c7e91-6b2d-4f08-9c4e-1d8b5f3a6e27",
        })
    })

    it("names Step's service account for a change the portal made", () => {
        const view = logEntryView(
            audit("KeycloakAdminEvent", {
                ...details,
                made_by: "SERVICE_ACCOUNT",
                representation: {username: "lena.ortiz", enabled: true},
            })
        )
        expect(view.origin).toBe(ELogOrigin.SERVICE_ACCOUNT)
        expect(view.keycloak).toMatchObject({
            origin: ELogOrigin.SERVICE_ACCOUNT,
            appliesTo: "lena.ortiz",
        })
    })

    it("accepts an event without a representation, a path or an actor", () => {
        const view = logEntryView(
            audit("KeycloakAdminEvent", {
                operation: "DELETE",
                resource_type: "USER",
                representation: null,
                error: "not_found",
            })
        )
        expect(view.keycloak).toMatchObject({
            representation: null,
            resourcePath: null,
            appliesTo: null,
            username: null,
            error: "not_found",
        })
    })

    it("keeps a representation sent as text", () => {
        const view = logEntryView(
            audit("KeycloakAdminEvent", {...details, representation: "enabled=false"})
        )
        expect(view.keycloak?.representation).toBe("enabled=false")
    })
})

describe("the log's own entries", () => {
    it("reads an export's format, scope, range, count and hash", () => {
        const view = logEntryView(
            audit("LogsExported", {
                format: "CSV",
                scope: "ACCOUNT_WITH_EVENTS",
                username: "rosa.marin",
                election_id: null,
                area_id: null,
                created_from: "2028-02-01T00:00:00Z",
                created_to: null,
                time_zone: "Europe/Madrid",
                entries: 1234,
                sha256: "dd44",
                document_id: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a01",
            })
        )
        expect(view.exported).toEqual({
            format: "CSV",
            scope: "ACCOUNT_WITH_EVENTS",
            username: "rosa.marin",
            electionId: null,
            areaId: null,
            createdFrom: "2028-02-01T00:00:00Z",
            createdTo: null,
            timeZone: "Europe/Madrid",
            entries: "1234",
            sha256: "dd44",
            documentId: "0f6f2a8e-3c1b-4d7e-9b2a-5e4d3c2b1a01",
        })
    })

    it.each(["LogWriterStarted", "LogWriterStopped"])("reads %s", (kind) => {
        const view = logEntryView(audit(kind, {release: "10.1.0", last_entry_id: null}))
        expect(view.writer).toEqual({release: "10.1.0", lastEntryId: null})
    })

    it("reads the key an AdminPublicKey entry publishes", () => {
        const view = logEntryView(
            row("AdminPublicKey", {AdminPublicKey: ["tenant", USER_ID, "MCowBQYDK2VwAyEAkey"]})
        )
        expect(view.publicKey).toBe("MCowBQYDK2VwAyEAkey")
    })
})

const tenantActivity = (kind: string, details: unknown, options: IOptions = {}) =>
    row(
        kind,
        {
            TenantActivity: {
                kind,
                event_type: "USER",
                log_type: "ERROR",
                description: "Request to delete an election event denied.",
                details_json: JSON.stringify(details),
            },
        },
        options
    )

describe("a request recorded as TenantAdminAction", () => {
    const details = {
        action: "delete-election-event",
        outcome: "denied",
        status: 403,
        user_id: USER_ID,
        username: "rosa.marin",
        details: {election_event_id: EVENT_ID},
    }

    it("was made in Harvest, by the account that asked", () => {
        const view = logEntryView(
            tenantActivity("TenantAdminAction", details, {username: null, userId: null})
        )
        expect(view).toMatchObject({
            unreadable: false,
            statementKind: "TenantAdminAction",
            origin: ELogOrigin.HARVEST,
            who: {username: "rosa.marin", userId: USER_ID},
            // The action, the outcome and what was named are the explanation's lines.
            action: null,
            changes: [],
            extra: [],
        })
    })

    it("keeps the account the message names", () => {
        const view = logEntryView(
            tenantActivity("TenantAdminAction", details, {username: "ada", userId: "ada-id"})
        )
        expect(view.who).toMatchObject({username: "ada", userId: "ada-id"})
    })

    it("reads a TenantLogOpened entry without a requester", () => {
        const view = logEntryView(
            tenantActivity(
                "TenantLogOpened",
                {board: "devtenant1", public_key: "key"},
                {username: null, userId: null}
            )
        )
        expect(view).toMatchObject({origin: null, who: {username: null, userId: null}})
    })
})

describe("values as shown", () => {
    it.each([
        [undefined, null],
        [null, null],
        ["", null],
        [[], null],
        [{}, null],
        ["UTC", "UTC"],
        [0, "0"],
        [false, false],
        [true, true],
        [["en", "es"], "en, es"],
        [[{a: 1}, ["x"]], '{"a":1}, ["x"]'],
        [{a: {b: null}, c: [1, 2]}, "a: b: -; c: 1, 2"],
    ])("shows %j as %j", (value, shown) => {
        expect(logValue(value)).toBe(shown)
    })

    it("shortens a long key and keeps a short one", () => {
        expect(shortened("MCowBQYDK2VwAyEA9sLwABCDEFGHIJKLMNOPW5c=")).toBe(
            "MCowBQYDK2VwAyEA9sLw…PW5c="
        )
        expect(shortened("short")).toBe("short")
    })
})

describe("field and object names", () => {
    it("names the tenant settings the Settings tabs save", () => {
        expect(logFieldLabelKey("tenant", "settings.logs_timezone")).toBe(
            "logsScreen.entry.field.tenant.logsTimezone"
        )
        expect(logFieldLabelKey("tenant", "voting_channels.kiosk")).toBe(
            "logsScreen.entry.field.tenant.kioskVoting"
        )
    })

    it("leaves any other path as logged", () => {
        expect(logFieldLabelKey("tenant", "settings.unknown")).toBeNull()
        expect(logFieldLabelKey("election", "settings.logs_timezone")).toBeNull()
        expect(logFieldLabelKey(null, "name")).toBeNull()
        expect(logFieldLabelKey("tenant", "constructor")).toBeNull()
    })

    it("names the object types it knows", () => {
        expect(logObjectTypeKey("staff_account")).toBe("logsScreen.entry.objectType.staff_account")
        expect(logObjectTypeKey("ballot_style")).toBeNull()
    })
})

describe("certified import and readiness evidence", () => {
    it("shows voter import hash, stable area counts and the authenticated actor", () => {
        const view = logEntryView(
            audit(
                "VotersImported",
                {
                    actor: {user_id: USER_ID, username: "certified.operator", role: "voter-write"},
                    import_id: "import-123",
                    document_id: "document-456",
                    file_name: "registry.csv",
                    sha256: "ab".repeat(32),
                    declared_sha256: "ab".repeat(32),
                    counts_by_area: {"Area A": 12, "Area B": 0},
                    imported_at: "2026-10-14T09:00:00Z",
                },
                {username: null, userId: null}
            )
        )
        expect(view.who).toMatchObject({
            username: "certified.operator",
            userId: USER_ID,
            role: "voter-write",
        })
        expect(view.extra).toEqual(
            expect.arrayContaining([
                {name: "import_id", value: "import-123"},
                {name: "sha256", value: "ab".repeat(32)},
                {name: "counts_by_area", value: "Area A: 12; Area B: 0"},
            ])
        )
    })
    it("keeps complete readiness checks and evidence visible in the event log", () => {
        const report = {
            official_event_id: EVENT_ID,
            tested_event_id: "tested-event",
            passed: true,
            checks: [
                {name: "configuration", passed: true, evidence: {content_sha256: "cd".repeat(32)}},
            ],
        }
        const view = logEntryView(
            audit("ReadinessVerified", {actor: {username: "operator"}, report})
        )
        expect(view.extra).toEqual(
            expect.arrayContaining([
                {name: "report", value: expect.stringContaining("tested-event")},
                {name: "report", value: expect.stringContaining("cd".repeat(32))},
            ])
        )
    })
})

describe("configuration package import evidence", () => {
    const details = {
        action: "Imported",
        external_id: "approved-configuration",
        revision: 9,
        manifest_sha256: "ab".repeat(32),
        design_digests: [],
    }
    it("shows the signed content hash separately from the manifest hash", () => {
        const view = logEntryView(
            row("ConfigurationPackageImported", {
                ConfigurationPackageWithContentHash: [
                    EVENT_ID,
                    {...details, content_hash: "cd".repeat(32)},
                ],
            })
        )
        expect(view.extra).toEqual(
            expect.arrayContaining([
                {name: "content_hash", value: "cd".repeat(32)},
                {name: "manifest_sha256", value: "ab".repeat(32)},
            ])
        )
    })
    it("shows historical import details without inventing a content hash", () => {
        const view = logEntryView(
            row("ConfigurationPackageImported", {
                ConfigurationPackage: [EVENT_ID, details],
            })
        )
        expect(view.extra).toContainEqual({name: "manifest_sha256", value: "ab".repeat(32)})
        expect(view.extra.some(({name}) => name === "content_hash")).toBe(false)
    })
})

// This exact body is asserted against the signed Rust election-board message and the Java queue producer.
describe("Scanovate failure contract", () => {
    it("shows every bounded field from the shared durable body", () => {
        const view = logEntryView(
            row("KeycloakUserEvent", scanovateContract.statement_body, {
                userId: scanovateContract.queue_input.user_id,
                username: scanovateContract.queue_input.username,
                head: {event: scanovateContract.queue_input.election_event_id, log_type: "ERROR"},
            })
        )
        const expected = JSON.parse(scanovateContract.queue_input.body)
        expect(view.signIn?.error).toBe("scanovate_verification_failed")
        expect(
            Object.fromEntries(view.signIn!.details.map(({name, value}) => [name, value]))
        ).toEqual(expected.details)
        expect(view.who.userId).toBe(scanovateContract.queue_input.user_id)
        expect(view.signIn?.text).toBeNull()
    })
    it("continues to display older unstructured failures", () => {
        const view = logEntryView(
            row("KeycloakUserEvent", {
                KeycloakUserEvent: ["scanovate_verification_failed", "REGISTER_ERROR"],
            })
        )
        expect(view.signIn?.text).toBe("scanovate_verification_failed")
        expect(view.signIn?.details).toEqual([])
    })
})
