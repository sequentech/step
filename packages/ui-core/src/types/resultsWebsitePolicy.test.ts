// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {defaultResultsWebsitePolicy, parseResultsWebsitePolicy} from "./ElectionEventPresentation"

it("defaults publication to disabled and parses both object and JSON representations", () => {
    const defaults = defaultResultsWebsitePolicy()
    expect(defaults).toEqual({status: "disabled", access: "public", visibility_scope: "full_event"})
    for (const status of ["enabled", "disabled"]) {
        for (const access of ["public", "authenticated"]) {
            for (const visibility_scope of ["full_event", "area_based"]) {
                const policy = {status, access, visibility_scope}
                expect(parseResultsWebsitePolicy(policy)).toEqual(policy)
                expect(parseResultsWebsitePolicy(JSON.stringify(policy))).toEqual(policy)
            }
        }
    }
})

it.each([
    undefined,
    null,
    [],
    true,
    1,
    "{broken",
    "[]",
    {},
    {status: "unknown", access: "public", visibility_scope: "full_event"},
    {status: "enabled", access: "unknown", visibility_scope: "full_event"},
    {status: "enabled", access: "public", visibility_scope: "unknown"},
])("rejects malformed publication policy %# instead of enabling a guessed policy", (value) => {
    expect(parseResultsWebsitePolicy(value)).toBeUndefined()
})
