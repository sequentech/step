// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {DATAFIX_ID_ANNOTATION, isDatafixElectionEvent} from "./Datafix"

describe("isDatafixElectionEvent", () => {
    it("is true when the datafix:id annotation is present", () => {
        expect(isDatafixElectionEvent({annotations: {[DATAFIX_ID_ANNOTATION]: "abc-123"}})).toBe(
            true
        )
    })

    it("is true when the annotation is present alongside others", () => {
        expect(
            isDatafixElectionEvent({
                annotations: {
                    "datafix:password_policy": "strict",
                    [DATAFIX_ID_ANNOTATION]: "abc-123",
                },
            })
        ).toBe(true)
    })

    it("is true when the annotation is present but null, since only the key is checked", () => {
        expect(isDatafixElectionEvent({annotations: {[DATAFIX_ID_ANNOTATION]: null}})).toBe(true)
    })

    it("is false when annotations carry other keys only", () => {
        expect(isDatafixElectionEvent({annotations: {"datafix:voterview_request": "yes"}})).toBe(
            false
        )
    })

    it("is false for an empty annotations object", () => {
        expect(isDatafixElectionEvent({annotations: {}})).toBe(false)
    })

    it("is false for absent, null and undefined annotations", () => {
        expect(isDatafixElectionEvent({})).toBe(false)
        expect(isDatafixElectionEvent({annotations: null})).toBe(false)
        expect(isDatafixElectionEvent({annotations: undefined})).toBe(false)
    })

    it("is false for a missing election event, which is the tenant-level voters view", () => {
        expect(isDatafixElectionEvent(undefined)).toBe(false)
        expect(isDatafixElectionEvent(null)).toBe(false)
    })

    it("is false for malformed annotations that are not a plain object", () => {
        expect(isDatafixElectionEvent({annotations: "datafix:id"})).toBe(false)
        expect(isDatafixElectionEvent({annotations: 42})).toBe(false)
        expect(isDatafixElectionEvent({annotations: [DATAFIX_ID_ANNOTATION]})).toBe(false)
    })

    it("does not match a key that merely contains the annotation name", () => {
        expect(isDatafixElectionEvent({annotations: {"x-datafix:id": "abc"}})).toBe(false)
        expect(isDatafixElectionEvent({annotations: {"datafix:idx": "abc"}})).toBe(false)
    })
})
