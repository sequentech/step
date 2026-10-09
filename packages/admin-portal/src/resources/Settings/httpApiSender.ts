// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The form of a provider described by configuration (HTTP_API) and the checks
// of its shape. Mirrors `HttpApiSender` in
// packages/sequent-core/src/types/messaging.rs; the server stays authoritative.
import {
    ECredentialName,
    EDigestEncoding,
    EJwtAlgorithm,
    EMessageAttemptState,
    EMessagePurpose,
    EMessagingProvider,
    EPhoneFormat,
    IAccountSender,
    IApprovedTemplates,
    IHttpJwt,
    IHttpReconcile,
    IHttpReports,
    IHttpRequestTemplate,
    IHttpTokenRequest,
    MESSAGE_ATTEMPT_STATES,
    MESSAGE_PURPOSES,
} from "@/types/messaging"
import {acceptedCredentials} from "@/services/messaging"

/** The optional parts of a configured provider, each edited as JSON. */
export enum EHttpSection {
    CHECK = "CHECK",
    TOKEN = "TOKEN",
    JWT = "JWT",
    REPORTS = "REPORTS",
    RECONCILE = "RECONCILE",
}

export const HTTP_SECTIONS: EHttpSection[] = [
    EHttpSection.CHECK,
    EHttpSection.TOKEN,
    EHttpSection.JWT,
    EHttpSection.REPORTS,
    EHttpSection.RECONCILE,
]

/** A part of the form that can have problems. */
export type THttpFormPart = EHttpSection | "SEND" | "MESSAGE_ID_POINTER" | "CONVERSATION_WINDOW"

export enum EHttpConfigProblem {
    NOT_AN_OBJECT = "NOT_AN_OBJECT",
    MISSING_URL = "MISSING_URL",
    INVALID_METHOD = "INVALID_METHOD",
    INVALID_HEADERS = "INVALID_HEADERS",
    UNKNOWN_FIELD = "UNKNOWN_FIELD",
    UNKNOWN_PLACEHOLDER = "UNKNOWN_PLACEHOLDER",
    INVALID_POINTER = "INVALID_POINTER",
    INVALID_STATES = "INVALID_STATES",
    INVALID_AUTH = "INVALID_AUTH",
    INVALID_LIFETIME = "INVALID_LIFETIME",
    INVALID_ALGORITHM = "INVALID_ALGORITHM",
    INVALID_CLAIMS = "INVALID_CLAIMS",
    INVALID_HOURS = "INVALID_HOURS",
}

export interface IHttpConfigProblem {
    problem: EHttpConfigProblem
    /** Where in the section, such as `headers.Authorization`; empty for the section itself. */
    path: string
}

export interface IHttpPlaceholder {
    placeholder: string
    /** Key of its explanation under `messagingAccounts.http.placeholder`. */
    id: string
}

/** Every placeholder a request may hold, in the order the reference lists them. */
export const HTTP_PLACEHOLDERS: IHttpPlaceholder[] = [
    {placeholder: "{{to}}", id: "to"},
    {placeholder: "{{text}}", id: "text"},
    {placeholder: "{{subject}}", id: "subject"},
    {placeholder: "{{html}}", id: "html"},
    {placeholder: "{{code}}", id: "code"},
    {placeholder: "{{template}}", id: "template"},
    {placeholder: "{{language}}", id: "language"},
    {placeholder: "{{message_id}}", id: "message_id"},
    {placeholder: "{{callback_url}}", id: "callback_url"},
    {placeholder: "{{param.1}}", id: "param"},
    {placeholder: "{{credential.NAME}}", id: "credential"},
    {placeholder: "{{basic_auth}}", id: "basic_auth"},
    {placeholder: "{{token}}", id: "token"},
    {placeholder: "{{jwt}}", id: "jwt"},
    {placeholder: "{{parameters}}", id: "parameters"},
    {placeholder: "{{named_parameters}}", id: "named_parameters"},
]

const PLAIN_PLACEHOLDERS = new Set([
    "to",
    "text",
    "subject",
    "html",
    "code",
    "template",
    "language",
    "message_id",
    "callback_url",
    "basic_auth",
    "token",
    "jwt",
    "parameters",
    "named_parameters",
])
const PLACEHOLDER = /\{\{([^{}]*)\}\}/g
const POSITIONAL = /^param\.[1-9]\d*$/
const CREDENTIAL = /^credential\.(.+)$/
const SIGNED_HEADER = /^header\.[^\s.]+$/

const requestPlaceholder = (name: string): boolean => {
    if (PLAIN_PLACEHOLDERS.has(name) || POSITIONAL.test(name)) {
        return true
    }
    const credential = CREDENTIAL.exec(name)?.[1]
    return (
        !!credential &&
        acceptedCredentials(EMessagingProvider.HTTP_API).includes(credential as ECredentialName)
    )
}

const signedPlaceholder = (name: string): boolean => name === "body" || SIGNED_HEADER.test(name)

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const at = (prefix: string, key: string): string => (prefix ? `${prefix}.${key}` : key)

const problem = (kind: EHttpConfigProblem, path: string): IHttpConfigProblem => ({
    problem: kind,
    path,
})

const unknownPlaceholders = (
    value: unknown,
    path: string,
    known: (name: string) => boolean = requestPlaceholder
): IHttpConfigProblem[] => {
    if (typeof value === "string") {
        const names = Array.from(value.matchAll(PLACEHOLDER), (match) => match[1])
        return names.every(known) ? [] : [problem(EHttpConfigProblem.UNKNOWN_PLACEHOLDER, path)]
    }
    if (Array.isArray(value)) {
        return value.flatMap((item, index) => unknownPlaceholders(item, at(path, String(index))))
    }
    if (isRecord(value)) {
        return Object.entries(value).flatMap(([key, item]) =>
            unknownPlaceholders(item, at(path, key))
        )
    }
    return []
}

const unknownFields = (
    value: Record<string, unknown>,
    known: string[],
    prefix: string
): IHttpConfigProblem[] =>
    Object.keys(value)
        .filter((key) => !known.includes(key))
        .map((key) => problem(EHttpConfigProblem.UNKNOWN_FIELD, at(prefix, key)))

const absent = (value: unknown): boolean => value === undefined || value === null

/** A JSON pointer (RFC 6901): empty for the whole document, or `/`-separated. */
const isPointer = (value: unknown): boolean =>
    typeof value === "string" && (value === "" || value.startsWith("/"))

const pointer = (value: unknown, path: string, optional: boolean): IHttpConfigProblem[] =>
    (optional && absent(value)) || isPointer(value)
        ? []
        : [problem(EHttpConfigProblem.INVALID_POINTER, path)]

const lifetime = (value: unknown, path: string): IHttpConfigProblem[] =>
    absent(value) || (typeof value === "number" && Number.isInteger(value) && value > 0)
        ? []
        : [problem(EHttpConfigProblem.INVALID_LIFETIME, path)]

/** Checks an `HttpRequestTemplate`: its fields and the placeholders it uses. */
export const validateHttpRequest = (value: unknown, prefix = ""): IHttpConfigProblem[] => {
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.NOT_AN_OBJECT, prefix)]
    }
    const problems: IHttpConfigProblem[] = []
    if (typeof value.url !== "string" || !value.url.trim()) {
        problems.push(problem(EHttpConfigProblem.MISSING_URL, at(prefix, "url")))
    } else {
        problems.push(...unknownPlaceholders(value.url, at(prefix, "url")))
    }
    if (!absent(value.method) && (typeof value.method !== "string" || !value.method.trim())) {
        problems.push(problem(EHttpConfigProblem.INVALID_METHOD, at(prefix, "method")))
    }
    if (!absent(value.headers)) {
        const headers = at(prefix, "headers")
        if (!isRecord(value.headers)) {
            problems.push(problem(EHttpConfigProblem.INVALID_HEADERS, headers))
        } else {
            for (const [name, header] of Object.entries(value.headers)) {
                problems.push(
                    ...(typeof header === "string"
                        ? unknownPlaceholders(header, at(headers, name))
                        : [problem(EHttpConfigProblem.INVALID_HEADERS, at(headers, name))])
                )
            }
        }
    }
    problems.push(...unknownPlaceholders(value.body, at(prefix, "body")))
    problems.push(...unknownFields(value, ["method", "url", "headers", "body"], prefix))
    return problems
}

const validateStatus = (value: unknown, prefix: string): IHttpConfigProblem[] => {
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.NOT_AN_OBJECT, prefix)]
    }
    const problems = [
        ...pointer(value.message_id_pointer, at(prefix, "message_id_pointer"), false),
        ...pointer(value.state_pointer, at(prefix, "state_pointer"), false),
    ]
    const states = at(prefix, "states")
    if (!isRecord(value.states) || Object.keys(value.states).length === 0) {
        problems.push(problem(EHttpConfigProblem.INVALID_STATES, states))
    } else {
        for (const [name, state] of Object.entries(value.states)) {
            if (!MESSAGE_ATTEMPT_STATES.includes(state as EMessageAttemptState)) {
                problems.push(problem(EHttpConfigProblem.INVALID_STATES, at(states, name)))
            }
        }
    }
    problems.push(...pointer(value.error_pointer, at(prefix, "error_pointer"), true))
    problems.push(
        ...unknownFields(
            value,
            ["message_id_pointer", "state_pointer", "states", "error_pointer"],
            prefix
        )
    )
    return problems
}

const validateAuth = (value: unknown, prefix: string): IHttpConfigProblem[] => {
    if (absent(value)) {
        return []
    }
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.INVALID_AUTH, prefix)]
    }
    const header = (): IHttpConfigProblem[] =>
        typeof value.header === "string" && value.header.trim()
            ? []
            : [problem(EHttpConfigProblem.INVALID_AUTH, at(prefix, "header"))]
    switch (value.kind) {
        case "URL_KEY":
            return unknownFields(value, ["kind"], prefix)
        case "HEADER_SECRET":
        case "JWT_HS256":
            return [...header(), ...unknownFields(value, ["kind", "header"], prefix)]
        case "HMAC_SHA256": {
            const problems = header()
            if (!absent(value.prefix) && typeof value.prefix !== "string") {
                problems.push(problem(EHttpConfigProblem.INVALID_AUTH, at(prefix, "prefix")))
            }
            if (
                !absent(value.encoding) &&
                !Object.values(EDigestEncoding).includes(value.encoding as EDigestEncoding)
            ) {
                problems.push(problem(EHttpConfigProblem.INVALID_AUTH, at(prefix, "encoding")))
            }
            if (!absent(value.signed)) {
                problems.push(
                    ...(typeof value.signed === "string"
                        ? unknownPlaceholders(value.signed, at(prefix, "signed"), signedPlaceholder)
                        : [problem(EHttpConfigProblem.INVALID_AUTH, at(prefix, "signed"))])
                )
            }
            return [
                ...problems,
                ...unknownFields(value, ["kind", "header", "prefix", "encoding", "signed"], prefix),
            ]
        }
        default:
            return [problem(EHttpConfigProblem.INVALID_AUTH, at(prefix, "kind"))]
    }
}

/** Checks an `HttpTokenRequest`. */
export const validateHttpToken = (value: unknown): IHttpConfigProblem[] => {
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.NOT_AN_OBJECT, "")]
    }
    return [
        ...validateHttpRequest(value.request, "request"),
        ...pointer(value.token_pointer, "token_pointer", false),
        ...lifetime(value.lifetime_seconds, "lifetime_seconds"),
        ...unknownFields(value, ["request", "token_pointer", "lifetime_seconds"], ""),
    ]
}

/** Checks an `HttpJwt`. */
export const validateHttpJwt = (value: unknown): IHttpConfigProblem[] => {
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.NOT_AN_OBJECT, "")]
    }
    const problems: IHttpConfigProblem[] = []
    if (
        !absent(value.algorithm) &&
        !Object.values(EJwtAlgorithm).includes(value.algorithm as EJwtAlgorithm)
    ) {
        problems.push(problem(EHttpConfigProblem.INVALID_ALGORITHM, "algorithm"))
    }
    if (!absent(value.claims)) {
        problems.push(
            ...(isRecord(value.claims)
                ? unknownPlaceholders(value.claims, "claims")
                : [problem(EHttpConfigProblem.INVALID_CLAIMS, "claims")])
        )
    }
    return [
        ...problems,
        ...lifetime(value.lifetime_seconds, "lifetime_seconds"),
        ...unknownFields(value, ["algorithm", "claims", "lifetime_seconds"], ""),
    ]
}

/** Checks an `HttpReports`. */
export const validateHttpReports = (value: unknown): IHttpConfigProblem[] => {
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.NOT_AN_OBJECT, "")]
    }
    return [
        ...validateAuth(value.auth, "auth"),
        ...pointer(value.items_pointer, "items_pointer", true),
        ...validateStatus(value.status, "status"),
        ...pointer(value.inbound_from_pointer, "inbound_from_pointer", true),
        ...unknownFields(value, ["auth", "items_pointer", "status", "inbound_from_pointer"], ""),
    ]
}

/** Checks an `HttpReconcile`. */
export const validateHttpReconcile = (value: unknown): IHttpConfigProblem[] => {
    if (!isRecord(value)) {
        return [problem(EHttpConfigProblem.NOT_AN_OBJECT, "")]
    }
    return [
        ...validateHttpRequest(value.request, "request"),
        ...validateStatus(value.status, "status"),
        ...unknownFields(value, ["request", "status"], ""),
    ]
}

const SECTION_VALIDATORS: Record<EHttpSection, (value: unknown) => IHttpConfigProblem[]> = {
    [EHttpSection.CHECK]: (value) => validateHttpRequest(value),
    [EHttpSection.TOKEN]: validateHttpToken,
    [EHttpSection.JWT]: validateHttpJwt,
    [EHttpSection.REPORTS]: validateHttpReports,
    [EHttpSection.RECONCILE]: validateHttpReconcile,
}

const EMPTY_STATUS = {message_id_pointer: "", state_pointer: "", states: {}}

/** What an optional section starts from when it is added. */
export const HTTP_SECTION_SKELETONS: Record<EHttpSection, Record<string, unknown>> = {
    [EHttpSection.CHECK]: {method: "GET", url: "", headers: {}},
    [EHttpSection.TOKEN]: {
        request: {method: "POST", url: "", headers: {}, body: {}},
        token_pointer: "/access_token",
        lifetime_seconds: 300,
    },
    [EHttpSection.JWT]: {algorithm: EJwtAlgorithm.RS256, claims: {}, lifetime_seconds: 300},
    [EHttpSection.REPORTS]: {auth: {kind: "URL_KEY"}, status: EMPTY_STATUS},
    [EHttpSection.RECONCILE]: {request: {method: "GET", url: ""}, status: EMPTY_STATUS},
}

/**
 * A worked example: a Viber partner that takes a template, its language and
 * parameters in a JSON POST authenticated with a bearer API key, answers with
 * the message's ID and posts delivery reports signed with a shared secret.
 */
export const HTTP_API_EXAMPLE: {
    send: IHttpRequestTemplate
    message_id_pointer: string
    reports: IHttpReports
} = {
    send: {
        method: "POST",
        url: "https://api.partner.example/v1/viber/messages",
        headers: {
            "Authorization": "Bearer {{credential.API_KEY}}",
            "Content-Type": "application/json",
        },
        body: {
            to: "{{to}}",
            template: "{{template}}",
            language: "{{language}}",
            parameters: "{{parameters}}",
            callback: "{{callback_url}}",
        },
    },
    message_id_pointer: "/message_id",
    reports: {
        auth: {kind: "HEADER_SECRET", header: "X-Webhook-Secret"},
        items_pointer: "/reports",
        status: {
            message_id_pointer: "/message_id",
            state_pointer: "/status",
            states: {
                sent: EMessageAttemptState.ACCEPTED,
                delivered: EMessageAttemptState.DELIVERED,
                failed: EMessageAttemptState.FAILED,
                expired: EMessageAttemptState.FAILED,
            },
            error_pointer: "/error",
        },
    },
}

export interface IHttpApiForm {
    phoneFormat: EPhoneFormat
    templateRequiredFor: EMessagePurpose[]
    conversationWindowHours: string
    messageIdPointer: string
    /** Per purpose, the approved template languages, separated by commas. */
    approvedLanguages: Record<EMessagePurpose, string>
    send: unknown
    /** The optional sections; `null` when not configured. */
    sections: Record<EHttpSection, unknown>
}

export const emptyHttpApiForm = (): IHttpApiForm => ({
    phoneFormat: EPhoneFormat.E164,
    templateRequiredFor: [],
    conversationWindowHours: "",
    messageIdPointer: "",
    approvedLanguages: {[EMessagePurpose.OTP]: "", [EMessagePurpose.NOTICE]: ""},
    send: {method: "POST", url: "", headers: {}, body: {}},
    sections: {
        [EHttpSection.CHECK]: null,
        [EHttpSection.TOKEN]: null,
        [EHttpSection.JWT]: null,
        [EHttpSection.REPORTS]: null,
        [EHttpSection.RECONCILE]: null,
    },
})

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T

/** Fills the send request, the message ID pointer and the reports with the example. */
export const withHttpExample = (form: IHttpApiForm): IHttpApiForm => ({
    ...form,
    send: clone(HTTP_API_EXAMPLE.send),
    messageIdPointer: HTTP_API_EXAMPLE.message_id_pointer,
    sections: {...form.sections, [EHttpSection.REPORTS]: clone(HTTP_API_EXAMPLE.reports)},
})

export const httpFormFromSender = (sender: IAccountSender): IHttpApiForm => {
    if (sender.provider !== EMessagingProvider.HTTP_API) {
        return emptyHttpApiForm()
    }
    const hours = sender.conversation_window_hours
    return {
        phoneFormat: sender.phone_format ?? EPhoneFormat.E164,
        templateRequiredFor: [...(sender.template_required_for ?? [])],
        conversationWindowHours: hours === null || hours === undefined ? "" : String(hours),
        messageIdPointer: sender.message_id_pointer ?? "",
        approvedLanguages: {
            [EMessagePurpose.OTP]: (sender.approved_templates?.OTP ?? []).join(", "),
            [EMessagePurpose.NOTICE]: (sender.approved_templates?.NOTICE ?? []).join(", "),
        },
        send: sender.send,
        sections: {
            [EHttpSection.CHECK]: sender.check ?? null,
            [EHttpSection.TOKEN]: sender.token ?? null,
            [EHttpSection.JWT]: sender.jwt ?? null,
            [EHttpSection.REPORTS]: sender.reports ?? null,
            [EHttpSection.RECONCILE]: sender.reconcile ?? null,
        },
    }
}

const HOURS = /^[1-9]\d*$/

const languages = (text: string): string[] =>
    Array.from(new Set(text.split(/[\s,;]+/).filter(Boolean)))

/** The problems of each part of the form; empty when it can be saved. */
export const httpFormProblems = (
    form: IHttpApiForm
): Partial<Record<THttpFormPart, IHttpConfigProblem[]>> => {
    const found: Partial<Record<THttpFormPart, IHttpConfigProblem[]>> = {}
    const add = (part: THttpFormPart, problems: IHttpConfigProblem[]) => {
        if (problems.length) {
            found[part] = problems
        }
    }
    add("SEND", validateHttpRequest(form.send))
    for (const section of HTTP_SECTIONS) {
        const value = form.sections[section]
        if (!absent(value)) {
            add(section, SECTION_VALIDATORS[section](value))
        }
    }
    const messageId = form.messageIdPointer.trim()
    if (messageId && !isPointer(messageId)) {
        add("MESSAGE_ID_POINTER", [problem(EHttpConfigProblem.INVALID_POINTER, "")])
    }
    const hours = form.conversationWindowHours.trim()
    if (hours && !HOURS.test(hours)) {
        add("CONVERSATION_WINDOW", [problem(EHttpConfigProblem.INVALID_HOURS, "")])
    }
    return found
}

export const httpSenderFromForm = (label: string, form: IHttpApiForm): IAccountSender => {
    const approved: IApprovedTemplates = {}
    for (const purpose of MESSAGE_PURPOSES) {
        const codes = languages(form.approvedLanguages[purpose])
        if (codes.length) {
            approved[purpose] = codes
        }
    }
    const hours = form.conversationWindowHours.trim()
    const section = <T>(name: EHttpSection): T | null => (form.sections[name] ?? null) as T | null
    return {
        provider: EMessagingProvider.HTTP_API,
        label: label.trim() || null,
        send: form.send as IHttpRequestTemplate,
        message_id_pointer: form.messageIdPointer.trim() || null,
        phone_format: form.phoneFormat,
        template_required_for: MESSAGE_PURPOSES.filter((purpose) =>
            form.templateRequiredFor.includes(purpose)
        ),
        conversation_window_hours: HOURS.test(hours) ? Number(hours) : null,
        check: section<IHttpRequestTemplate>(EHttpSection.CHECK),
        token: section<IHttpTokenRequest>(EHttpSection.TOKEN),
        jwt: section<IHttpJwt>(EHttpSection.JWT),
        reports: section<IHttpReports>(EHttpSection.REPORTS),
        reconcile: section<IHttpReconcile>(EHttpSection.RECONCILE),
        approved_templates: approved,
    }
}
