// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it, jest} from "@jest/globals"
import {sleep} from "./sleep"

it("resolves only after the requested delay without making the suite wait in real time", async () => {
    jest.useFakeTimers()
    try {
        const finished = jest.fn()
        const waiting = sleep(500).then(finished)
        await jest.advanceTimersByTimeAsync(499)
        expect(finished).not.toHaveBeenCalled()
        await jest.advanceTimersByTimeAsync(1)
        await waiting
        expect(finished).toHaveBeenCalledWith(undefined)
        expect(finished).toHaveBeenCalledTimes(1)
    } finally {
        jest.useRealTimers()
    }
})
