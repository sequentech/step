// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {formatUserSortToJsonb} from "./GetUsers"

describe("formatUserSortToJsonb", () => {
    it.each([
        ["username", "username"],
        ["attributes['voted-channel']", "voted-channel"],
        ["attributes.authorized-election-ids", "authorized-election-ids"],
    ])("normalizes %s to the backend sort field %s", (source, field) => {
        expect(formatUserSortToJsonb({field: source, order: "DESC"})).toEqual({
            "'field'": field,
            "'order'": "DESC",
        })
    })
})
