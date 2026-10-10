// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import en from "./en"
import es from "./es"
import ca from "./cat"
import eu from "./eu"
import fr from "./fr"
import gl from "./gl"
import nl from "./nl"
import tl from "./tl"

const SECTIONS = ["messagingEvent", "messaging", "messagingAccounts"] as const
// Messaging texts outside the three sections.
const EXTRA_KEYS = ["usersAndRolesScreen.permissions.messaging-config-read"]

type Tree = {[key: string]: string | Tree}

const leaves = (tree: Tree, prefix = ""): Array<[string, string]> =>
    Object.entries(tree).flatMap(([key, value]) =>
        typeof value === "string"
            ? [[`${prefix}${key}`, value] as [string, string]]
            : leaves(value, `${prefix}${key}.`)
    )

// The same in every language: brand and product names, placeholders only, and
// words that are the same in a language ("Status" in Dutch, "Post" for a Post).
const SAME_EVERYWHERE =
    /^(messaging\.channel\.|messaging\.provider\.(AWS_SES|AWS_SNS|WHATSAPP_CLOUD_API|MESSENGER_SEND_API|VIBER_INFOBIP)$)/
// Only placeholders and punctuation: "{{channel}}: {{purpose}}", never words.
const PLACEHOLDERS_ONLY = /^(\s|[:·-]|\{\{\w+\}\})*$/
const SAME_IN: Record<string, string[]> = {
    es: [
        "messagingEvent.column.post",
        "messagingAccounts.status.no",
        "messagingAccounts.status.templateCategory.MARKETING",
    ],
    ca: ["messagingEvent.column.post", "messagingAccounts.status.no"],
    eu: ["messagingEvent.column.post"],
    fr: [
        "messagingEvent.column.post",
        "messagingAccounts.column.actions",
        "messagingAccounts.status.templateCategory.MARKETING",
    ],
    gl: ["messagingEvent.column.post"],
    nl: [
        "messagingEvent.column.status",
        "messagingEvent.column.post",
        "messaging.templates.parameter",
        "messaging.templates.account",
        "messaging.logs.status",
        "messagingAccounts.column.name",
        "messagingAccounts.column.provider",
        "messagingAccounts.editor.provider",
        "messagingAccounts.status.templateCategory.MARKETING",
    ],
    // "Channel" and "Post" are kept as loanwords, as in the rest of the Tagalog texts.
    tl: [
        "messagingEvent.column.channel",
        "messagingEvent.column.post",
        "messaging.send.channelColumn",
        "messaging.voter.channel",
        "messaging.logs.channel",
        "messagingAccounts.column.channel",
        "messagingAccounts.editor.channel",
        "messagingAccounts.status.templateCategory.MARKETING",
    ],
}

const at = (tree: Tree, path: string): string | undefined => {
    const value = path
        .split(".")
        .reduce<
            string | Tree | undefined
        >((node, key) => (typeof node === "object" ? node[key] : undefined), tree)
    return typeof value === "string" ? value : undefined
}

const messagingTexts = (translations: Tree): Array<[string, string]> => [
    ...SECTIONS.flatMap((section) => leaves(translations[section] as Tree, `${section}.`)),
    ...EXTRA_KEYS.flatMap((key): Array<[string, string]> => {
        const value = at(translations, key)
        return value === undefined ? [] : [[key, value]]
    }),
]

const variables = (text: string): string[] =>
    Array.from(text.matchAll(/\{\{\s*(\w+)\s*\}\}/g), (match) => match[1]).sort()

describe("messaging translations", () => {
    const english = new Map(messagingTexts(en.translations as unknown as Tree))

    it("has the permission label in English", () => {
        expect(english.get(EXTRA_KEYS[0])).toBeTruthy()
    })

    it.each(Object.entries({es, ca, eu, fr, gl, nl, tl}))(
        "%s translates every messaging text rather than copying the English",
        (language, translation) => {
            const copies = messagingTexts(translation.translations as unknown as Tree)
                .filter(([key, value]) => english.get(key) === value)
                .map(([key]) => key)
                .filter(
                    (key) =>
                        !SAME_EVERYWHERE.test(key) &&
                        !PLACEHOLDERS_ONLY.test(english.get(key) ?? "") &&
                        !SAME_IN[language].includes(key)
                )
            expect(copies).toEqual([])
        }
    )

    it.each(Object.entries({es, ca, eu, fr, gl, nl, tl}))(
        "%s keeps every text's {{variables}}, and has every text",
        (_language, translation) => {
            const texts = new Map(messagingTexts(translation.translations as unknown as Tree))
            const mismatched = Array.from(english.entries())
                .filter(
                    ([key, value]) =>
                        JSON.stringify(variables(texts.get(key) ?? "")) !==
                        JSON.stringify(variables(value))
                )
                .map(([key]) => key)
            expect(mismatched).toEqual([])
            expect(Array.from(english.keys()).filter((key) => !texts.has(key))).toEqual([])
        }
    )

    // FX3 follow-up (R7-10): the reconnect at sign-in needs the voter's password; without one,
    // staff change the voter's channel. Every language names the password.
    it.each(
        Object.entries({
            en: [en, "password"],
            es: [es, "contraseña"],
            ca: [ca, "contrasenya"],
            eu: [eu, "pasahitz"],
            fr: [fr, "mot de passe"],
            gl: [gl, "contrasinal"],
            nl: [nl, "wachtwoord"],
            tl: [tl, "password"],
        } as const)
    )("%s: the Page-change warning names both recovery paths", (_language, [translation, word]) => {
        const warning = new Map(messagingTexts(translation.translations as unknown as Tree)).get(
            "messagingAccounts.warning.pageChange"
        )
        expect(warning?.toLowerCase()).toContain(word)
        expect(warning).toContain("{{page}}")
    })

    it("names no client", () => {
        for (const translation of [en, es, ca, eu, fr, gl, nl, tl]) {
            const texts = messagingTexts(translation.translations as unknown as Tree)
            expect(JSON.stringify(texts)).not.toMatch(/COMELEC/i)
        }
    })
})
