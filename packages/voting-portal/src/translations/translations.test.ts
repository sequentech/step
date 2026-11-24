// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import i18next from "i18next"
import {votingPortalTranslations} from "./index"

type Tree = {[key: string]: string | Tree}

// Admin portal strings kept in this catalogue for type parity; the voting
// portal never renders them.
const ADMIN_ONLY = ["settings", "forensic"]

// Words a language legitimately shares with English.
const SAME_AS_ENGLISH: Record<string, string[]> = {
    cat: ["helpSections.signatures.title"],
    fr: ["helpSections.signatures.title"],
    nl: ["channels.online", "channels.kiosk"],
    // Tagalog keeps "Ballot ID", as the rest of its catalogue does.
    tl: ["ballotId", "checkLabels.ballotId", "channels.online", "channels.kiosk"],
}

const leaves = (tree: Tree, prefix = ""): Array<[string, string]> =>
    Object.entries(tree).flatMap(([key, value]) =>
        typeof value === "string"
            ? [[`${prefix}${key}`, value] as [string, string]]
            : leaves(value, `${prefix}${key}.`)
    )

const catalogue = (language: string) =>
    (votingPortalTranslations[language] as {translations: Tree}).translations

const english = catalogue("en")
const languages = Object.keys(votingPortalTranslations).filter((language) => language !== "en")

it.each([
    [1, "1 ballot"],
    [2, "2 ballots"],
])("counts %i ballot box entries in the singular or plural", async (count, expected) => {
    const i18n = i18next.createInstance()
    await i18n.init({
        lng: "en",
        resources: {en: votingPortalTranslations.en},
        ns: ["translations"],
        defaultNS: "translations",
    })
    expect(i18n.t("ballotPrivacy.ballots", {count})).toBe(expected)
})

describe.each(languages)("%s voting portal catalogue", (language) => {
    const translated = catalogue(language)

    it("translates the ballot box log", () => {
        const allowed = SAME_AS_ENGLISH[language] ?? []
        const ballotPrivacy = translated.ballotPrivacy as Tree
        const untranslated = leaves(english.ballotPrivacy as Tree)
            .filter(([key]) => !ADMIN_ONLY.includes(key.split(".")[0]))
            .filter(([key, value]) => {
                const local = leaves(ballotPrivacy).find(([localKey]) => localKey === key)
                return local?.[1] === value && !allowed.includes(key)
            })
            .map(([key]) => key)
        expect(untranslated).toEqual([])
    })

    it("translates the ballot locator's Logs tab", () => {
        const tabs = (translated.ballotLocator as Tree).tabs as Tree
        const englishTabs = (english.ballotLocator as Tree).tabs as Tree
        expect(tabs.logs).not.toEqual(englishTabs.logs)
    })

    it("keeps every interpolation placeholder of the ballot box log", () => {
        const placeholders = (text: string) => (text.match(/{{\w+}}/g) ?? []).sort()
        const local = new Map(leaves(translated.ballotPrivacy as Tree))
        for (const [key, value] of leaves(english.ballotPrivacy as Tree)) {
            expect([key, placeholders(local.get(key) ?? "")]).toEqual([key, placeholders(value)])
        }
    })
})
