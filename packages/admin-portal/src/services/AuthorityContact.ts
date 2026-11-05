// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election authority's contact (Vitrine specs/policy.md, section 3,
// rule 19): how voters reach it, for example to report an audit mismatch.
// The Transparency tab stores it in `presentation.authority_contact`, and
// windmill drafts one `contact` line per kind from it. The checks here are
// the policy's, so that what the tab saves is what the draft can carry; the
// draft is checked again on the server.

export enum EContactKind {
    NAME = "name",
    EMAIL = "email",
    PHONE = "phone",
    URL = "url",
}

export const CONTACT_KINDS = [
    EContactKind.NAME,
    EContactKind.EMAIL,
    EContactKind.PHONE,
    EContactKind.URL,
] as const

export type IAuthorityContact = Record<EContactKind, string>

export const EMPTY_CONTACT: IAuthorityContact = {
    [EContactKind.NAME]: "",
    [EContactKind.EMAIL]: "",
    [EContactKind.PHONE]: "",
    [EContactKind.URL]: "",
}

const MAX_NAME = 128
const MAX_EMAIL = 254
const MAX_LOCAL = 64
const MAX_LABEL = 63

/** The Unicode 16.0 format characters that Vitrine's `text` excludes
 * (specs/entry.md, section 3): [first, last] code points. */
const FORMAT_RANGES: readonly (readonly [number, number])[] = [
    [0x00ad, 0x00ad],
    [0x0600, 0x0605],
    [0x061c, 0x061c],
    [0x06dd, 0x06dd],
    [0x070f, 0x070f],
    [0x0890, 0x0891],
    [0x08e2, 0x08e2],
    [0x180e, 0x180e],
    [0x200b, 0x200f],
    [0x202a, 0x202e],
    [0x2060, 0x2064],
    [0x2066, 0x206f],
    [0xfeff, 0xfeff],
    [0xfff9, 0xfffb],
    [0x110bd, 0x110bd],
    [0x110cd, 0x110cd],
    [0x13430, 0x1343f],
    [0x1bca0, 0x1bca3],
    [0x1d173, 0x1d17a],
    [0xe0001, 0xe0001],
    [0xe0020, 0xe007f],
]

const isPlainChar = (code: number): boolean =>
    !(code < 0x20 || (code >= 0x7f && code <= 0x9f)) &&
    !FORMAT_RANGES.some(([first, last]) => code >= first && code <= last)

const utf8Length = (value: string): number => new TextEncoder().encode(value).length

const isName = (value: string): boolean =>
    utf8Length(value) >= 1 &&
    utf8Length(value) <= MAX_NAME &&
    Array.from(value).every((ch) => isPlainChar(ch.codePointAt(0) ?? 0)) &&
    !value.startsWith(" ") &&
    !value.endsWith(" ") &&
    !value.includes("  ")

const isEmail = (value: string): boolean => {
    if (value.length > MAX_EMAIL || !/^[\x21-\x7e]*$/.test(value)) return false
    const parts = value.split("@")
    if (parts.length !== 2) return false
    const [local, domain] = parts as [string, string]
    if (local.length < 1 || local.length > MAX_LOCAL || !/^[A-Za-z0-9._%+-]+$/.test(local)) {
        return false
    }
    if (local.startsWith(".") || local.endsWith(".") || local.includes("..")) return false
    const labels = domain.split(".")
    return (
        labels.length >= 2 &&
        labels.every(
            (label) =>
                label.length <= MAX_LABEL && /^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?$/.test(label)
        )
    )
}

const isPhone = (value: string): boolean => /^\+[1-9][0-9]{6,14}$/.test(value)

/** An absolute `https` URL with a host and no user information. */
const isUrl = (value: string): boolean => {
    if (!/^[\x21-\x7e]+$/.test(value) || !value.startsWith("https://")) return false
    try {
        const url = new URL(value)
        return url.protocol === "https:" && url.hostname !== "" && !url.username && !url.password
    } catch {
        return false
    }
}

const CHECKS: Record<EContactKind, (value: string) => boolean> = {
    [EContactKind.NAME]: isName,
    [EContactKind.EMAIL]: isEmail,
    [EContactKind.PHONE]: isPhone,
    [EContactKind.URL]: isUrl,
}

/** The kinds whose value breaks the policy's syntax. A value is read as the
 * admin typed it, trimmed; an empty one is not set, so it is never an error. */
export const contactErrors = (contact: IAuthorityContact): EContactKind[] =>
    CONTACT_KINDS.filter((kind) => {
        const value = contact[kind].trim()
        return value !== "" && !CHECKS[kind](value)
    })

const asObject = (value: unknown): Record<string, unknown> | undefined => {
    let v = value
    if (typeof v === "string") {
        try {
            v = JSON.parse(v)
        } catch {
            return undefined
        }
    }
    return v !== null && typeof v === "object" && !Array.isArray(v)
        ? (v as Record<string, unknown>)
        : undefined
}

/** The contact of an election event's `presentation`: each kind as stored, or
 * empty. */
export const readAuthorityContact = (presentation: unknown): IAuthorityContact => {
    const stored = asObject(asObject(presentation)?.authority_contact)
    const contact = {...EMPTY_CONTACT}
    for (const kind of CONTACT_KINDS) {
        const value = stored?.[kind]
        if (typeof value === "string") contact[kind] = value
    }
    return contact
}

/** What to append to the event's `presentation` to store the contact: each
 * kind that is set, trimmed. `_append` replaces the whole `authority_contact`,
 * so a kind that was cleared is dropped. */
export const authorityContactPatch = (contact: IAuthorityContact) => ({
    authority_contact: Object.fromEntries(
        CONTACT_KINDS.flatMap((kind) => {
            const value = contact[kind].trim()
            return value === "" ? [] : [[kind, value]]
        })
    ),
})

/** Whether two contacts say the same, as they would be stored. */
export const sameContact = (a: IAuthorityContact, b: IAuthorityContact): boolean =>
    CONTACT_KINDS.every((kind) => a[kind].trim() === b[kind].trim())
