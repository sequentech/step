// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {renderHook} from "@testing-library/react"
import {EStatusStep, EStepState} from "../../services/vitrine/ballotStatus"
import {useStepTimes} from "./useStepTimes"

const steps = (
    record: EStepState,
    witnesses: EStepState,
    anchors: EStepState
): Record<EStatusStep, EStepState> => ({record, witnesses, anchors})

describe("useStepTimes", () => {
    it("times a step when it is first seen done, and keeps that time", () => {
        let clock = 100
        const now = () => clock
        const {result, rerender} = renderHook(
            ({current}) => useStepTimes(current, now, "receipt"),
            {
                initialProps: {
                    current: steps(EStepState.ACTIVE, EStepState.WAITING, EStepState.WAITING),
                },
            }
        )
        expect(result.current).toEqual({})

        clock = 110
        rerender({current: steps(EStepState.DONE, EStepState.DONE, EStepState.ACTIVE)})
        expect(result.current).toEqual({record: 110, witnesses: 110})

        clock = 200
        rerender({current: steps(EStepState.DONE, EStepState.DONE, EStepState.DONE)})
        expect(result.current).toEqual({record: 110, witnesses: 110, anchors: 200})
    })

    it("forgets a step that is no longer done", () => {
        const {result, rerender} = renderHook(
            ({current}) => useStepTimes(current, () => 5, "receipt"),
            {
                initialProps: {
                    current: steps(EStepState.DONE, EStepState.ACTIVE, EStepState.WAITING),
                },
            }
        )
        expect(result.current).toEqual({record: 5})
        rerender({current: steps(EStepState.ACTIVE, EStepState.WAITING, EStepState.WAITING)})
        expect(result.current).toEqual({})
    })

    it("starts over for another receipt", () => {
        let clock = 1
        const done = steps(EStepState.DONE, EStepState.DONE, EStepState.DONE)
        const {result, rerender} = renderHook(({key}) => useStepTimes(done, () => clock, key), {
            initialProps: {key: "one"},
        })
        expect(result.current.record).toBe(1)
        clock = 9
        rerender({key: "two"})
        expect(result.current).toEqual({record: 9, witnesses: 9, anchors: 9})
    })

    it("has no times without steps", () => {
        const {result} = renderHook(() => useStepTimes(undefined, () => 1, "receipt"))
        expect(result.current).toEqual({})
    })
})
