// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    BallotSelection,
    EBlankVotePolicy,
    EConsolidatedReportPolicy,
    EEarlyVotingPolicy,
    EElectionEventDelegatedVotingPolicy,
    EOverVotePolicy,
    EUnderVotePolicy,
    EVotingPortalCountdownPolicy,
    EVotingStatus,
    IBallotStyle as IElectionEml,
    IDecodedVoteContest,
    IElectionStatus,
} from "@sequentech/ui-core"
import {configureStore} from "@reduxjs/toolkit"
import {
    ELECTION_CATEGORIES,
    ELECTION_WITH_INVALID,
    ELECTION_WRITEINS_SIMPLE,
    SIMPLE_ELECTION_PLURALITY,
} from "@voting-portal/fixtures/election"
import {
    STUDIO_WRITEINS_PAGINATED,
    STUDIO_WRITEIN_ELECTION_ID,
} from "./fixtures/studioElections"
import {isVoteRouteScene} from "./catalog"
import auditableBallotsReducer from "@voting-portal/store/auditableBallots/auditableBallotsSlice"
import {
    IBallotStyle,
    setBallotStyle,
    ballotStylesSlice,
} from "@voting-portal/store/ballotStyles/ballotStylesSlice"
import ballotSelectionsReducer, {
    resetBallotSelection,
    setBallotSelection,
} from "@voting-portal/store/ballotSelections/ballotSelectionsSlice"
import {addCastVotes} from "@voting-portal/store/castVotes/castVotesSlice"
import castVotesReducer from "@voting-portal/store/castVotes/castVotesSlice"
import confirmationScreenDataReducer, {
    setConfirmationScreenData,
} from "@voting-portal/store/castVotes/confirmationScreenDataSlice"
import {setAuditableBallot} from "@voting-portal/store/auditableBallots/auditableBallotsSlice"
import documentsReducer from "@voting-portal/store/documents/documentsSlice"
import {provideBallotService} from "@voting-portal/services/BallotService"
import electionEventReducer, {setElectionEvent} from "@voting-portal/store/electionEvents/electionEventsSlice"
import electionsReducer, {IElectionExtended, setElection} from "@voting-portal/store/elections/electionsSlice"
import extraReducer from "@voting-portal/store/extra/extraSlice"
import supportMaterialReducer, {setSupportMaterial} from "@voting-portal/store/supportMaterials/supportMaterialsSlice"
import {LOC_STUDIO_LANGUAGES} from "./i18n"
import {buildPreviewBallotStyles, UploadedBallotStyle, UploadedElectionEvent} from "./uploadedElection"

const stripSortOrders = (eml: IElectionEml): void => {
    if (eml.election_event_presentation) {
        delete eml.election_event_presentation.elections_order
    }
    if (eml.election_presentation) {
        delete eml.election_presentation.contests_order
    }
    eml.contests?.forEach((contest) => {
        if (contest.presentation) {
            delete contest.presentation.candidates_order
        }
    })
}

export const STUDIO_TENANT_ID = SIMPLE_ELECTION_PLURALITY.tenant_id
export const STUDIO_EVENT_ID = SIMPLE_ELECTION_PLURALITY.election_event_id
export const STUDIO_OPEN_ELECTION_ID = SIMPLE_ELECTION_PLURALITY.election_id
export const STUDIO_CLOSED_ELECTION_ID = ELECTION_CATEGORIES.election_id
export const STUDIO_BLANK_ELECTION_ID = ELECTION_WITH_INVALID.election_id
export const STUDIO_WRITEINS_PAGINATED_ELECTION_ID = STUDIO_WRITEINS_PAGINATED.election_id
export const STUDIO_BALLOT_ID = "a1b2c3d4e5f67890a1b2c3d4e5f67890"

type SelectionMode = "none" | "first" | "overvote" | "undervote" | "blank" | "invalid"

const ballotEmlForScene = (sceneId: string): IElectionEml => {
    switch (sceneId) {
        case "write-in":
            return STUDIO_WRITEINS_PAGINATED
        case "overvote":
        case "undervote":
            return ELECTION_WRITEINS_SIMPLE
        case "blank":
        case "invalid":
            return ELECTION_WITH_INVALID
        default:
            return SIMPLE_ELECTION_PLURALITY
    }
}

const selectionModeForScene = (sceneId: string, variantId: string): SelectionMode => {
    switch (sceneId) {
        case "overvote":
            return "overvote"
        case "undervote":
            return "undervote"
        case "blank":
            return "blank"
        case "invalid":
            return "invalid"
        case "voting":
            return variantId === "default" ? "none" : "first"
        default:
            return "none"
    }
}

const openStatus = (votingStatus: EVotingStatus): IElectionStatus => ({
    is_published: true,
    voting_status: votingStatus,
    kiosk_voting_status: EVotingStatus.NOT_STARTED,
    early_voting_status: EVotingStatus.NOT_STARTED,
    voting_period_dates: {},
    kiosk_voting_period_dates: {},
    early_voting_period_dates: {},
})

const cloneEml = (eml: IElectionEml): IElectionEml => JSON.parse(JSON.stringify(eml)) as IElectionEml

const patchContestPolicies = (eml: IElectionEml): IElectionEml => {
    const next = cloneEml(eml)
    next.contests = next.contests.map((contest) => ({
        ...contest,
        presentation: {
            ...contest.presentation,
            over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_ALERT,
            under_vote_policy: EUnderVotePolicy.WARN_AND_ALERT,
            blank_vote_policy: EBlankVotePolicy.WARN,
        },
    }))
    return next
}

export const electionIdForScene = (sceneId: string, _variantId: string): string => {
    switch (sceneId) {
        case "write-in":
            return STUDIO_WRITEINS_PAGINATED_ELECTION_ID
        case "overvote":
        case "undervote":
            return STUDIO_WRITEIN_ELECTION_ID
        case "blank":
        case "invalid":
            return STUDIO_BLANK_ELECTION_ID
        default:
            return STUDIO_OPEN_ELECTION_ID
    }
}

export const isDemoVariant = (sceneId: string, variantId: string): boolean =>
    variantId === "demo" ||
    (sceneId === "confirmation" && variantId === "demo") ||
    (sceneId === "start" && variantId === "demo") ||
    (sceneId === "election-list" && variantId === "demo")

export const disableAuthFor = (sceneId: string, variantId: string): boolean => {
    if (sceneId === "election-list" && variantId === "errors") {
        return false
    }
    if (sceneId === "ballot-locator" && (variantId === "found" || variantId === "not-found")) {
        return false
    }
    if (sceneId === "review" && variantId === "error") {
        return false
    }
    return true
}

const wrapBallotStyle = (
    eml: IElectionEml,
    options: {isDemo: boolean; confirmCast: boolean; countdown: boolean}
): IBallotStyle => {
    const ballotEml = patchContestPolicies(eml)
    ballotEml.tenant_id = STUDIO_TENANT_ID
    ballotEml.election_event_id = STUDIO_EVENT_ID
    ballotEml.num_allowed_revotes = 1
    ballotEml.public_key = {
        public_key: eml.public_key?.public_key || "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
        is_demo: options.isDemo,
    }
    ballotEml.area_presentation = {
        allow_early_voting: EEarlyVotingPolicy.NO_EARLY_VOTING,
        ...ballotEml.area_presentation,
    }
    ballotEml.election_presentation = {
        ...ballotEml.election_presentation,
        consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
        cast_vote_confirm: options.confirmCast,
    }
    ballotEml.election_event_presentation = {
        ...ballotEml.election_event_presentation,
        delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
        show_user_profile: true,
        materials: {activated: true},
        language_conf: {
            enabled_language_codes: [...LOC_STUDIO_LANGUAGES],
            default_language_code: "en",
        },
        voting_portal_countdown_policy: {
            policy: options.countdown
                ? EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT
                : EVotingPortalCountdownPolicy.NO_COUNTDOWN,
            countdown_alert_anticipation_secs: options.countdown ? 60 : 0,
            countdown_anticipation_secs: options.countdown ? 120 : 0,
        },
    }
    return {
        id: ballotEml.id,
        election_id: ballotEml.election_id,
        election_event_id: STUDIO_EVENT_ID,
        tenant_id: STUDIO_TENANT_ID,
        ballot_eml: ballotEml,
        created_at: "",
        last_updated_at: "",
        area_id: ballotEml.area_id,
    }
}

const toElection = (
    ballotStyle: IBallotStyle,
    votingStatus: EVotingStatus,
    title: string
): IElectionExtended => ({
    id: ballotStyle.election_id,
    election_event_id: STUDIO_EVENT_ID,
    tenant_id: STUDIO_TENANT_ID,
    name: title,
    description: ballotStyle.ballot_eml.description,
    image_document_id: "",
    contests: ballotStyle.ballot_eml.contests,
    presentation: ballotStyle.ballot_eml.election_presentation,
    num_allowed_revotes: 1,
    status: openStatus(votingStatus) as unknown as string,
})

const selectionFor = (ballotStyle: IBallotStyle, mode: SelectionMode): BallotSelection =>
    ballotStyle.ballot_eml.contests.map((contest): IDecodedVoteContest => {
        const explicitInvalidId = contest.candidates.find(
            (candidate) => candidate.presentation?.is_explicit_invalid
        )?.id
        const regularCandidates = contest.candidates.filter(
            (candidate) =>
                !candidate.presentation?.is_write_in && !candidate.presentation?.is_explicit_invalid
        )
        return {
            contest_id: contest.id,
            is_explicit_invalid: mode === "invalid",
            invalid_errors: [],
            invalid_alerts: [],
            choices: contest.candidates.map((candidate, index) => {
                if (mode === "invalid") {
                    return {id: candidate.id, selected: -1}
                }
                if (mode === "blank") {
                    return {id: candidate.id, selected: -1}
                }
                if (mode === "overvote") {
                    return {id: candidate.id, selected: index < 3 ? 0 : -1}
                }
                if (mode === "undervote") {
                    const regularIndex = regularCandidates.findIndex(
                        (entry) => entry.id === candidate.id
                    )
                    return {
                        id: candidate.id,
                        selected: regularIndex === 0 ? 0 : -1,
                    }
                }
                if (mode === "first") {
                    return {id: candidate.id, selected: index === 0 ? 0 : -1}
                }
                return {id: candidate.id, selected: -1}
            }),
        }
    })

export const createStudioStore = (sceneId: string, variantId: string) => {
    const store = configureStore({
        reducer: {
            elections: electionsReducer,
            castVotes: castVotesReducer,
            ballotStyles: ballotStylesSlice.reducer,
            ballotSelections: ballotSelectionsReducer,
            auditableBallots: auditableBallotsReducer,
            supportMaterials: supportMaterialReducer,
            electionEvent: electionEventReducer,
            extra: extraReducer,
            documents: documentsReducer,
            confirmationScreenData: confirmationScreenDataReducer,
        },
    })

    const isDemo = isDemoVariant(sceneId, variantId)
    const confirmCast = sceneId === "review" && variantId === "confirm"
    const countdown = sceneId === "session" && variantId === "timeout"

    if (isDemo) {
        sessionStorage.setItem("isDemo", "true")
    } else {
        sessionStorage.removeItem("isDemo")
    }

    const openEml = ballotEmlForScene(sceneId)
    const openBallot = wrapBallotStyle(openEml, {isDemo, confirmCast, countdown})
    const closedBallot = wrapBallotStyle(ELECTION_CATEGORIES, {
        isDemo: false,
        confirmCast: false,
        countdown: false,
    })

    store.dispatch(setBallotStyle(openBallot))
    store.dispatch(
        setElection(
            toElection(openBallot, EVotingStatus.OPEN, openEml.contests[0]?.name || "Mayor")
        )
    )
    store.dispatch(setBallotStyle(closedBallot))
    store.dispatch(
        setElection(
            toElection(
                closedBallot,
                EVotingStatus.CLOSED,
                ELECTION_CATEGORIES.contests[0]?.name || "City Council"
            )
        )
    )

    store.dispatch(
        setElectionEvent({
            id: STUDIO_EVENT_ID,
            tenant_id: STUDIO_TENANT_ID,
            name: "Municipal Election",
            description: "Official voting booth",
            presentation: openBallot.ballot_eml.election_event_presentation,
            status: openStatus(EVotingStatus.OPEN) as unknown as string,
        })
    )

    store.dispatch(
        addCastVotes([
            {
                id: "closed-cast",
                tenant_id: STUDIO_TENANT_ID,
                election_id: STUDIO_CLOSED_ELECTION_ID,
                election_event_id: STUDIO_EVENT_ID,
            },
        ])
    )

    store.dispatch(
        setSupportMaterial({
            id: "material-1",
            tenant_id: STUDIO_TENANT_ID,
            election_event_id: STUDIO_EVENT_ID,
            kind: "pdf",
            data: {
                title: "Voter information",
                subtitle: "How to mark your ballot",
            } as unknown as string,
        })
    )

    const selectionMode = isVoteRouteScene(sceneId)
        ? selectionModeForScene(sceneId, variantId)
        : "first"

    store.dispatch(resetBallotSelection({ballotStyle: openBallot, force: true}))
    store.dispatch(
        setBallotSelection({
            ballotStyle: openBallot,
            ballotSelection: selectionFor(openBallot, selectionMode),
        })
    )

    store.dispatch(
        setConfirmationScreenData({
            electionId: openBallot.election_id,
            confirmationScreenData: {
                ballotId: STUDIO_BALLOT_ID,
                isDemo,
            },
        })
    )

    try {
        const {encryptBallotSelection, hashBallot} = provideBallotService()
        const reviewSelection =
            selectionMode === "none" || selectionMode === "blank" || selectionMode === "invalid"
                ? selectionFor(openBallot, "first")
                : selectionFor(openBallot, selectionMode)
        const encoded = encryptBallotSelection(reviewSelection, openBallot.ballot_eml)
        const ballotId = hashBallot(encoded)
        store.dispatch(
            setAuditableBallot({
                electionId: openBallot.election_id,
                auditableBallot: encoded,
            })
        )
        if (ballotId) {
            store.dispatch(
                setConfirmationScreenData({
                    electionId: openBallot.election_id,
                    confirmationScreenData: {
                        ballotId,
                        isDemo,
                    },
                })
            )
        }
    } catch (error) {
        console.error("Loc studio could not encode a sample ballot", error)
    }

    return store
}

export type StudioStore = ReturnType<typeof createStudioStore>

const DEMO_PUBLIC_KEY = "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4"

const finalizeUploadedBallotStyle = (
    ballotStyle: UploadedBallotStyle,
    options: {isDemo: boolean; confirmCast: boolean; languages: string[]}
): IBallotStyle => {
    const eml = patchContestPolicies(cloneEml(ballotStyle.ballot_eml))
    stripSortOrders(eml)
    eml.tenant_id = eml.tenant_id || "loc-studio-tenant"
    eml.election_event_id = eml.election_event_id || "loc-studio-event"
    eml.area_id = eml.area_id || "loc-studio-area"
    eml.num_allowed_revotes = eml.num_allowed_revotes ?? 1
    eml.public_key = eml.public_key?.public_key
        ? eml.public_key
        : {public_key: DEMO_PUBLIC_KEY, is_demo: options.isDemo}
    eml.area_presentation = {
        allow_early_voting: EEarlyVotingPolicy.NO_EARLY_VOTING,
        ...eml.area_presentation,
    }
    eml.election_presentation = {
        consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
        ...eml.election_presentation,
        cast_vote_confirm: options.confirmCast || Boolean(eml.election_presentation?.cast_vote_confirm),
    }
    eml.election_event_presentation = {
        delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
        show_user_profile: true,
        materials: {activated: true},
        language_conf: {
            enabled_language_codes: options.languages,
            default_language_code: options.languages[0] || "en",
        },
        voting_portal_countdown_policy: {
            policy: EVotingPortalCountdownPolicy.NO_COUNTDOWN,
            countdown_alert_anticipation_secs: 0,
            countdown_anticipation_secs: 0,
        },
        ...eml.election_event_presentation,
    }
    return {
        id: ballotStyle.id,
        election_id: ballotStyle.election_id,
        election_event_id: eml.election_event_id,
        tenant_id: eml.tenant_id,
        ballot_eml: eml,
        created_at: "",
        last_updated_at: "",
        area_id: eml.area_id,
    }
}

const toUploadedElection = (
    ballotStyle: IBallotStyle,
    votingStatus: EVotingStatus
): IElectionExtended => {
    const firstContest = ballotStyle.ballot_eml.contests[0]
    return {
        id: ballotStyle.election_id,
        election_event_id: ballotStyle.election_event_id,
        tenant_id: ballotStyle.tenant_id,
        name: firstContest?.name || ballotStyle.election_id,
        description: ballotStyle.ballot_eml.description,
        image_document_id: "",
        contests: ballotStyle.ballot_eml.contests,
        presentation: ballotStyle.ballot_eml.election_presentation,
        num_allowed_revotes: ballotStyle.ballot_eml.num_allowed_revotes || 1,
        status: openStatus(votingStatus) as unknown as string,
    }
}

export const createStudioStoreFromUpload = (
    uploaded: UploadedElectionEvent,
    sceneId: string,
    variantId: string,
    language: string
) => {
    const store = configureStore({
        reducer: {
            elections: electionsReducer,
            castVotes: castVotesReducer,
            ballotStyles: ballotStylesSlice.reducer,
            ballotSelections: ballotSelectionsReducer,
            auditableBallots: auditableBallotsReducer,
            supportMaterials: supportMaterialReducer,
            electionEvent: electionEventReducer,
            extra: extraReducer,
            documents: documentsReducer,
            confirmationScreenData: confirmationScreenDataReducer,
        },
    })

    const isDemo = isDemoVariant(sceneId, variantId)
    const confirmCast = sceneId === "review" && variantId === "confirm"

    if (isDemo) {
        sessionStorage.setItem("isDemo", "true")
    } else {
        sessionStorage.removeItem("isDemo")
    }

    const previewBallotStyles = buildPreviewBallotStyles(uploaded, language).map((bs) =>
        finalizeUploadedBallotStyle(bs, {isDemo, confirmCast, languages: uploaded.languages})
    )

    previewBallotStyles.forEach((ballotStyle, index) => {
        store.dispatch(setBallotStyle(ballotStyle))
        store.dispatch(setElection(toUploadedElection(ballotStyle, EVotingStatus.OPEN)))
        void index
    })

    const primary = previewBallotStyles[0]

    store.dispatch(
        setElectionEvent({
            id: primary.election_event_id,
            tenant_id: primary.tenant_id,
            name: "Uploaded election event",
            description: "",
            presentation: primary.ballot_eml.election_event_presentation,
            status: openStatus(EVotingStatus.OPEN) as unknown as string,
        })
    )

    previewBallotStyles.forEach((ballotStyle) => {
        if (ballotStyle.ballot_eml.contests.length === 0) {
            console.warn(`Loc studio: election ${ballotStyle.election_id} has no contests`)
        }
    })

    const selectionMode = isVoteRouteScene(sceneId)
        ? selectionModeForScene(sceneId, variantId)
        : "first"

    store.dispatch(resetBallotSelection({ballotStyle: primary, force: true}))
    store.dispatch(
        setBallotSelection({
            ballotStyle: primary,
            ballotSelection: selectionFor(primary, selectionMode),
        })
    )

    store.dispatch(
        setConfirmationScreenData({
            electionId: primary.election_id,
            confirmationScreenData: {
                ballotId: STUDIO_BALLOT_ID,
                isDemo,
            },
        })
    )

    try {
        const {encryptBallotSelection, hashBallot} = provideBallotService()
        const reviewSelection =
            selectionMode === "none" || selectionMode === "blank" || selectionMode === "invalid"
                ? selectionFor(primary, "first")
                : selectionFor(primary, selectionMode)
        const encoded = encryptBallotSelection(reviewSelection, primary.ballot_eml)
        const ballotId = hashBallot(encoded)
        store.dispatch(
            setAuditableBallot({
                electionId: primary.election_id,
                auditableBallot: encoded,
            })
        )
        if (ballotId) {
            store.dispatch(
                setConfirmationScreenData({
                    electionId: primary.election_id,
                    confirmationScreenData: {
                        ballotId,
                        isDemo,
                    },
                })
            )
        }
    } catch (error) {
        console.error("Loc studio could not encode the uploaded ballot", error)
    }

    return store
}

export type UploadedStudioStore = ReturnType<typeof createStudioStoreFromUpload>
