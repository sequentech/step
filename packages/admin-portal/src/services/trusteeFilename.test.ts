// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {trusteeFilename} from "./trusteeFilename"

it.each([
    ["trustee1", "trustee1-private-keys.toml"],
    ["trustee2", "trustee2-private-keys.toml"],
    ["Éva.Trustee", "Éva.Trustee-private-keys.toml"],
    ["alice@example.org", "alice-example.org-private-keys.toml"],
    ["../alice/$()", "alice-private-keys.toml"],
    [" Bob Smith ", "Bob-Smith-private-keys.toml"],
    ["", "trustee-private-keys.toml"],
])("names downloads for %j without path or shell syntax", (name, expected) => {
    expect(trusteeFilename(name, "private-keys.toml")).toBe(expected)
})
