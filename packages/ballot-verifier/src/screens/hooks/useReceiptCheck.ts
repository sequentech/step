// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Runs the receipt screen's rounds (services/vitrine/check.ts): the log-only
// round as soon as the receipt, the pinned policy and the expectation are
// known, then the full round at a random moment, at once when the result
// can't wait, or when the user asks. A receipt reference is first rebuilt
// into a chain from the log.

import {useCallback, useEffect, useRef, useState} from "react"
import type {Expect, PolicyHistory, ReceiptReference} from "@vitrine/verify"
import {
    chainFromReference,
    checkRound,
    ECheckRound,
    fullRoundDelay,
    ICheckOutcome,
    isPrivate,
    needsFullRoundNow,
} from "../../services/vitrine/check"
import type {FetchLike} from "../../services/vitrine/http"
import type {IPolicyView} from "../../services/vitrine/policies"
import type {IVitrineSdk} from "../../services/vitrine/sdk"

export enum ECheckProblem {
    /** The reference is for another policy than the pinned one. */
    POLICY_MISMATCH = "policy-mismatch",
    UNKNOWN_LOG = "unknown-log",
    /** The log doesn't serve proofs during its quiet period. */
    QUIET_PERIOD = "quiet-period",
    /** The log has no such entry, or not yet. */
    NOT_FOUND = "not-found",
    /** The log's entry at the reference's index is another one. */
    ENTRY_MISMATCH = "entry-mismatch",
    /** A private event's log serves no one, so a reference can't be rebuilt. */
    PRIVATE_REFERENCE = "private-reference",
    FAILED = "failed",
}

export interface ICheckProblem {
    problem: ECheckProblem
    /** For a quiet period: when it ends, POSIX seconds. */
    quietUntil?: number
}

export interface IReceiptCheckDeps {
    fetch?: FetchLike
    /** The device clock, POSIX seconds. */
    now?: () => number
    /** Random numbers in [0, 1), for the full round's moment. */
    random?: () => number
    /** Runs `run` after `ms` milliseconds; returns what cancels it. */
    schedule?: (run: () => void, ms: number) => () => void
}

export type TCheckSource = {chain: string} | {reference: ReceiptReference}

export interface IReceiptCheckInput {
    source: TCheckSource
    view: IPolicyView
    expect: Expect
    history: PolicyHistory
}

export interface IReceiptCheck {
    outcome?: ICheckOutcome
    /** The round that is running. */
    running?: ECheckRound.LOG | ECheckRound.FULL
    problem?: ICheckProblem
    /** True while the full round waits for its moment. */
    fullDue: boolean
    /** Runs the full round now. */
    checkEverything: () => void
    /** Runs the last kind of round again. */
    retry: () => void
}

interface IState {
    outcome?: ICheckOutcome
    running?: ECheckRound.LOG | ECheckRound.FULL
    problem?: ICheckProblem
    fullDue: boolean
}

const defaultSchedule = (run: () => void, ms: number): (() => void) => {
    const id = setTimeout(run, ms)
    return () => clearTimeout(id)
}

const UPGRADE_PROBLEMS: Readonly<Record<string, ECheckProblem>> = {
    "policy-mismatch": ECheckProblem.POLICY_MISMATCH,
    "unknown-log": ECheckProblem.UNKNOWN_LOG,
    "quiet-period": ECheckProblem.QUIET_PERIOD,
    "not-found": ECheckProblem.NOT_FOUND,
    "entry-mismatch": ECheckProblem.ENTRY_MISMATCH,
}

const problemOf = (sdk: IVitrineSdk, e: unknown): ICheckProblem => {
    if (e instanceof sdk.verify.UpgradeError) {
        const problem = UPGRADE_PROBLEMS[e.code] ?? ECheckProblem.FAILED
        return problem === ECheckProblem.QUIET_PERIOD && e.quietUntil !== undefined
            ? {problem, quietUntil: e.quietUntil}
            : {problem}
    }
    return {problem: ECheckProblem.FAILED}
}

export const useReceiptCheck = (
    sdk: IVitrineSdk | undefined,
    input: IReceiptCheckInput | undefined,
    deps: IReceiptCheckDeps = {}
): IReceiptCheck => {
    const [state, setState] = useState<IState>({fullDue: false})
    const depsRef = useRef(deps)
    depsRef.current = deps
    // The chain as far as the rounds upgraded it.
    const chain = useRef<string | undefined>(undefined)
    const generation = useRef(0)
    const abort = useRef<AbortController | undefined>(undefined)
    const cancelFull = useRef<(() => void) | undefined>(undefined)
    const lastRound = useRef<ECheckRound.LOG | ECheckRound.FULL>(ECheckRound.LOG)

    const stop = useCallback(() => {
        generation.current++
        abort.current?.abort()
        abort.current = undefined
        cancelFull.current?.()
        cancelFull.current = undefined
    }, [])

    const run = useCallback(
        async (round: ECheckRound.LOG | ECheckRound.FULL): Promise<void> => {
            if (!sdk || !input) return
            stop()
            const gen = generation.current
            const controller = new AbortController()
            abort.current = controller
            lastRound.current = round
            const {fetch, now, random = Math.random, schedule = defaultSchedule} = depsRef.current
            const opts = {
                ...(fetch ? {fetch} : {}),
                ...(now ? {now} : {}),
                signal: controller.signal,
            }
            setState((s) => ({...s, running: round, problem: undefined, fullDue: false}))
            try {
                let current = chain.current
                if (current === undefined) {
                    if ("chain" in input.source) {
                        current = input.source.chain
                    } else if (isPrivate(input.view)) {
                        setState((s) => ({
                            ...s,
                            running: undefined,
                            problem: {problem: ECheckProblem.PRIVATE_REFERENCE},
                        }))
                        return
                    } else {
                        current = await chainFromReference(
                            sdk,
                            input.source.reference,
                            input.view,
                            input.history,
                            opts
                        )
                    }
                    if (gen !== generation.current) return
                    chain.current = current
                }
                const outcome = await checkRound(
                    sdk,
                    {
                        chain: current,
                        view: input.view,
                        expect: input.expect,
                        history: input.history,
                    },
                    round,
                    opts
                )
                if (gen !== generation.current) return
                chain.current = outcome.chain
                const fullDue = outcome.round === ECheckRound.LOG
                setState({outcome, fullDue})
                if (!fullDue) return
                if (needsFullRoundNow(outcome.result)) {
                    void run(ECheckRound.FULL)
                } else {
                    cancelFull.current = schedule(
                        () => void run(ECheckRound.FULL),
                        fullRoundDelay(random)
                    )
                }
            } catch (e) {
                if (gen !== generation.current) return
                setState((s) => ({...s, running: undefined, problem: problemOf(sdk, e)}))
            }
        },
        [sdk, input, stop]
    )

    useEffect(() => {
        chain.current = undefined
        setState({fullDue: false})
        void run(ECheckRound.LOG)
        return stop
    }, [run, stop])

    return {
        ...state,
        checkEverything: () => void run(ECheckRound.FULL),
        retry: () => void run(lastRound.current),
    }
}
