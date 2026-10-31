// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import i18next from "i18next"
import {createElement} from "react"
import {renderToStaticMarkup} from "react-dom/server"
import englishTranslation from "@/translations/en"
import spanishTranslation from "@/translations/es"
import frenchTranslation from "@/translations/fr"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {getMenuActionConfirmMessage} from "./menuActionConfirmMessage"

let mockLanguage = "en"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({i18n: {language: mockLanguage}}),
}))

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../../../ui-core/src/services/translate")
)

const i18n = i18next.createInstance()

beforeAll(async () => {
    await i18n.init({
        resources: {en: englishTranslation, es: spanishTranslation, fr: frenchTranslation},
        lng: "en",
        fallbackLng: "en",
        ns: ["translations"],
        defaultNS: "translations",
        interpolation: {escapeValue: false},
    })
})

const message = (
    language: string,
    action: Parameters<typeof getMenuActionConfirmMessage>[1],
    item: unknown
) => {
    mockLanguage = language

    return getMenuActionConfirmMessage(
        i18n.getFixedT(language),
        action,
        // useAliasRenderer only reads the mocked useTranslation, so it runs outside a component.
        // eslint-disable-next-line react-hooks/rules-of-hooks
        useAliasRenderer()(item)
    )
}

const election = {
    presentation: {
        i18n: {
            en: {name: "General Election", alias: "Board 2026"},
            es: {name: "Elección General"},
        },
    },
}

describe("getMenuActionConfirmMessage", () => {
    it("names the item by its alias", () => {
        expect(message("en", "delete", election)).toBe(
            'Are you sure you want to delete "Board 2026"?'
        )
        expect(message("en", "archive", election)).toBe(
            'Are you sure you want to archive "Board 2026"?'
        )
        expect(message("en", "unarchive", election)).toBe(
            'Are you sure you want to unarchive "Board 2026"?'
        )
    })

    it("names the item in the admin's language", () => {
        expect(message("es", "delete", election)).toBe(
            '¿Está seguro de que desea eliminar "Elección General"?'
        )
    })

    it("falls back to the English alias when the admin's language has none", () => {
        expect(message("fr", "delete", election)).toBe(
            'Êtes-vous sûr de vouloir supprimer "Board 2026" ?'
        )
    })

    it("keeps the generic message when the item has no alias or name", () => {
        expect(message("en", "delete", {presentation: {i18n: {en: {}}}})).toBe(
            "Are you sure to delete this item?"
        )
        expect(message("es", "archive", null)).toBe(
            "¿Está seguro de que desea archivar este elemento?"
        )
        expect(getMenuActionConfirmMessage(i18n.getFixedT("en"), "delete", "  ")).toBe(
            "Are you sure to delete this item?"
        )
    })

    it("keeps markup in a name as plain text", () => {
        const markup = {presentation: {i18n: {en: {alias: '<b>Board</b> & "Co"'}}}}

        expect(message("en", "delete", markup)).toBe(
            'Are you sure you want to delete "<b>Board</b> & "Co""?'
        )
    })

    it("renders markup in a name as escaped text", () => {
        const markup = {presentation: {i18n: {en: {alias: '<b>Board</b> & "Co"'}}}}

        expect(
            renderToStaticMarkup(createElement("p", null, message("en", "delete", markup)))
        ).toBe(
            "<p>Are you sure you want to delete &quot;&lt;b&gt;Board&lt;/b&gt; &amp; &quot;Co&quot;&quot;?</p>"
        )
    })
})
