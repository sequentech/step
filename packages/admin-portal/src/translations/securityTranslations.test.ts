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
        expect(Object.keys(locale.translations.attackDetection).sort()).toEqual(
            Object.keys(en.translations.attackDetection).sort()
        )
        expect(locale.translations.attackDetection.accountFailures).not.toEqual(
            en.translations.attackDetection.accountFailures
        )
        expect(locale.translations.copyableFingerprint).toEqual({
            copy: expect.any(String),
            copied: expect.any(String),
            copyError: expect.any(String),
        })
        expect(locale.translations.copyableFingerprint.copy).not.toEqual(
            en.translations.copyableFingerprint.copy
        )
        expect(Object.keys(locale.translations.tsfIntegrity).sort()).toEqual(
            Object.keys(en.translations.tsfIntegrity).sort()
        )
        expect(locale.translations.tsfIntegrity.run).not.toEqual(en.translations.tsfIntegrity.run)
        for (const group of ["outcome", "partOutcome"] as const) {
            expect(locale.translations.tsfIntegrity[group]).toEqual({
                verified: expect.any(String),
                different: expect.any(String),
                unverifiable: expect.any(String),
            })
            expect(locale.translations.tsfIntegrity[group].different).not.toEqual(
                en.translations.tsfIntegrity[group].different
            )
        }
        const revoting = locale.translations.electionRevoting
        expect(Object.keys(revoting).sort()).toEqual(
            Object.keys(en.translations.electionRevoting).sort()
        )
        expect(revoting.disable).not.toEqual(en.translations.electionRevoting.disable)
        const placeholders = (text: string) => (text.match(/{{\w+}}/g) ?? []).sort()
        for (const key of Object.keys(revoting) as Array<keyof typeof revoting>) {
            expect(placeholders(revoting[key])).toEqual(
                placeholders(en.translations.electionRevoting[key])
            )
        }
    }
)
