// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {EMessageChannel} from "@/types/messaging"
import {reasonText} from "@/services/messagingReasons"
import {IFakeMessageContent, IFakeProviderMessage, IFakeTemplateParameter} from "./types"

/** Historic synthetic senders keep their entries, shown with the demo labels. */
export const demoLabel = (value: string): string =>
    value
        .replace(/^Fake(?=[ A-Z])/, "Demo")
        .replace(/fake-(?=provider\.example\.org|wa-number-|waba-|infobip\.example\.org)/g, "demo-")
        .replace(/fake\.(?=elections|staff\.association)/g, "demo.")

/** A template parameter as written: `name: value` for a named one. */
export const parameterText = ({name, value}: IFakeTemplateParameter): string =>
    name ? `${name}: ${value}` : value

/** Whether an email has HTML: `has_html` in the list, the HTML itself in a full entry. */
export const hasHtml = (message: IFakeProviderMessage): boolean =>
    !!(message.content.has_html ?? message.content.html)

/** The template line of a template message: its name and language. */
export const templateLabel = (content: IFakeMessageContent, t: TFunction): string =>
    t("sentMessages.content.template", {
        name: content.template_name,
        language: content.template_language ?? "",
    })

/** One line for a table cell or a chat list. */
export const contentPreview = (message: IFakeProviderMessage, t: TFunction): string => {
    const {content} = message
    if (message.channel === EMessageChannel.EMAIL) {
        return content.subject || t("sentMessages.email.noSubject")
    }
    if (content.template_name) {
        const parameters = (content.parameters ?? []).map(parameterText).join(", ")
        const template = templateLabel(content, t)
        return parameters ? `${template}: ${parameters}` : template
    }
    return content.text || t("sentMessages.content.noText")
}

/**
 * The fake provider's reasons (`routes/fake_provider.rs`), verbatim, mapped to
 * their translations; the messaging reasons it shares through `reasonText`,
 * and any other as harvest wrote it.
 */
export const FAKE_PROVIDER_REASONS: Record<string, string> = {
    "Several accounts use this sender": "SEVERAL_ACCOUNTS",
    "The accounts couldn't be read": "ACCOUNTS_UNREADABLE",
    "The webhook component didn't answer": "UNREACHABLE",
    "Paste the m.me link from the enrollment page": "LINK_UNREADABLE",
    "The link has no ref: copy the whole link": "LINK_WITHOUT_REF",
    "No Messenger account of this organization uses this Page": "NO_PAGE_ACCOUNT",
    "Several Messenger accounts of this organization use this Page": "SEVERAL_PAGE_ACCOUNTS",
}

export const fakeReasonText = (t: TFunction, reason: string | null | undefined): string => {
    if (!reason) return t("sentMessages.report.noReason")
    const key = FAKE_PROVIDER_REASONS[reason]
    return key ? t(`sentMessages.reason.${key}`) : reasonText(t, reason)
}

const time = (message: IFakeProviderMessage) => Date.parse(message.received_at) || 0

/** Newest first, as the list and the inbox show them. */
export const newestFirst = (messages: IFakeProviderMessage[]): IFakeProviderMessage[] =>
    [...messages].sort((a, b) => time(b) - time(a))

export interface IConversation {
    /** Identifies the chat: its sender and recipient. */
    key: string
    sender: string
    recipient: string
    /** Oldest first, as a chat reads. */
    messages: IFakeProviderMessage[]
    latest: IFakeProviderMessage
}

/**
 * A chat per sender and recipient, as the apps keep one chat per business
 * number (R6-07); the most recent chat first.
 */
export const conversations = (messages: IFakeProviderMessage[]): IConversation[] => {
    const chats = new Map<string, IFakeProviderMessage[]>()
    for (const message of messages) {
        const key = JSON.stringify([message.sender, message.recipient])
        chats.set(key, [...(chats.get(key) ?? []), message])
    }
    return Array.from(chats, ([key, chat]) => {
        const ordered = [...chat].sort((a, b) => time(a) - time(b))
        const latest = ordered[ordered.length - 1]
        return {key, sender: latest.sender, recipient: latest.recipient, messages: ordered, latest}
    }).sort((a, b) => time(b.latest) - time(a.latest))
}

/** Date and time in the user's language; the raw value if it doesn't parse. */
export const formatTime = (value: string, language: string): string => {
    const date = new Date(value)
    if (Number.isNaN(date.getTime())) return value
    try {
        return date.toLocaleString(language, {dateStyle: "short", timeStyle: "medium"})
    } catch {
        return date.toLocaleString()
    }
}
