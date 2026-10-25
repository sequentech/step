// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {buildTenantConfigurationCsv, type TenantState} from "./ComelecTenantConfig"

// Minimal RFC4180 single-row parser, matching how the backend csv reader unquotes.
function parseCsvRow(row: string): string[] {
    const fields: string[] = []
    let current = ""
    let inQuotes = false
    for (let i = 0; i < row.length; i++) {
        const c = row[i]
        if (inQuotes) {
            if (c === '"' && row[i + 1] === '"') {
                current += '"'
                i++
            } else if (c === '"') {
                inQuotes = false
            } else {
                current += c
            }
        } else if (c === '"') {
            inQuotes = true
        } else if (c === ",") {
            fields.push(current)
            current = ""
        } else {
            current += c
        }
    }
    fields.push(current)
    return fields
}

describe("buildTenantConfigurationCsv", () => {
    const target: TenantState = {
        id: "t-1",
        slug: "comelec",
        created_at: "2027-01-01T00:00:00+00:00",
        updated_at: "2027-01-02T00:00:00+00:00",
        labels: null,
        annotations: {css: "header { }"},
        is_active: true,
        voting_channels: null,
        settings: {existing: "kept", gapi_key: {private_key: "TOP-SECRET"}},
        test: 7,
    }

    it("merges display_name, preserves id/slug and unrelated settings, strips the secret", () => {
        const csv = buildTenantConfigurationCsv(target, "COMELEC")
        const [header, row] = csv.trim().split("\n")
        expect(header).toBe(
            "id,slug,created_at,updated_at,labels,annotations,is_active,voting_channels,settings,test"
        )
        const fields = parseCsvRow(row)
        // slug is a bare string (the importer reads get(1) verbatim — not JSON-quoted).
        expect(fields[1]).toBe("comelec")
        expect(fields[0]).toBe("t-1")
        expect(fields[6]).toBe("true")
        expect(fields[9]).toBe("7") // `test` is column 9
        // Settings column is valid JSON; it gains display_name, keeps unrelated keys,
        // and the Google service-account key is gone.
        const settings = JSON.parse(fields[8])
        expect(settings.display_name).toBe("COMELEC")
        expect(settings.existing).toBe("kept")
        expect(settings.gapi_key).toBeUndefined()
        expect(csv).not.toContain("TOP-SECRET")
    })

    it("leaves the test column blank when the target has none", () => {
        const csv = buildTenantConfigurationCsv({...target, test: null}, "COMELEC")
        const fields = parseCsvRow(csv.trim().split("\n")[1])
        expect(fields[9]).toBe("")
    })
})
