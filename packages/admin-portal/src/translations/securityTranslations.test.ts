// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import en from "./en"
import cat from "./cat"
import es from "./es"
import eu from "./eu"
import fr from "./fr"
import gl from "./gl"
import nl from "./nl"
import tl from "./tl"

it.each(Object.entries({cat, es, eu, fr, gl, nl, tl}))(
    "%s names every protected action and its permission",
    (_language, locale) => {
        expect(Object.keys(locale.translations.signing.actions).sort()).toEqual(
            Object.keys(en.translations.signing.actions).sort()
        )
        const permissions = locale.translations.usersAndRolesScreen.permissions
        for (const key of Object.keys(en.translations.usersAndRolesScreen.permissions)) {
            expect(permissions[key as keyof typeof permissions]).toEqual(expect.any(String))
        }
        expect(locale.translations.securityTests).toEqual(
            expect.objectContaining({
                title: expect.any(String),
                readFailed: expect.any(String),
            })
        )
    }
)
