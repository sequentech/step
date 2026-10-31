// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {collectForensicExport} from "./ForensicExport"

it("fetches bounded pages and exports only explicitly selected addresses", async () => {
    const rows = Array.from({length: 205}, (_, index) => ({id: String(index)}))
    const fetchPage = jest.fn(async (offset: number, limit: number) => ({
        items: rows.slice(offset, offset + limit),
        total: rows.length,
    }))
    expect(await collectForensicExport(fetchPage, ["2", "104", "204"])).toEqual([
        {id: "2"},
        {id: "104"},
        {id: "204"},
    ])
    expect(fetchPage.mock.calls).toEqual([
        [0, 100],
        [100, 100],
        [200, 100],
    ])
})

it("does not download a partial result when a later page fails", async () => {
    const fetchPage = jest
        .fn()
        .mockResolvedValueOnce({items: [{id: "first"}], total: 2})
        .mockRejectedValueOnce(new Error("unavailable"))
    await expect(collectForensicExport(fetchPage)).rejects.toThrow("unavailable")
})
