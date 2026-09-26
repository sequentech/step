// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {getLanguageFromURL} from "./queryParams"

afterEach(() => window.history.replaceState({}, "", "/"))

it.each([
    ["?lang=fr-CA", "fr-CA"],
    ["?other=en", undefined],
    ["?lang=", undefined],
])("reads an explicit language from %s", (query, expected) => {
    window.history.replaceState({}, "", `/${query}`)
    expect(getLanguageFromURL()).toBe(expected)
})
