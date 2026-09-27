// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Mounting a contest from this package, with the ports a host supplies.
 *
 * The portal's `voting-portal/src/testing/ballotHarness.tsx` does the same through
 * its redux store; this one has no store and no router, only the two ports the
 * ballot actually reads: a `BallotSelectionPort` over a mutable record and a
 * `BallotEngine` answering the five WebAssembly questions deterministically. If the
 * components ever reach past those ports again, these tests are where it shows.
 */

import {ThemeProvider} from "@mui/material/styles"
import {render, type RenderResult} from "@testing-library/react"
import type {BallotSelection, ICandidate, IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import i18next from "i18next"
import React from "react"
import {I18nextProvider} from "react-i18next"

import {BallotEngineProvider, type BallotEngine} from "../ballot/engine"
import {InvalidErrorsList} from "../ballot/InvalidErrorsList"
import {Question} from "../ballot/Question"
import {BallotSelectionProvider, type BallotSelectionPort} from "../ballot/selection"
import type {IBallotStyle} from "../ballot/types"
import {theme} from "../services/theme"
import essentials from "../translations/en"

export const ELECTION_ID = "election-1"
export const CONTEST_ID = "contest-1"

/**
 * This package's English plus the host strings the ballot reads.
 *
 * `candidatesList.*`, `reviewScreen.*` and `a11y.*` are resolved by the ballot but
 * shipped by its host (the portal's catalogue); the few assertions on them use the
 * portal's wording.
 */
const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {
        en: {
            translation: {
                ...essentials.translations,
                a11y: {
                    selectUpTo_one: "Select up to {{count}} option",
                    selectUpTo_other: "Select up to {{count}} options",
                    selectExactly_one: "Select {{count}} option",
                    selectExactly_other: "Select {{count}} options",
                    selectBetween: "Select between {{min}} and {{max}} options",
                },
                candidatesList: {
                    collapseToggle: "Toggle list {{listTitle}}",
                    showCandidates: "Show candidates",
                    hideCandidates: "Hide candidates",
                    selectedCandidate: "{{count}} candidate selected",
                    selectedCandidates: "{{count}} candidates selected",
                    expandAll: "Expand all",
                    collapseAll: "Collapse all",
                },
                reviewScreen: {
                    blankBallot: "Blank ballot",
                    declineToVote: "Declined to vote",
                },
                contest: {
                    acclamation: {description: "Decided by acclamation"},
                },
            },
        },
    },
    interpolation: {escapeValue: false},
})

/** A candidate, with only what a test cares about spelled out. */
export const aCandidate = (id: string, name: string, over: Partial<ICandidate> = {}): ICandidate =>
    ({
        id,
        contest_id: CONTEST_ID,
        name,
        description: "",
        sort_order: 0,
        presentation: {},
        ...over,
    }) as unknown as ICandidate

/** A blank, invalid or write-in option: one flag on `presentation`. */
export const anOption = (
    id: string,
    name: string,
    flag: "is_explicit_blank" | "is_explicit_invalid" | "is_write_in",
    extra: Record<string, unknown> = {}
): ICandidate => aCandidate(id, name, {presentation: {[flag]: true, ...extra}} as never)

/** A contest. `presentation` is where every layout decision is made. */
export const aContest = (over: Partial<IContest> = {}): IContest =>
    ({
        id: CONTEST_ID,
        election_id: ELECTION_ID,
        name: "President",
        description: "",
        min_votes: 0,
        max_votes: 1,
        winning_candidates_num: 1,
        counting_algorithm: "plurality-at-large",
        candidates: [aCandidate("a", "Alice Okonjo"), aCandidate("b", "Bob Iyer")],
        presentation: {},
        ...over,
    }) as unknown as IContest

/** The ballot style the components take, around one contest. */
export const aBallotStyle = (contest: IContest): IBallotStyle =>
    ({
        id: "style-1",
        election_id: ELECTION_ID,
        election_event_id: "event-1",
        tenant_id: "tenant-1",
        area_id: "area-1",
        ballot_eml: {
            id: "style-1",
            tenant_id: "tenant-1",
            election_event_id: "event-1",
            election_id: ELECTION_ID,
            area_id: "area-1",
            contests: [contest],
            election_event_presentation: {},
            election_presentation: {},
        },
    }) as unknown as IBallotStyle

/** The voter's marks on one contest. `selected` is a rank; `-1` is unselected. */
export const marks = (
    contest: IContest,
    chosen: Record<string, number> = {},
    over: Partial<IDecodedVoteContest> = {}
): IDecodedVoteContest =>
    ({
        contest_id: contest.id,
        is_explicit_invalid: false,
        choices: contest.candidates.map((candidate) => ({
            id: candidate.id,
            selected: chosen[candidate.id] ?? -1,
            write_in_text: "",
        })),
        invalid_errors: [],
        invalid_alerts: [],
        ...over,
    }) as unknown as IDecodedVoteContest

/** One encoder complaint; `message` is a translation key. */
export const anError = (message: string): {message: string} => ({message})

/** Every call a port received, so a test can say what the ballot asked for. */
export type PortCall = [keyof BallotSelectionPort, unknown]

/** The selection port over a mutable record, re-rendering the tree on each write. */
const portOver = (
    held: {current: IDecodedVoteContest},
    calls: Array<PortCall>,
    rerender: () => void,
    isVoted = false
): BallotSelectionPort => {
    const write =
        (name: keyof BallotSelectionPort, apply: (input: never) => IDecodedVoteContest) =>
        (input: never) => {
            calls.push([name, input])
            held.current = apply(input)
            rerender()
        }
    return {
        contest: () => held.current,
        choice: (_style, _contestId, candidateId) =>
            held.current.choices.find((choice) => choice.id === candidateId),
        setChoice: write("setChoice", ({voteChoice}: {voteChoice: {id: string}}) => {
            const known = held.current.choices.some((choice) => choice.id === voteChoice.id)
            return {
                ...held.current,
                choices: known
                    ? held.current.choices.map((choice) =>
                          choice.id === voteChoice.id ? {...choice, ...voteChoice} : choice
                      )
                    : [...held.current.choices, voteChoice],
            } as IDecodedVoteContest
        }),
        setBlank: write("setBlank", ({candidateId}: {candidateId: string}) => ({
            ...held.current,
            choices: held.current.choices.map((choice) => ({
                ...choice,
                selected: choice.id === candidateId ? 0 : -1,
            })),
        })),
        setInvalid: write("setInvalid", ({isExplicitInvalid}: {isExplicitInvalid: boolean}) => ({
            ...held.current,
            is_explicit_invalid: isExplicitInvalid,
        })),
        reset: write("reset", () => ({
            ...held.current,
            is_explicit_invalid: false,
            choices: held.current.choices.map((choice) => ({...choice, selected: -1})),
        })),
        isVoted: () => isVoted,
        imageBaseUrl: "https://assets.example/",
    }
}

/**
 * The engine, standing in for sequent-core: order preserved, blank when nothing
 * is marked, preferential by algorithm name, and a write-in budget of
 * `writeInBudget` characters minus what has been typed.
 */
export const anEngine = (writeInBudget = 240): BallotEngine => ({
    sortCandidatesInContest: (candidates) => candidates,
    isPreferential: (algorithm) =>
        /borda|instant.?runoff|stv|preferential|ranked/i.test(String(algorithm ?? "")),
    checkIsBlank: (contest) =>
        contest.is_explicit_invalid === true
            ? false
            : (contest.choices ?? []).every((choice) => choice.selected < 0),
    getWriteInAvailableCharacters: (contest) =>
        writeInBudget -
        contest.choices.reduce((used, choice) => used + (choice.write_in_text?.length ?? 0), 0),
    isEligibleAcclaimedCandidate: (candidate) =>
        !candidate.presentation?.is_explicit_blank &&
        !candidate.presentation?.is_explicit_invalid &&
        !candidate.presentation?.is_disabled &&
        !candidate.presentation?.is_write_in,
})

const Providers: React.FC<{
    engine: BallotEngine
    port: BallotSelectionPort
    children: React.ReactNode
}> = ({engine, port, children}) => (
    <I18nextProvider i18n={i18n}>
        <ThemeProvider theme={theme}>
            <BallotEngineProvider engine={engine}>
                <BallotSelectionProvider port={port}>{children}</BallotSelectionProvider>
            </BallotEngineProvider>
        </ThemeProvider>
    </I18nextProvider>
)

export interface MountErrorsOptions {
    alerts?: Array<{message: string}>
    errors?: Array<{message: string}>
    isReview?: boolean
    isTouched?: boolean
    isVoted?: boolean
    hasWriteIns?: boolean
    selection?: IDecodedVoteContest
    engine?: BallotEngine
}

export interface MountedErrors extends RenderResult {
    /** What the list handed up through `setDecodedContests`. */
    decoded: () => IDecodedVoteContest | undefined
    /** The last value it passed to `setIsInvalidWriteIns`. */
    invalidWriteIns: () => boolean | undefined
    /** Whether it asked to be marked touched. */
    touched: () => boolean
}

/** Mount the warning list on its own, so its policy matrix is directly reachable. */
export const mountErrors = (
    contest: IContest,
    {
        alerts = [],
        errors = [],
        isReview = false,
        isTouched = isReview,
        isVoted = false,
        hasWriteIns = false,
        selection,
        engine = anEngine(),
    }: MountErrorsOptions = {}
): MountedErrors => {
    const ballotStyle = aBallotStyle(contest)
    const state = {
        ...(selection ?? marks(contest)),
        invalid_alerts: alerts,
        invalid_errors: errors,
    } as unknown as IDecodedVoteContest
    let decoded: IDecodedVoteContest | undefined
    let invalidWriteIns: boolean | undefined
    let touched = false
    const result = render(
        <Providers engine={engine} port={portOver({current: state}, [], () => undefined, isVoted)}>
            <InvalidErrorsList
                ballotStyle={ballotStyle}
                question={contest}
                hasWriteIns={hasWriteIns}
                isInvalidWriteIns={false}
                setIsInvalidWriteIns={(value) => {
                    invalidWriteIns = value
                }}
                setDecodedContests={(next) => {
                    decoded = next
                }}
                isReview={isReview}
                errorSelectionState={[state]}
                isTouched={isTouched}
                setIsTouched={(value) => {
                    touched = value
                }}
            />
        </Providers>
    )
    return {
        ...result,
        decoded: () => decoded,
        invalidWriteIns: () => invalidWriteIns,
        touched: () => touched,
    }
}

export interface MountOptions {
    /** What the voter has marked so far. Defaults to nothing marked. */
    selection?: IDecodedVoteContest
    isReview?: boolean
    isDeclineToVote?: boolean
    isBlankBallot?: boolean
    /** Errors the encoder reported, which the screen injects rather than derives. */
    errors?: BallotSelection
    engine?: BallotEngine
    ballotStyle?: IBallotStyle
}

export interface Mounted extends RenderResult {
    ballotStyle: IBallotStyle
    /** The voter's marks as the port now holds them. */
    held: () => IDecodedVoteContest
    /** Every write the ballot made through the port, in order. */
    calls: Array<PortCall>
    /** What the component last handed up through `setDecodedContests`. */
    decoded: () => IDecodedVoteContest | undefined
    /** Whether the component asked for Next to be disabled. */
    nextDisabled: () => boolean
}

/** Mount one contest the way a screen mounts it. */
export const mountContest = (
    contest: IContest,
    {
        selection,
        isReview = false,
        isDeclineToVote,
        isBlankBallot,
        errors = [],
        engine = anEngine(),
        ballotStyle = aBallotStyle(contest),
    }: MountOptions = {}
): Mounted => {
    const held = {current: selection ?? marks(contest)}
    const calls: Array<PortCall> = []
    let handedUp: IDecodedVoteContest | undefined
    let disabled = false
    // A store notifies its subscribers on every write; bumping a counter above
    // the ballot is the equivalent, so a click is visible to the next render.
    const bump = {current: (): void => undefined}
    const port = portOver(held, calls, () => bump.current())
    const Live: React.FC = () => {
        const [, setTick] = React.useState(0)
        bump.current = () => setTick((tick) => tick + 1)
        return (
            <Question
                ballotStyle={ballotStyle}
                question={contest}
                isReview={isReview}
                isDeclineToVote={isDeclineToVote}
                isBlankBallot={isBlankBallot}
                errorSelectionState={errors}
                setDecodedContests={(next) => {
                    handedUp = next
                }}
                setDisableNext={(value) => {
                    disabled = value
                }}
            />
        )
    }
    const result = render(
        <Providers engine={engine} port={port}>
            <Live />
        </Providers>
    )

    return {
        ...result,
        ballotStyle,
        held: () => held.current,
        calls,
        decoded: () => handedUp,
        nextDisabled: () => disabled,
    }
}
