// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    EInitializationScope as Scope,
    EUnsignedScheduledClosePolicy as Close,
} from "../../../../ui-core/src/types/ElectionEventPresentation"
import {
    EPolicyChange,
    closePolicyChange,
    policiesOf,
    ruleChange,
    scopeChange,
} from "./lifecyclePolicyChange"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation"),
}))

describe("policy changes", () => {
    it.each([
        [Scope.POST, Scope.EVENT, EPolicyChange.TIGHTENS],
        [Scope.POST, Scope.POST_AND_COUNTRY, EPolicyChange.TIGHTENS],
        [Scope.EVENT, Scope.POST, EPolicyChange.LOOSENS],
        [Scope.POST_AND_COUNTRY, Scope.POST, EPolicyChange.LOOSENS],
        // Neither scope includes the other: the new one applies now, the old one
        // stays until the next approved publication.
        [Scope.EVENT, Scope.POST_AND_COUNTRY, EPolicyChange.MIXED],
        [Scope.POST_AND_COUNTRY, Scope.EVENT, EPolicyChange.MIXED],
        [Scope.EVENT, Scope.EVENT, EPolicyChange.NONE],
    ])("initialization scope %s → %s %s", (before, after, change) => {
        expect(scopeChange(before, after)).toBe(change)
    })

    it("a close that runs without signatures loosens; refusing it again tightens", () => {
        expect(closePolicyChange(Close.REFUSE, Close.RUN_AS_SYSTEM)).toBe(EPolicyChange.LOOSENS)
        expect(closePolicyChange(Close.RUN_AS_SYSTEM, Close.REFUSE)).toBe(EPolicyChange.TIGHTENS)
    })

    it("reads missing policies as the defaults: per election, refuse", () => {
        expect(policiesOf(undefined)).toEqual({
            initialization_scope: Scope.POST,
            unsigned_scheduled_close: Close.REFUSE,
        })
    })
})

describe("ruleChange", () => {
    it.each([
        [{required: false}, {required: true, signatures: 2}, EPolicyChange.TIGHTENS],
        [{required: true, signatures: 2}, {required: false}, EPolicyChange.LOOSENS],
        [{required: true, signatures: 2}, {required: true, signatures: 3}, EPolicyChange.TIGHTENS],
        [{required: true, signatures: 3}, {required: true, signatures: 2}, EPolicyChange.LOOSENS],
        [{required: true, signatures: 2}, {required: true, signatures: 2}, EPolicyChange.NONE],
        // Without signatures the number doesn't matter.
        [{required: false, signatures: 2}, {required: false, signatures: 3}, EPolicyChange.NONE],
    ])("%o → %o %s", (before, after, change) => {
        expect(ruleChange(before, after)).toBe(change)
    })
})
