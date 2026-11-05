// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {vitrineLanguage} from "./language"
import {ETransparencyMode, transparencyMode} from "./presentation"

describe("vitrineLanguage", () => {
    it.each([
        ["en", "en"],
        ["es", "es"],
        ["cat", "ca"],
        ["fr-FR", "fr"],
        ["gl", "gl"],
        ["tl", "tl"],
        ["de", "en"],
        ["", "en"],
        [undefined, "en"],
    ])("reads %s as the UI kit's %s", (appLanguage, kit) => {
        expect(vitrineLanguage(appLanguage)).toBe(kit)
    })
})

describe("transparencyMode", () => {
    it.each([
        [{transparency: "public"}, ETransparencyMode.PUBLIC],
        [{transparency: "private"}, ETransparencyMode.PRIVATE],
        [{transparency: "disabled"}, ETransparencyMode.DISABLED],
        [{transparency: "everything"}, ETransparencyMode.DISABLED],
        [{}, ETransparencyMode.DISABLED],
        [null, ETransparencyMode.DISABLED],
        ["public", ETransparencyMode.DISABLED],
    ])("reads %j as %s", (presentation, mode) => {
        expect(transparencyMode(presentation)).toBe(mode)
    })
})
