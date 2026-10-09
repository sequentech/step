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
const PLACEHOLDERS_ONLY = /^[\s:·\-{}a-z_]*$/
const SAME_IN: Record<string, string[]> = {
    es: ["messagingEvent.column.post"],
    ca: ["messagingEvent.column.post"],
    eu: ["messagingEvent.column.post"],
    fr: ["messagingEvent.column.post", "messagingAccounts.column.actions"],
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
    ],
    tl: [],
}

describe("messaging translations", () => {
    const english = new Map(
        SECTIONS.flatMap((section) =>
            leaves(en.translations[section] as unknown as Tree, `${section}.`)
        )
    )

    it.each(Object.entries({es, ca, eu, fr, gl, nl, tl}))(
        "%s translates every messaging text rather than copying the English",
        (language, translation) => {
            const copies = SECTIONS.flatMap((section) =>
                leaves(translation.translations[section] as unknown as Tree, `${section}.`)
            )
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

    it("names no client", () => {
        for (const translation of [en, es, ca, eu, fr, gl, nl, tl]) {
            expect(JSON.stringify(translation.translations.messaging)).not.toMatch(/COMELEC/i)
            expect(JSON.stringify(translation.translations.messagingAccounts)).not.toMatch(
                /COMELEC/i
            )
        }
    })
})
