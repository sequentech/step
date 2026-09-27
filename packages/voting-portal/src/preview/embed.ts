// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

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
 * screen the voter is on. Both sides refuse a version they do not know.
 */
export const EMBED_PROTOCOL = "sequent.voter-preview"
export const EMBED_VERSION = 1

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
    /** Embed to parent: a `show` was refused or the portal could not load its document. */
    FAILED = "failed",
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

export type EmbedReply =
    | {type: EmbedMessageType.READY}
    | {type: EmbedMessageType.SHOWN; screen?: PreviewScreen; path: string}
    | {type: EmbedMessageType.FAILED; issues: string[]}

export type EmbedMessage = (EmbedReply | ({type: EmbedMessageType.SHOW} & ShowRequest)) & {
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

export const embedMessage = <M extends EmbedReply | ({type: EmbedMessageType.SHOW} & ShowRequest)>(
    message: M
) => ({protocol: EMBED_PROTOCOL, version: EMBED_VERSION, ...message})

const isObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const isNonEmptyString = (value: unknown): value is string =>
    typeof value === "string" && value.length > 0

const describe = (value: unknown) => (value === undefined ? "nothing" : JSON.stringify(value))

/**
 * The request in a `show` message, or nothing for any other message. Other scripts post
 * messages to a window too, so only this protocol's `show` is read; a malformed one throws.
 */
export function readShowMessage(data: unknown): ShowRequest | undefined {
    if (!isObject(data) || data.protocol !== EMBED_PROTOCOL || data.type !== EmbedMessageType.SHOW)
        return undefined
    if (data.version !== EMBED_VERSION)
        throw new EmbedMessageError([
            `version: expected ${EMBED_VERSION}, found ${describe(data.version)}`,
        ])

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
