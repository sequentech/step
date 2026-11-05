// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {previewScope} from "./viewContext"

describe("previewScope", () => {
    const chosen = {region: "ncr", post: "tokyo", country: "jp"}

    it("keeps only the choices the dashboard offers", () => {
        expect(previewScope(chosen, {selectors: ["region", "country"]}, null)).toEqual({
            region: "ncr",
            country: "jp",
        })
    })

    it("leaves the Post and its Region out on an election's page, which pins them", () => {
        expect(
            previewScope(chosen, {selectors: ["region", "post", "country"]}, "election-1")
        ).toEqual({country: "jp"})
    })

    it("is the whole event when the dashboard is not known yet", () => {
        expect(previewScope(chosen, undefined, null)).toEqual({})
    })
})
