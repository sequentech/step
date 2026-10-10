// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {readFileSync} from "fs"
import {resolve} from "path"
import type {TFunction} from "i18next"
import en from "@/translations/en"
import {EReadinessBlocker} from "@/types/messaging"
import {KNOWN_REASONS, REASON_SHAPES, actionRefusalReason, reasonText} from "./messagingReasons"

const PACKAGES = resolve(__dirname, "../../..")
const source = (path: string) => readFileSync(resolve(PACKAGES, path), "utf8")

/** The body of `pub enum <name> { … }` in a Rust source. */
const rustEnum = (text: string, name: string): string => {
    const start = text.indexOf(`pub enum ${name} {`)
    expect(start).toBeGreaterThanOrEqual(0)
    return text.slice(start, text.indexOf("\n}", start))
}

// Every reason the backend writes, read from the backend's own sources, so a
// new or reworded reason fails here until it is translated (R4-06).
const backendReasons = (): string[] => {
    const core = source("sequent-core/src/types/messaging.rs")
    const messageReasons = Array.from(
        rustEnum(core, "MessageReason").matchAll(/#\[strum\(serialize = "([^"]+)"\)\]/g),
        (match) => match[1]
    )
    const preflight = Array.from(
        rustEnum(source("messaging/src/sender.rs"), "PreflightError").matchAll(/^\s+([A-Z_]+),$/gm),
        (match) => match[1]
    )
    return [
        ...messageReasons,
        ...preflight,
        constant("windmill/src/postgres/messaging.rs", "ENDPOINT_CREDENTIALS_REASON"),
        constant("windmill/src/postgres/messaging.rs", "ENDPOINT_HTTPS_REASON"),
        constant("windmill/src/services/messaging/accounts.rs", "CALLBACK_NOT_CONFIGURED"),
        constant("windmill/src/services/messaging/accounts.rs", "REPORT_CREDENTIALS_MISSING"),
        constant("windmill/src/services/messaging/accounts.rs", "CALLBACK_NOT_HTTPS"),
        constant("windmill/src/services/messaging/accounts.rs", "CANDIDATE_REFUSED_REASON"),
        constant("windmill/src/services/messaging/accounts.rs", "ACCOUNT_IN_USE_REASON"),
        constant("windmill/src/services/messaging/accounts.rs", "NUMBER_CHANGED_REASON"),
        constant("windmill/src/postgres/messaging.rs", "IDENTITY_CHANGED_REASON"),
        constant("windmill/src/postgres/rate_bucket.rs", "CAPACITY_BUSY"),
        constant("windmill/src/services/messaging/dispatch.rs", "PROVIDER_ANSWER_LATE"),
        constant("messaging/src/providers/mod.rs", "TEMPLATES_NOT_READ"),
        constant("messaging/src/providers/infobip.rs", "PARTNER_LOGIN_REFUSED"),
        constant("messaging/src/providers/messenger.rs", "NOT_SUBSCRIBED"),
        constant("messaging/src/providers/messenger.rs", "NO_PAGES_MESSAGING"),
        constant("messaging/src/providers/whatsapp.rs", "NUMBER_STATUS_NOT_READ"),
        constant("windmill/src/services/messaging/accounts.rs", "CHECK_DROPPED_REASON"),
        ...PROVIDER_REASONS.map((name) => constant("messaging/src/providers/mod.rs", name)),
    ]
}

/** A `pub const <name>: &str = "…"` of a Rust source. */
const constant = (path: string, name: string) => {
    const match = new RegExp(`pub const ${name}: &str =\\s*"([^"]+)"`).exec(source(path))
    expect(match).not.toBeNull()
    return match![1]
}

/** The provider checks' constant reasons (R5-15). */
const PROVIDER_REASONS = [
    "PROVIDER_UNREACHABLE",
    "NUMBER_IN_SANDBOX",
    "NUMBER_NOT_VERIFIED",
    "SMS_SANDBOX",
    "SES_SANDBOX",
    "SES_SENDING_DISABLED",
    "SMTP_REFUSED",
    "TOKEN_INVALID",
    "TOKEN_CANNOT_SEND",
    "TOKEN_NOT_CHECKED",
    "APP_SECRET_REFUSED",
    "APP_SECRET_NOT_CHECKED",
]

/** The backend's reasons with a value in them: where each one starts. */
const backendShapes = (): string[] => [
    `${constant("windmill/src/services/messaging/accounts.rs", "NOT_READY_REASON")} (`,
    ...[
        "PROVIDER_STATUS_PREFIX",
        "MISSING_CREDENTIAL_PREFIX",
        "CHECK_FAILED_PREFIX",
        "NUMBER_STATUS_PREFIX",
        "NUMBER_NAME_PREFIX",
    ].map((name) => constant("messaging/src/providers/mod.rs", name)),
]

const reasonKeys = Object.keys(
    (en.translations.messaging as unknown as {reason: Record<string, string>}).reason
)

const keyOnly = ((key: string, options?: {code?: string}) =>
    options?.code ? `${key}(${options.code})` : key) as unknown as TFunction

describe("backend reasons (R4-06)", () => {
    it("knows every reason the backend writes", () => {
        const reasons = backendReasons()
        expect(reasons.length).toBeGreaterThan(20)
        expect(reasons.filter((reason) => !KNOWN_REASONS[reason])).toEqual([])
    })

    it("translates every known reason", () => {
        expect(Object.values(KNOWN_REASONS).filter((code) => !reasonKeys.includes(code))).toEqual(
            []
        )
    })

    it("knows every reason with a value in it, and translates it with the value (R5-15)", () => {
        expect(backendShapes()).toEqual(REASON_SHAPES.map((shape) => shape.prefix))
        expect(
            REASON_SHAPES.map((shape) => shape.code).filter((code) => !reasonKeys.includes(code))
        ).toEqual([])
        const withValues = ((key: string, options?: Record<string, string>) =>
            options && Object.keys(options).length
                ? `${key}(${Object.values(options).join()})`
                : key) as unknown as TFunction
        expect(reasonText(withValues, "The provider answered with status 401")).toBe(
            "messaging.reason.PROVIDER_STATUS(401)"
        )
        expect(reasonText(withValues, "Not sent: the account isn't ready (UNCHECKED)")).toBe(
            "messaging.reason.ACCOUNT_NOT_READY(messaging.blocker.UNCHECKED(UNCHECKED))"
        )
        expect(
            reasonText(
                withValues,
                "The access token lacks the permission to send; " +
                    "The new credentials didn't connect: the previous ones are kept"
            )
        ).toBe("messaging.reason.TOKEN_CANNOT_SEND; messaging.reason.CANDIDATE_REFUSED")
    })

    it("shows an unknown reason as the backend wrote it, and a provider's code translated", () => {
        expect(reasonText(keyOnly, "Rate limit hit (131056)")).toBe("Rate limit hit (131056)")
        expect(reasonText(keyOnly, "provider code 131026")).toBe(
            "messaging.reason.PROVIDER_CODE(131026)"
        )
    })

    it("splits only on the last separator: a provider's text may hold one (Follow-up 2)", () => {
        expect(
            reasonText(keyOnly, "Bad request; invalid param (100); Reported too late to retry")
        ).toBe("Bad request; invalid param (100); messaging.reason.REPORTED_TOO_LATE")
        expect(reasonText(keyOnly, "a; b")).toBe("a; b")
    })

    it("keeps the real failure next to why it isn't retried", () => {
        expect(
            reasonText(keyOnly, "Invalid destination for the channel; Reported too late to retry")
        ).toBe("messaging.reason.INVALID_DESTINATION; messaging.reason.REPORTED_TOO_LATE")
    })
})

describe("readiness blockers", () => {
    it("has every blocker the backend can give, each with a label", () => {
        const rust = Array.from(
            rustEnum(source("sequent-core/src/types/messaging.rs"), "ReadinessBlocker").matchAll(
                /^\s+([A-Z_]+),$/gm
            ),
            (match) => match[1]
        )
        expect(Object.values(EReadinessBlocker).sort()).toEqual(rust.sort())
        const labels = Object.keys(
            (en.translations.messaging as unknown as {blocker: Record<string, string>}).blocker
        )
        expect(rust.filter((blocker) => !labels.includes(blocker))).toEqual([])
    })
})

describe("actionRefusalReason", () => {
    it("reads the reason Hasura forwarded", () => {
        expect(actionRefusalReason({graphQLErrors: [{message: "Select at least one voter"}]})).toBe(
            "Select at least one voter"
        )
    })

    it("reads harvest's JSON or plain-text answer before Hasura's own message", () => {
        const wrapped = (body: string) => ({
            graphQLErrors: [
                {
                    message: "not a valid json response from webhook",
                    extensions: {internal: {response: {body}}},
                },
            ],
        })
        expect(actionRefusalReason(wrapped('{"message": "Callback address not configured"}'))).toBe(
            "Callback address not configured"
        )
        expect(actionRefusalReason(wrapped("unknown account"))).toBe("unknown account")
    })
})
