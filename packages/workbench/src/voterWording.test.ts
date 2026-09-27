// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, test} from "vitest"
import {voterWordingKeys} from "./voterWording"

test("the wording keys are the portal's and ui-core's, dotted, sorted and once each", () => {
    const keys = voterWordingKeys()
    expect(keys).toContain("common.goBack")
    // Drawn by the ballot list and kept in ui-core's catalogue.
    expect(keys).toContain("selectElection.voteButton")
    // Keycloak's, not the portal's.
    expect(keys).not.toContain("doLogIn")
    expect(keys).toEqual([...new Set(keys)].sort())
    expect(keys.every((key) => /^[\w-]+(\.[\w-]+)*$/.test(key))).toBe(true)
})
