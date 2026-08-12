// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Mounting a contest, and building the fixtures a contest needs.
 *
 * **Why this file exists separately from the tests.** The components it mounts
 * are about to move into `@sequentech/ui-essentials`, where they will take their
 * selection state as props instead of reading it from redux. If each test wired
 * up its own store, every test would have to be rewritten by that move — and a
 * test rewritten alongside the code it guards guards nothing.
 *
 * So all the wiring lives here, in one place, behind `mountContest`. Today it
 * builds a real store and wraps the tree in a `Provider`. After the move it will
 * pass props. **The assertions in the test files do not change either way**,
 * which is the entire point: they describe what a voter sees, and that is what
 * must survive.
 *
 * `renderToStaticMarkup` was the other option and is what the four existing
 * `ui-essentials` tests use. It cannot click, so it cannot see any of the
 * behaviour that actually breaks — selection, over-vote disabling, collapse. A
 * DOM was worth the setup.
 */

import {configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import {render, RenderResult} from "@testing-library/react"
import {BallotSelectionProvider, theme} from "@sequentech/ui-essentials"
import type {BallotSelectionPort} from "@sequentech/ui-essentials"
import type {BallotSelection, ICandidate, IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"

import ballotSelectionsReducer from "../store/ballotSelections/ballotSelectionsSlice"
import ballotStylesReducer from "../store/ballotStyles/ballotStylesSlice"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import extraReducer from "../store/extra/extraSlice"
import englishTranslation from "../translations/en"
// The other half of what a ballot reads. Imported by source path rather than
// through the package barrel, which would pull `TallyResults` and a data grid.
import essentialsTranslation from "../../../ui-essentials/src/translations/en"

export const ELECTION_ID = "election-1"
export const CONTEST_ID = "contest-1"

/**
 * i18n with the real English bundles, both of them, merged.
 *
 * Real strings rather than a stub, because a test that asserts on "Blank vote"
 * should fail when somebody renames it in the catalogue — that rename reaches
 * voters.
 *
 * **Both** bundles, because a ballot reads from two: `candidatesList.*` and
 * `votingScreen.*` come from `voting-portal`, while `candidate.*` — the write-in
 * placeholder, the ordinals, the "more information" link — comes from
 * `ui-essentials`, whose components draw the rows. The running app merges them in
 * `ui-core`'s `initializeLanguages`; this mirrors that, portal last so it wins,
 * which is the same precedence.
 *
 * Getting this wrong is quiet: with no resources loaded, `t()` returns the key
 * itself, so a heading reads `candidatesList.expandAll` and only a test that
 * asserts on visible text notices. It cost two failures out of twenty-five here,
 * and the other twenty-three were green for the wrong reason — they assert on
 * candidate data, which is not translated.
 */
const deepMerge = (base: Record<string, unknown>, over: Record<string, unknown>) => {
    const out: Record<string, unknown> = {...base}
    for (const [key, value] of Object.entries(over)) {
        const mine = out[key]
        out[key] =
            value !== null &&
            typeof value === "object" &&
            !Array.isArray(value) &&
            mine !== null &&
            typeof mine === "object" &&
            !Array.isArray(mine)
                ? deepMerge(mine as Record<string, unknown>, value as Record<string, unknown>)
                : value
    }
    return out
}

const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {
        en: {
            translation: deepMerge(
                essentialsTranslation.translations as Record<string, unknown>,
                englishTranslation.translations as Record<string, unknown>
            ),
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

/** The ballot style wrapper the components take, around one contest. */
export const aBallotStyle = (contest: IContest): IBallotStyle =>
    ({
        id: "style-1",
        election_id: ELECTION_ID,
        election_event_id: "event-1",
        tenant_id: "tenant-1",
        area_id: "area-1",
        created_at: "2027-01-01T00:00:00Z",
        last_updated_at: "2027-01-01T00:00:00Z",
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

/**
 * The voter's marks on one contest, in the platform's own decoded shape.
 *
 * `selected` is a rank, and `-1` means unselected — not `0`, which is the first
 * preference. Worth stating in the fixture builder because getting it wrong
 * produces a ballot that looks blank while every box is ticked.
 */
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

/**
 * One encoder complaint, in the shape the WASM reports them.
 *
 * `message` is a translation key, which is why these read like
 * `errors.implicit.underVote` — the component translates it, and `warnIdToClassName`
 * turns it into a class so a client's CSS can target one specific warning.
 */
export const anError = (message: string): {message: string} => ({message})


/**
 * The selection port, over a plain object.
 *
 * The third implementation of `BallotSelectionPort` — the portal has one over
 * redux, the wizard will have one over local state, and this one is a mutable
 * record. Having it here is not only convenience: if the port turned out to be
 * satisfiable *only* by a redux store, the wizard could not satisfy it either, and
 * this file is where that would show up first.
 */
const portOver = (
    held: {current: IDecodedVoteContest},
    isVoted = false
): BallotSelectionPort => ({
    contest: () => held.current,
    choice: (_style, _contestId, candidateId) =>
        held.current.choices.find((choice) => choice.id === candidateId),
    setChoice: ({voteChoice}) => {
        held.current = {
            ...held.current,
            choices: held.current.choices.map((choice) =>
                choice.id === voteChoice.id ? {...choice, ...voteChoice} : choice
            ),
        } as IDecodedVoteContest
    },
    setBlank: ({candidateId}) => {
        held.current = {
            ...held.current,
            choices: held.current.choices.map((choice) => ({
                ...choice,
                selected: choice.id === candidateId ? 0 : -1,
            })),
        } as IDecodedVoteContest
    },
    setInvalid: ({isExplicitInvalid}) => {
        held.current = {
            ...held.current,
            is_explicit_invalid: isExplicitInvalid,
        } as IDecodedVoteContest
    },
    reset: () => {
        held.current = {
            ...held.current,
            is_explicit_invalid: false,
            choices: held.current.choices.map((choice) => ({...choice, selected: -1})),
        } as IDecodedVoteContest
    },
    isVoted: () => isVoted,
    imageBaseUrl: "",
})

export interface MountErrorsOptions {
    /** What the encoder said about this contest. */
    alerts?: Array<{message: string}>
    errors?: Array<{message: string}>
    isReview?: boolean
    /** `Question` starts this at `isReview`: nothing is warned about until touched. */
    isTouched?: boolean
    isVoted?: boolean
}

/**
 * Mount the warning list on its own.
 *
 * Separately from `mountContest` because the thing under test is
 * `filterErrorList` — eighty lines of interacting policy predicates that decide
 * which of the encoder's complaints a voter is shown, and when. Driving it
 * through a whole contest would mean provoking real encoder errors, which needs
 * the real WASM; driving it directly means the policy matrix is reachable.
 *
 * `useParams` is why this needs a router: the component reads the election id off
 * the URL rather than from a prop. `EA-F1-004` turns that into a prop, and this
 * wrapper is where that change lands.
 */
export const mountErrors = (
    contest: IContest,
    {
        alerts = [],
        errors = [],
        isReview = false,
        isTouched = isReview,
        isVoted = false,
    }: MountErrorsOptions = {}
): RenderResult => {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const {InvalidErrorsList} = require("@sequentech/ui-essentials") as {
        InvalidErrorsList: React.FC<Record<string, unknown>>
    }
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const {MemoryRouter, Route, Routes} = require("react-router-dom") as {
        MemoryRouter: React.FC<Record<string, unknown>>
        Route: React.FC<Record<string, unknown>>
        Routes: React.FC<Record<string, unknown>>
    }

    const ballotStyle = aBallotStyle(contest)
    const state = {
        ...marks(contest),
        invalid_alerts: alerts,
        invalid_errors: errors,
    } as unknown as IDecodedVoteContest

    const store = configureStore({
        reducer: {
            ballotSelections: ballotSelectionsReducer,
            ballotStyles: ballotStylesReducer,
            extra: extraReducer,
        },
        preloadedState: {
            ballotSelections: {[ELECTION_ID]: [state]},
            ballotStyles: {[ELECTION_ID]: ballotStyle},
            extra: {isVoted: isVoted ? {[ELECTION_ID]: true} : {}},
        } as never,
    })

    const list = (
        <InvalidErrorsList
            ballotStyle={ballotStyle}
            question={contest}
            hasWriteIns={false}
            isInvalidWriteIns={false}
            setIsInvalidWriteIns={() => undefined}
            setDecodedContests={() => undefined}
            isReview={isReview}
            errorSelectionState={[state]}
            isTouched={isTouched}
            setIsTouched={() => undefined}
        />
    )

    return render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <BallotSelectionProvider port={portOver({current: state}, isVoted)}>
                    <MemoryRouter
                        initialEntries={[
                            `/tenant/tenant-1/event/event-1/election/${ELECTION_ID}/vote`,
                        ]}
                    >
                        <Routes>
                            <Route
                                path="/tenant/:tenantId/event/:eventId/election/:electionId/vote"
                                element={list}
                            />
                        </Routes>
                    </MemoryRouter>
                    </BallotSelectionProvider>
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
}

export interface MountOptions {
    /** What the voter has marked so far. Defaults to nothing marked. */
    selection?: IDecodedVoteContest
    /** The review screen renders the same contest read-only-ish. */
    isReview?: boolean
    isDeclineToVote?: boolean
    /** Errors the encoder reported, which the screen injects rather than derives. */
    errors?: BallotSelection
}

export interface Mounted extends RenderResult {
    contest: IContest
    ballotStyle: IBallotStyle
    /** What the component last handed up through `setDecodedContests`. */
    decoded: () => IDecodedVoteContest | undefined
    /** Whether the component asked for Next to be disabled. */
    nextDisabled: () => boolean
}

/**
 * Mount one contest the way a screen mounts it.
 *
 * Everything specific to redux is in this function. `Question` is imported
 * lazily so that a test file importing this module does not pull the component
 * tree until it mounts something — which keeps a failure in one fixture from
 * reading as a failure to import.
 */
export const mountContest = (
    contest: IContest,
    {selection, isReview = false, isDeclineToVote, errors = []}: MountOptions = {}
): Mounted => {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const {Question} = require("@sequentech/ui-essentials") as {
        Question: React.FC<Record<string, unknown>>
    }

    const ballotStyle = aBallotStyle(contest)
    const chosen = selection ?? marks(contest)
    // Held in a box the port mutates, so a click is visible to the next render —
    // which is what the store used to do, and is the whole behaviour these tests
    // are about.
    const held = {current: chosen}

    const store = configureStore({
        reducer: {
            ballotSelections: ballotSelectionsReducer,
            ballotStyles: ballotStylesReducer,
            extra: extraReducer,
        },
        preloadedState: {
            ballotSelections: {[ELECTION_ID]: [chosen]},
            ballotStyles: {[ELECTION_ID]: ballotStyle},
        } as never,
    })

    let handedUp: IDecodedVoteContest | undefined
    let disabled = false

    const result = render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <BallotSelectionProvider port={portOver(held)}>
                    <Question
                        ballotStyle={ballotStyle}
                        question={contest}
                        isReview={isReview}
                        isDeclineToVote={isDeclineToVote}
                        errorSelectionState={errors}
                        setDecodedContests={(next: IDecodedVoteContest) => {
                            handedUp = next
                        }}
                        setDisableNext={(value: boolean) => {
                            disabled = value
                        }}
                    />
                    </BallotSelectionProvider>
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )

    return {
        ...result,
        contest,
        ballotStyle,
        decoded: () => handedUp,
        nextDisabled: () => disabled,
    }
}
