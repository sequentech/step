// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import cat from "./cat"
import en from "./en"
import es from "./es"
import eu from "./eu"
import fr from "./fr"
import gl from "./gl"
import nl from "./nl"
import tl from "./tl"
import {IPermissions} from "@/types/keycloak"

const languages = {cat, en, es, eu, fr, gl, nl, tl}

const leaves = (value: unknown, path = ""): Array<[string, unknown]> =>
    typeof value === "object" && value !== null
        ? Object.entries(value).flatMap(([key, child]) =>
              leaves(child, path ? `${path}.${key}` : key)
          )
        : [[path, value]]

describe("electoral-log console labels", () => {
    const english = leaves(en.translations.electoralLogConsole).map(([path]) => path)

    it.each(Object.entries(languages))("are complete in %s", (_, language) => {
        const {electoralLogConsole, sideMenu, usersAndRolesScreen} = language.translations
        const labels = leaves(electoralLogConsole)
        expect(labels.map(([path]) => path)).toEqual(english)
        for (const label of [
            ...labels.map(([, label]) => label),
            sideMenu.electoralLogConsole,
            usersAndRolesScreen.permissions[IPermissions.ELECTORAL_LOG_CONSOLE_READ],
            usersAndRolesScreen.permissions[IPermissions.ELECTORAL_LOG_CONSOLE_QUERY],
            usersAndRolesScreen.permissions[IPermissions.ELECTORAL_LOG_PERSONAL_DATA_READ],
        ]) {
            expect(typeof label).toBe("string")
            expect((label as string).trim()).not.toHaveLength(0)
        }
    })
})
