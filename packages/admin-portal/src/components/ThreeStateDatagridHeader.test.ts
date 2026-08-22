// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {resolveThreeStateSort} from "./threeStateSort"

describe("resolveThreeStateSort", () => {
    it("starts a newly selected column in ascending order", () => {
        expect(
            resolveThreeStateSort({field: "id", order: "ASC"}, {field: "username", order: "DESC"})
        ).toEqual({field: "username", order: "ASC"})
    })

    it("moves an ascending column to descending", () => {
        expect(
            resolveThreeStateSort(
                {field: "username", order: "ASC"},
                {field: "username", order: "DESC"}
            )
        ).toEqual({field: "username", order: "DESC"})
    })

    it("clears sorting after descending", () => {
        expect(
            resolveThreeStateSort(
                {field: "username", order: "DESC"},
                {field: "username", order: "ASC"}
            )
        ).toEqual({field: "", order: "ASC"})
    })
})
