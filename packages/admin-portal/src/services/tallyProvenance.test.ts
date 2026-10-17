// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {getTallyProvenance} from "./tallyProvenance"
import {ETallyProvenance, TALLY_PROVENANCE_ANNOTATION_KEY} from "@/types/ceremonies"

it("reads an imported tally session as imported", () => {
    expect(
        getTallyProvenance({
            executer_username: "admin",
            [TALLY_PROVENANCE_ANNOTATION_KEY]: ETallyProvenance.IMPORTED,
        })
    ).toBe(ETallyProvenance.IMPORTED)
})

it.each([undefined, null, {}, {executer_username: "admin"}, "IMPORTED", []])(
    "reads %p as native",
    (annotations) => {
        expect(getTallyProvenance(annotations)).toBe(ETallyProvenance.NATIVE)
    }
)

it("reads an explicit native value as native", () => {
    expect(getTallyProvenance({[TALLY_PROVENANCE_ANNOTATION_KEY]: ETallyProvenance.NATIVE})).toBe(
        ETallyProvenance.NATIVE
    )
})
