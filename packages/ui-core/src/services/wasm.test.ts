// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {afterEach, beforeEach, expect, it, jest} from "@jest/globals"
import * as backend from "sequent-core"
import * as adapter from "./wasm"
import * as publicApi from "../index"
import {
    auditableBallot,
    ballotStyle,
    candidate,
    contest,
    decodedContest,
} from "../../tests/fixtures"
import {ICountingAlgorithm} from "../types/CoreTypes"
import {CandidatesOrder} from "../types/ContestPresentation"
import {ContestsOrder} from "../types/ElectionPresentation"
import {ElectionsOrder} from "../types/ElectionEventPresentation"

jest.mock("sequent-core", () => {
    const functions = [
        "sort_elections_list_js",
        "sort_contests_list_js",
        "sort_candidates_list_js",
        "is_eligible_acclaimed_candidate_js",
        "is_preferential_js",
        "to_hashable_ballot_js",
        "to_hashable_multi_ballot_js",
        "hash_auditable_ballot_js",
        "hash_auditable_multi_ballot_js",
        "encrypt_decoded_contest_js",
        "encrypt_decoded_multi_contest_js",
        "sign_hashable_ballot_with_ephemeral_voter_signing_key_js",
        "sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js",
        "test_contest_reencoding_js",
        "test_multi_contest_reencoding_js",
        "get_write_in_available_characters_js",
        "decode_auditable_ballot_js",
        "decode_auditable_multi_ballot_js",
        "check_is_blank_js",
        "verify_ballot_signature_js",
        "verify_multi_ballot_signature_js",
        "check_voting_not_allowed_next",
        "check_voting_error_dialog",
        "get_layout_properties_from_contest_js",
        "get_candidate_points_js",
        "generate_sample_auditable_ballot_js",
        "get_default_duplicated_rank_policy_js",
        "get_default_preference_gaps_policy_js",
        "get_default_consolidated_report_policy_js",
        "get_default_language_detection_policy_js",
        "get_default_decline_to_vote_policy_js",
        "get_default_blank_ballots_policy_js",
        "get_default_voting_screen_back_policy_js",
        "get_voting_screen_back_policy_values_js",
        "set_hooks",
    ]
    return {
        __esModule: true,
        default: jest.fn(),
        ...Object.fromEntries(functions.map((name) => [name, jest.fn()])),
    }
})

beforeEach(() => {
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "error").mockImplementation(() => undefined)
})
afterEach(() => {
    jest.restoreAllMocks()
    jest.resetAllMocks()
})

const decoded = decodedContest({choices: [{id: "candidate-1", selected: 0}]})
const multiBallot = {...auditableBallot, contests: "synthetic-multi-contest"}
const option = candidate("candidate-1")
const question = contest({candidates: [option]})
const election = {
    id: "election-1",
    tenant_id: "tenant-1",
    election_event_id: "event-1",
    contests: [question],
    image_document_id: "image-1",
}

it("exports the shared runtime contracts through the package entry point", () => {
    expect(publicApi.initCore).toBe(adapter.initCore)
    expect(publicApi.hashBallot).toBe(adapter.hashBallot)
    expect(publicApi.IInvalidVotePosition).toEqual({TOP: "top", BOTTOM: "bottom"})
    expect(publicApi.EEarlyVotingPolicy).toEqual({
        ALLOW_EARLY_VOTING: "allow_early_voting",
        NO_EARLY_VOTING: "no_early_voting",
    })
    expect(publicApi.REALM_ATTR_VOTER_CERTIFICATE_POLICY).toBe("voter-certificate-policy")
    expect(publicApi.getImageUrl(candidate("without-image"))).toBeUndefined()
})

interface AdapterCase {
    name: string
    run: () => unknown
    backend: (...args: unknown[]) => unknown
    args: unknown[]
    result: unknown
    failure?: "null"
}

// This table tests the JS/WASM contract: argument order, returned values and
// failure policy. It makes no claim about mocked cryptography; real WASM runs
// separately in the browser integration suite.
const cases: AdapterCase[] = [
    {
        name: "election ordering",
        run: () => adapter.sortElectionList([election], ElectionsOrder.CUSTOM, false),
        backend: backend.sort_elections_list_js,
        args: [[election], ElectionsOrder.CUSTOM, false],
        result: [election],
    },
    {
        name: "contest ordering",
        run: () => adapter.sortContestList([question], ContestsOrder.CUSTOM, true),
        backend: backend.sort_contests_list_js,
        args: [[question], ContestsOrder.CUSTOM, true],
        result: [question],
    },
    {
        name: "candidate ordering",
        run: () => adapter.sortCandidatesInContest([option], CandidatesOrder.ALPHABETICAL, false),
        backend: backend.sort_candidates_list_js,
        args: [[option], CandidatesOrder.ALPHABETICAL, false],
        result: [option],
    },
    {
        name: "acclaimed eligibility",
        run: () => adapter.isEligibleAcclaimedCandidate(option),
        backend: backend.is_eligible_acclaimed_candidate_js,
        args: [option],
        result: true,
    },
    {
        name: "preferential algorithm",
        run: () => adapter.isPreferential(ICountingAlgorithm.INSTANT_RUNOFF),
        backend: backend.is_preferential_js,
        args: [ICountingAlgorithm.INSTANT_RUNOFF],
        result: true,
    },
    {
        name: "hashable ballot",
        run: () => adapter.toHashableBallot(auditableBallot),
        backend: backend.to_hashable_ballot_js,
        args: [auditableBallot],
        result: {version: 1, config: "canonical", contests: []},
    },
    {
        name: "hashable multi-ballot",
        run: () => adapter.toHashableMultiBallot(multiBallot),
        backend: backend.to_hashable_multi_ballot_js,
        args: [multiBallot],
        result: {version: 1, config: "canonical", contests: "encoded"},
    },
    {
        name: "ballot hash",
        run: () => adapter.hashBallot(auditableBallot),
        backend: backend.hash_auditable_ballot_js,
        args: [auditableBallot],
        result: "digest",
    },
    {
        name: "multi-ballot hash",
        run: () => adapter.hashMultiBallot(multiBallot),
        backend: backend.hash_auditable_multi_ballot_js,
        args: [multiBallot],
        result: "multi-digest",
    },
    {
        name: "hashBallot512 alias",
        run: () => adapter.hashBallot512(auditableBallot),
        backend: backend.hash_auditable_ballot_js,
        args: [auditableBallot],
        result: "digest",
    },
    {
        name: "ballot encryption",
        run: () => adapter.encryptBallotSelection([decoded], ballotStyle),
        backend: backend.encrypt_decoded_contest_js,
        args: [[decoded], ballotStyle],
        result: auditableBallot,
    },
    {
        name: "multi-ballot encryption",
        run: () => adapter.encryptMultiBallotSelection([decoded], ballotStyle),
        backend: backend.encrypt_decoded_multi_contest_js,
        args: [[decoded], ballotStyle],
        result: multiBallot,
    },
    {
        name: "ballot signing",
        run: () => adapter.signHashableBallot("ballot-1", "election-1", auditableBallot),
        backend: backend.sign_hashable_ballot_with_ephemeral_voter_signing_key_js,
        args: ["ballot-1", "election-1", auditableBallot],
        result: {public_key: "key", signature: "signature"},
    },
    {
        name: "multi-ballot signing",
        run: () => adapter.signHashableMultiBallot("ballot-1", "election-1", multiBallot),
        backend: backend.sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js,
        args: ["ballot-1", "election-1", multiBallot],
        result: {public_key: "key", signature: "signature"},
    },
    {
        name: "multi-contest interpretation",
        run: () => adapter.interpretMultiContestSelection([decoded], ballotStyle),
        backend: backend.test_multi_contest_reencoding_js,
        args: [[decoded], ballotStyle],
        result: [decoded],
    },
    {
        name: "write-in capacity",
        run: () => adapter.getWriteInAvailableCharacters(decoded, ballotStyle),
        backend: backend.get_write_in_available_characters_js,
        args: [decoded, ballotStyle],
        result: 40,
    },
    {
        name: "ballot decoding",
        run: () => adapter.decodeAuditableBallot(auditableBallot),
        backend: backend.decode_auditable_ballot_js,
        args: [auditableBallot],
        result: [decoded],
    },
    {
        name: "multi-ballot decoding",
        run: () => adapter.decodeAuditableMultiBallot(multiBallot),
        backend: backend.decode_auditable_multi_ballot_js,
        args: [multiBallot],
        result: [decoded],
    },
    {
        name: "blank detection",
        run: () => adapter.checkIsBlank(decoded),
        backend: backend.check_is_blank_js,
        args: [decoded],
        result: false,
        failure: "null",
    },
    {
        name: "ballot signature verification",
        run: () => adapter.verifyBallotSignature("ballot-1", "election-1", auditableBallot),
        backend: backend.verify_ballot_signature_js,
        args: ["ballot-1", "election-1", auditableBallot],
        result: false,
    },
    {
        name: "multi-ballot signature verification",
        run: () => adapter.verifyMultiBallotSignature("ballot-1", "election-1", multiBallot),
        backend: backend.verify_multi_ballot_signature_js,
        args: ["ballot-1", "election-1", multiBallot],
        result: true,
    },
    {
        name: "continue permission",
        run: () => adapter.check_voting_not_allowed_next_bool([question], {"contest-1": decoded}),
        backend: backend.check_voting_not_allowed_next,
        args: [[question], {"contest-1": decoded}],
        result: false,
    },
    {
        name: "voting warning",
        run: () => adapter.check_voting_error_dialog_bool(undefined, {}),
        backend: backend.check_voting_error_dialog,
        args: [undefined, {}],
        result: true,
    },
    {
        name: "layout",
        run: () => adapter.getLayoutProperties(question),
        backend: backend.get_layout_properties_from_contest_js,
        args: [question],
        result: {columns: 2},
        failure: "null",
    },
    {
        name: "candidate points",
        run: () => adapter.getPoints(question, decoded.choices[0]),
        backend: backend.get_candidate_points_js,
        args: [question, decoded.choices[0]],
        result: 3,
        failure: "null",
    },
    {
        name: "sample ballot",
        run: adapter.generateSampleAuditableBallot,
        backend: backend.generate_sample_auditable_ballot_js,
        args: [],
        result: auditableBallot,
        failure: "null",
    },
    {
        name: "duplicate-rank policy",
        run: adapter.getDefaultDuplicatedRankPolicy,
        backend: backend.get_default_duplicated_rank_policy_js,
        args: [],
        result: "allowed-warn-and-dialog",
    },
    {
        name: "preference-gap policy",
        run: adapter.getDefaultPreferenceGapsPolicy,
        backend: backend.get_default_preference_gaps_policy_js,
        args: [],
        result: "allowed-warn-and-dialog",
    },
    {
        name: "consolidated-report policy",
        run: adapter.getDefaultConsolidatedReportPolicy,
        backend: backend.get_default_consolidated_report_policy_js,
        args: [],
        result: "disabled",
    },
    {
        name: "language policy",
        run: adapter.getDefaultLanguageDetectionPolicy,
        backend: backend.get_default_language_detection_policy_js,
        args: [],
        result: "force-default",
    },
    {
        name: "decline-to-vote policy",
        run: adapter.getDefaultDeclineToVotePolicy,
        backend: backend.get_default_decline_to_vote_policy_js,
        args: [],
        result: "disabled",
    },
    {
        name: "blank-ballot policy",
        run: adapter.getDefaultBlankBallotsPolicy,
        backend: backend.get_default_blank_ballots_policy_js,
        args: [],
        result: "disabled",
    },
    {
        name: "back-button policy",
        run: adapter.getDefaultVotingScreenBackPolicy,
        backend: backend.get_default_voting_screen_back_policy_js,
        args: [],
        result: "allowed",
    },
    {
        name: "back-button policy choices",
        run: adapter.getVotingScreenBackPolicyValues,
        backend: backend.get_voting_screen_back_policy_values_js,
        args: [],
        result: ["allowed", "disabled"],
    },
]

it.each(cases)(
    "preserves the $name adapter contract",
    ({run, backend: implementation, args, result}) => {
        const call = jest.mocked(implementation).mockReturnValue(result)
        expect(run()).toBe(result)
        expect(call).toHaveBeenCalledTimes(1)
        expect(call).toHaveBeenCalledWith(...args)
    }
)

it.each(cases)("preserves the $name failure policy", ({run, backend: implementation, failure}) => {
    const error = new Error("synthetic WASM error")
    jest.mocked(implementation).mockImplementation(() => {
        throw error
    })
    if (failure === "null") expect(run()).toBeNull()
    else expect(run).toThrow(error)
})

it("does not invoke WASM for empty collections or an absent counting algorithm", () => {
    expect(adapter.sortElectionList([])).toEqual([])
    expect(adapter.sortContestList([])).toEqual([])
    expect(adapter.sortCandidatesInContest([])).toEqual([])
    expect(adapter.isPreferential()).toBe(false)
    expect(backend.sort_elections_list_js).not.toHaveBeenCalled()
    expect(backend.sort_contests_list_js).not.toHaveBeenCalled()
    expect(backend.sort_candidates_list_js).not.toHaveBeenCalled()
    expect(backend.is_preferential_js).not.toHaveBeenCalled()
})

it("interprets each contest in order and refuses partial results on failure", () => {
    const second = {...decoded, contest_id: "contest-2"}
    const interpreted = {...decoded, choices: []}
    const call = jest
        .mocked(backend.test_contest_reencoding_js)
        .mockReturnValueOnce(interpreted)
        .mockReturnValueOnce(second)
    expect(adapter.interpretContestSelection([decoded, second], ballotStyle)).toEqual([
        interpreted,
        second,
    ])
    expect(call).toHaveBeenNthCalledWith(1, decoded, ballotStyle)
    expect(call).toHaveBeenNthCalledWith(2, second, ballotStyle)
    const error = new Error("invalid second contest")
    call.mockReturnValueOnce(interpreted).mockImplementationOnce(() => {
        throw error
    })
    expect(() => adapter.interpretContestSelection([decoded, second], ballotStyle)).toThrow(error)
})

it.each(["ready", "failed"])(
    "shares one initialization promise when loading is %s",
    async (outcome) => {
        await jest.isolateModulesAsync(async () => {
            const isolatedBackend = await import("sequent-core")
            const isolatedAdapter = await import("./wasm")
            const error = new Error("synthetic initialization failure")
            const initialize = jest.mocked(isolatedBackend.default)
            if (outcome === "ready")
                initialize.mockResolvedValue(
                    {} as Awaited<ReturnType<typeof isolatedBackend.default>>
                )
            else initialize.mockRejectedValue(error)
            const first = isolatedAdapter.initCore()
            const second = isolatedAdapter.initCore()
            expect(first).toBe(second)
            if (outcome === "ready") {
                await expect(first).resolves.toBeUndefined()
                expect(isolatedBackend.set_hooks).toHaveBeenCalledTimes(1)
            } else {
                await expect(first).rejects.toBe(error)
                expect(isolatedBackend.set_hooks).not.toHaveBeenCalled()
            }
            expect(initialize).toHaveBeenCalledTimes(1)
        })
    }
)

it("preserves zero points while using null for an unavailable result", () => {
    const points = jest.mocked(backend.get_candidate_points_js)
    points.mockReturnValueOnce(0).mockReturnValueOnce(undefined)
    expect(adapter.getPoints(question, decoded.choices[0])).toBe(0)
    expect(adapter.getPoints(question, decoded.choices[0])).toBeNull()
    jest.mocked(backend.get_layout_properties_from_contest_js).mockReturnValue(undefined)
    expect(adapter.getLayoutProperties(question)).toBeNull()
})
