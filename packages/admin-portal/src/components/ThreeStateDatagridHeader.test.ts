// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {resolveThreeStateSort} from "./threeStateSort"

describe("resolveThreeStateSort", () => {
    it("uses the requested sort for the ascending and descending states", () => {
        expect(
            resolveThreeStateSort({field: "id", order: "ASC"}, {field: "username", order: "ASC"})
        ).toEqual({field: "username", order: "ASC"})
        expect(
            resolveThreeStateSort(
                {field: "username", order: "ASC"},
                {field: "username", order: "DESC"}
            )
        ).toEqual({field: "username", order: "DESC"})
    })

    it("returns to the default sort after descending", () => {
        expect(
            resolveThreeStateSort(
                {field: "username", order: "DESC"},
                {field: "username", order: "ASC"}
            )
        ).toEqual({field: "id", order: "ASC"})
    })
})
