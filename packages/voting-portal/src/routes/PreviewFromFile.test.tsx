// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What a preview file must hold before it is hydrated.
 *
 * `updateBallotStyleAndSelection` dispatches the event first and then iterates
 * `documents`, `elections`, `support_materials` and `ballot_styles`. A file
 * missing one of those lists used to pass the check, load the event into the
 * store, and then fail with "… is not iterable", leaving half an event behind.
 */

import {rejectPreviewDocument} from "./PreviewFromFile"

jest.mock("@sequentech/ui-essentials", () => ({PageLimit: () => null, theme: {}}))

const valid = () => ({
    ballot_styles: [{area_id: "area-1", election_id: "election-1"}],
    election_event: {id: "event-1"},
    elections: [],
    documents: [],
    support_materials: [],
})

describe("a preview file", () => {
    it("is accepted with its event and every list", () => {
        expect(rejectPreviewDocument(valid())).toBeNull()
    })

    it.each(["elections", "documents", "support_materials"] as const)(
        "is refused, with the list named, when %s is missing or not a list",
        (key) => {
            const missing: Record<string, unknown> = valid()
            delete missing[key]
            expect(rejectPreviewDocument(missing)).toContain(`\`${key}\``)
            expect(rejectPreviewDocument({...valid(), [key]: null})).toContain(`\`${key}\``)
        }
    )

    it("is refused when it is not an object, has no ballots or no event", () => {
        expect(rejectPreviewDocument(null)).not.toBeNull()
        expect(rejectPreviewDocument({...valid(), ballot_styles: []})).not.toBeNull()
        expect(rejectPreviewDocument({...valid(), election_event: null})).not.toBeNull()
    })
})
