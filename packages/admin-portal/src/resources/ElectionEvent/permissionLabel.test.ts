// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {withSubmittedPermissionLabel} from "./permissionLabel"

describe("the election event's permission label on submit", () => {
    it.each([
        [" north ", "north"],
        ["north", "north"],
        ["", null],
        ["   ", null],
        [null, null],
        [undefined, null],
    ])("an editor submits %p as %p", (typed, stored) => {
        expect(
            withSubmittedPermissionLabel({name: "event", permission_label: typed}, true)
        ).toEqual({name: "event", permission_label: stored})
    })

    it("stores no label when the form never held one", () => {
        const unlabelled: {name: string; permission_label?: string | null} = {name: "event"}
        expect(withSubmittedPermissionLabel(unlabelled, true)).toEqual({
            name: "event",
            permission_label: null,
        })
    })

    it("leaves the label out for a user who cannot edit it", () => {
        const submitted = withSubmittedPermissionLabel(
            {name: "event", permission_label: "north"},
            false
        )
        expect(submitted).toEqual({name: "event"})
        expect("permission_label" in submitted).toBe(false)
    })
})
