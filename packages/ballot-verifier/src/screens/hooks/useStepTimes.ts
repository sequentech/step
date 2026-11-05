// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// When this page first saw each step of a receipt done, by the device's
// clock. The page checks the receipt in rounds: the log's steps are done in
// the first, the anchors in a later one.

import {useEffect, useRef, useState} from "react"
import {EStatusStep, EStepState, STATUS_STEPS} from "../../services/vitrine/ballotStatus"

export type TStepTimes = Partial<Record<EStatusStep, number>>

interface ISeen {
    /** What the times are for: they start over when it changes. */
    key: unknown
    times: TStepTimes
}

/** POSIX seconds, for each step in `steps` that is done. A step that stops
 * being done is forgotten. */
export const useStepTimes = (
    steps: Record<EStatusStep, EStepState> | undefined,
    now: () => number,
    key: unknown
): TStepTimes => {
    const [seen, setSeen] = useState<ISeen>({key, times: {}})
    const clock = useRef(now)
    clock.current = now
    const doneSteps = STATUS_STEPS.filter((step) => steps?.[step] === EStepState.DONE).join()

    useEffect(() => {
        setSeen((current) => {
            const times: TStepTimes = current.key === key ? {...current.times} : {}
            let changed = current.key !== key
            for (const step of STATUS_STEPS) {
                const done = doneSteps.split(",").includes(step)
                if (done && times[step] === undefined) {
                    times[step] = clock.current()
                    changed = true
                } else if (!done && times[step] !== undefined) {
                    delete times[step]
                    changed = true
                }
            }
            return changed ? {key, times} : current
        })
    }, [doneSteps, key])

    return seen.key === key ? seen.times : {}
}
