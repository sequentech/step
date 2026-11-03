// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {STEP_BACK_PARAM, stepBack} from "./stepBack"

const label = {text: "Back", lang: "en"}
const url = {loginAction: "https://login.example/login-actions/registration?execution=e"}

describe("stepBack", () => {
    it("posts the Back parameter to the page's action when the server offers it", () => {
        expect(stepBack({url, stepBack: true}, label)).toEqual({
            label,
            action: url.loginAction,
            fields: {[STEP_BACK_PARAM]: "true"},
        })
    })

    it("offers nothing when the server doesn't", () => {
        expect(stepBack({url, stepBack: false}, label)).toBeUndefined()
        expect(stepBack({url}, label)).toBeUndefined()
    })
})
