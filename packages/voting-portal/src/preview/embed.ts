// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    keypadHint as sharedKeypadHint,
    type IvrCallStatus,
    type IvrEmulatorConfig,
} from "@sequentech/ui-essentials"
import {DEMO_PUBLIC_KEY} from "@sequentech/ui-test-kit/fixtures"
import {
    previewIssues,
    ScenarioChannel,
    type PreviewDocument,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {isPreviewScreen, PREVIEW_SCREENS, PreviewScreen, type PreviewSource} from "./screens"

/**
 * The messages between the embedded voter preview (`workbench/embed.html`) and the window
 * that frames it, such as the Election Architect. The parent sends a publication preview
 * document; the embed renders the portal's production screens for it and reports which
 * screen the voter is on. Or the parent places a telephone call: the embed runs the IVR
 * emulator it names and reports the call's status. Both sides refuse a version they do
 * not know.
 *
 * Version 2 added the call (`call` and `calling`).
 */
export const EMBED_PROTOCOL = "sequent.voter-preview"
export const EMBED_VERSION = 2

/**
 * The key the election-config core writes before a key ceremony: eight zero bytes,
 * deliberately not a key, so nothing can be encrypted to it.
 */
export const NOT_A_KEY = "AAAAAAAAAAA="

/** The tenant of a document whose event names none; nothing is fetched with it. */
export const EMBED_TENANT = "preview"

export enum EmbedMessageType {
    /** Embed to parent: listening for `show`. Sent once the page has loaded. */
    READY = "ready",
    /** Parent to embed: open a new voter session at a screen. */
    SHOW = "show",
    /** Embed to parent: the voter is on this screen, after `show` or their own navigation. */
    SHOWN = "shown",
    /** Embed to parent: a request was refused, or the portal or emulator could not load. */
    FAILED = "failed",
    /** Parent to embed: place a telephone call against the IVR emulator. */
    CALL = "call",
    /** Embed to parent: where the call is, or that no emulator is served at its URL. */
    CALLING = "calling",
}

export interface ShowRequest {
    document: PreviewDocument
    areaId: string
    /** The election whose screens open; by default the area's first. */
    electionId?: string
    screen: PreviewScreen
    /** A voting portal language code; by default the event's own. */
    language?: string
    /** How the voter reaches the portal; online by default. */
    channel?: ScenarioChannel
}

/** The words around a call, in the framing tool's language; English by default. */
export interface CallLabels {
    /** The keypad's accessible name. */
    input?: string
    /** The keypad's placeholder: `{{maxDigits}}`, `{{validInputs}}` and `{{timeout}}` are filled in. */
    placeholder?: string
    /**
     * The keypad's placeholder when any digits will do, such as for a PIN, which the Lambda
     * says by listing no valid inputs: `{{maxDigits}}` and `{{timeout}}` are filled in.
     */
    placeholderAnyKeys?: string
    /** The button that lets the caller's patience run out. */
    timeout?: string
    /** The button that presses the keys. */
    send?: string
    /** The line under a call that has ended. */
    disconnected?: string
    /** What the embed says while the emulator loads. */
    connecting?: string
}

export const CALL_LABEL_KEYS: readonly (keyof CallLabels)[] = [
    "input",
    "placeholder",
    "placeholderAnyKeys",
    "timeout",
    "send",
    "disconnected",
    "connecting",
]

export interface CallRequest {
    /** What the IVR Lambda is given in production: the event, the open ballots and the caller. */
    config: IvrEmulatorConfig
    /**
     * The emulator's base URL, absolute: `<url>.js` and `<url>_bg.wasm` are what
     * wasm-bindgen emits. The framing tool serves it, so it names it.
     */
    emulatorUrl: string
    labels?: CallLabels
}

/** `absent`: nothing is served at the emulator's URL, the normal case away from a deployment. */
export type CallStatus = IvrCallStatus | "loading" | "absent"

export type EmbedReply =
    | {type: EmbedMessageType.READY}
    | {type: EmbedMessageType.SHOWN; screen?: PreviewScreen; path: string}
    | {type: EmbedMessageType.FAILED; issues: string[]}
    | {type: EmbedMessageType.CALLING; status: CallStatus}

export type EmbedRequest =
    | ({type: EmbedMessageType.SHOW} & ShowRequest)
    | ({type: EmbedMessageType.CALL} & CallRequest)

export type EmbedMessage = (EmbedReply | EmbedRequest) & {
    protocol: typeof EMBED_PROTOCOL
    version: typeof EMBED_VERSION
}

/** A message of this protocol that cannot be followed; `issues` name each field. */
export class EmbedMessageError extends Error {
    readonly issues: string[]

    constructor(issues: string[]) {
        super(`Invalid voter preview message:\n- ${issues.join("\n- ")}`)
        this.name = "EmbedMessageError"
        this.issues = issues
    }
}

export const embedMessage = <M extends EmbedReply | EmbedRequest>(message: M) => ({
    protocol: EMBED_PROTOCOL,
    version: EMBED_VERSION,
    ...message,
})

const isObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const isNonEmptyString = (value: unknown): value is string =>
    typeof value === "string" && value.length > 0

const describe = (value: unknown) => (value === undefined ? "nothing" : JSON.stringify(value))

/** This protocol's message of that type, of this version; a message of another version throws. */
function ofType(data: unknown, type: EmbedMessageType): Record<string, unknown> | undefined {
    if (!isObject(data) || data.protocol !== EMBED_PROTOCOL || data.type !== type) return undefined
    if (data.version !== EMBED_VERSION)
        throw new EmbedMessageError([
            `version: expected ${EMBED_VERSION}, found ${describe(data.version)}`,
        ])
    return data
}

/**
 * The request in a `show` message, or nothing for any other message. Other scripts post
 * messages to a window too, so only this protocol's `show` is read; a malformed one throws.
 */
export function readShowMessage(message: unknown): ShowRequest | undefined {
    const data = ofType(message, EmbedMessageType.SHOW)
    if (!data) return undefined

    const issues: string[] = []
    if (!isNonEmptyString(data.areaId))
        issues.push(`areaId: expected a non-empty string, found ${describe(data.areaId)}`)
    if (!isPreviewScreen(data.screen))
        issues.push(
            `screen: expected one of ${PREVIEW_SCREENS.join(", ")}, found ${describe(data.screen)}`
        )
    for (const key of ["electionId", "language"])
        if (data[key] !== undefined && !isNonEmptyString(data[key]))
            issues.push(`${key}: expected a non-empty string, found ${describe(data[key])}`)
    const channels = Object.values(ScenarioChannel) as unknown[]
    if (data.channel !== undefined && !channels.includes(data.channel))
        issues.push(
            `channel: expected one of ${channels.join(", ")}, found ${describe(data.channel)}`
        )
    issues.push(...previewIssues(data.document, data.areaId))
    if (issues.length) throw new EmbedMessageError(issues)

    return {
        document: data.document as PreviewDocument,
        areaId: data.areaId as string,
        electionId: data.electionId as string | undefined,
        screen: data.screen as PreviewScreen,
        language: data.language as string | undefined,
        channel: data.channel as ScenarioChannel | undefined,
    }
}

const isStringList = (value: unknown): value is string[] =>
    Array.isArray(value) && value.every((each) => typeof each === "string")

const CONFIG_STRINGS = [
    "caller_number",
    "contact_id",
    "tenant_id",
    "election_event_id",
    "election_event",
] as const
const CONFIG_LISTS = ["ballot_styles", "open_elections", "blacklisted_numbers"] as const

/**
 * The request in a `call` message, or nothing for any other message; a malformed one
 * throws with every problem named. The emulator is only reached over http(s), and its
 * configuration is checked field by field, because the WebAssembly reports a missing
 * field as a panic rather than a sentence.
 */
export function readCallMessage(message: unknown): CallRequest | undefined {
    const data = ofType(message, EmbedMessageType.CALL)
    if (!data) return undefined

    const issues: string[] = []
    let emulatorUrl: URL | undefined
    try {
        emulatorUrl = isNonEmptyString(data.emulatorUrl) ? new URL(data.emulatorUrl) : undefined
    } catch {
        emulatorUrl = undefined
    }
    if (!emulatorUrl || !["http:", "https:"].includes(emulatorUrl.protocol))
        issues.push(
            `emulatorUrl: expected an absolute http(s) URL, found ${describe(data.emulatorUrl)}`
        )

    const config = data.config
    if (!isObject(config)) issues.push(`config: expected an object, found ${describe(config)}`)
    else {
        for (const key of CONFIG_STRINGS)
            if (!isNonEmptyString(config[key]))
                issues.push(
                    `config.${key}: expected a non-empty string, found ${describe(config[key])}`
                )
        for (const key of CONFIG_LISTS)
            if (!isStringList(config[key]))
                issues.push(
                    `config.${key}: expected a list of strings, found ${describe(config[key])}`
                )
        if (isStringList(config.ballot_styles) && config.ballot_styles.length === 0)
            issues.push("config.ballot_styles: a call needs at least one ballot style")
    }

    const labels = data.labels
    if (labels !== undefined) {
        if (!isObject(labels)) issues.push(`labels: expected an object, found ${describe(labels)}`)
        else
            for (const key of CALL_LABEL_KEYS)
                if (labels[key] !== undefined && !isNonEmptyString(labels[key]))
                    issues.push(
                        `labels.${key}: expected a non-empty string, found ${describe(labels[key])}`
                    )
    }
    if (issues.length) throw new EmbedMessageError(issues)

    return {
        config: config as unknown as IvrEmulatorConfig,
        emulatorUrl: data.emulatorUrl as string,
        labels: labels as CallLabels | undefined,
    }
}

/** A label's `{{name}}`s, filled in from `values`; an unknown name is left as it is. */
export const fillLabel = (template: string, values: Record<string, string | number>) =>
    template.replace(/\{\{\s*(\w+)\s*\}\}/g, (whole, name: string) =>
        name in values ? String(values[name]) : whole
    )

/**
 * What the keypad says while the call waits for keys, in a framing tool's words:
 * `placeholderAnyKeys` when the Lambda lists no keys because any digits will do.
 */
export const keypadHint = (
    labels: Required<Pick<CallLabels, "placeholder" | "placeholderAnyKeys">>,
    expected: {valid_inputs: string; max_digits: number; timeout: number}
): string =>
    sharedKeypadHint(expected, {
        listed: (values) => fillLabel(labels.placeholder, values),
        anyKeys: (values) => fillLabel(labels.placeholderAnyKeys, values),
    })

/**
 * The document, with the demo key where it has the core's stand-in.
 *
 * Review and confirmation show an encrypted ballot, and a document from before the key
 * ceremony (the Election Architect's) has nothing to encrypt to. The embed never sends
 * a ballot anywhere, so it encrypts to the demo key and says so: the key is `is_demo`,
 * which the portal's screens mark. Any other key is left as it is.
 */
function withDemoKey(document: PreviewDocument): PreviewDocument {
    const standIn = (style: PreviewDocument["ballot_styles"][number]) => {
        const key = style.public_key
        return isObject(key) && key.public_key === NOT_A_KEY
    }
    if (!document.ballot_styles.some(standIn)) return document
    return {
        ...document,
        ballot_styles: document.ballot_styles.map((style) =>
            standIn(style)
                ? {...style, public_key: {public_key: DEMO_PUBLIC_KEY, is_demo: true}}
                : style
        ),
    }
}

/** What the preview renders for a request. */
export function embeddedSource({
    document,
    areaId,
    electionId,
    channel,
}: ShowRequest): PreviewSource {
    const tenant = document.election_event.tenant_id
    return {
        tenantId: isNonEmptyString(tenant) ? tenant : EMBED_TENANT,
        areaId,
        electionId,
        channel: channel ?? ScenarioChannel.ONLINE,
        preview: withDemoKey(document),
    }
}
