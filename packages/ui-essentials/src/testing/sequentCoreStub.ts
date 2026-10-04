// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The WebAssembly boundary, stood in for so a ballot can be mounted in jsdom.
 *
 * **What this is and is not.** `sequent-core` is a wasm-pack `--target web`
 * package: it resolves its binary through `new URL("index_bg.wasm",
 * import.meta.url)`, which jest's CommonJS transform cannot load. So it is
 * mapped to this file for every test (see `moduleNameMapper` in
 * `jest.config.cjs`).
 *
 * The choice of *where* to stub matters, and this is the deliberate one. The
 * alternative was to mock `@sequentech/ui-core`, which would have replaced its
 * real, pure logic — `categorizeCandidates`, `getCheckableOptions`,
 * `checkIsRadioSelection`, the category shuffling — with approximations, and
 * those are exactly the decisions a ballot's layout turns on. Stubbing one
 * module lower means every pure line of `ui-core` runs for real and only the
 * genuinely-compiled calls are substituted.
 *
 * **What is therefore *not* pinned by any test using this file**: the ordering
 * rule inside `sort_candidates_list_js`, blank detection, the preferential
 * predicate, and the write-in character budget. Those live in Rust and are
 * covered by `cargo test -p sequent-core`, where they can be checked against the
 * real encoder. A test here that claimed to verify random-order fairness would
 * be verifying this file. Said plainly because the temptation is to read a green
 * suite as covering more than it does.
 *
 * The stand-ins are order-preserving and total rather than clever: they answer
 * in the shape the caller expects, deterministically, so that what a test
 * observes is the component's own branching.
 */

/** wasm-pack's init. Nothing to initialise; resolve so `initCore` completes. */
const SequentCoreLibInit = async (): Promise<void> => undefined
export default SequentCoreLibInit

export const set_hooks = (): void => undefined

/**
 * Candidates back in the order they arrived.
 *
 * Identity, not a sort. A test that wants to observe order sets it up in the
 * fixture, which keeps "what order did the ballot draw" a property of the test
 * rather than of this stub. `applyRandom` is accepted and ignored — randomising
 * here would make every assertion flaky for no gain.
 */
export const sort_candidates_list_js = <Item>(
    candidates: Array<Item>,
    _order?: unknown,
    _applyRandom?: unknown
): Array<Item> => candidates

export const sort_elections_list_js = <Item>(list: Array<Item>): Array<Item> => list
export const sort_contests_list_js = <Item>(list: Array<Item>): Array<Item> => list

/**
 * Preferential when the algorithm's name says so.
 *
 * The real predicate is a match over the platform's counting algorithms. This
 * reads the name, which is enough for a component that only asks "ordinals or
 * checkboxes" — and a fixture naming `instant-runoff` gets ordinals, which is
 * what a test setting that up means.
 */
export const is_preferential_js = (countingAlgorithm?: unknown): boolean => {
    const name = typeof countingAlgorithm === "string" ? countingAlgorithm : ""
    return /borda|instant.?runoff|stv|preferential|ranked/i.test(name)
}

/** Blank when nothing is selected and nothing is explicitly marked. */
export const check_is_blank_js = (contest: unknown): boolean => {
    const decoded = contest as
        | {choices?: Array<{selected: number}>; is_explicit_invalid?: boolean}
        | undefined
    if (decoded?.is_explicit_invalid === true) {
        return false
    }
    return (decoded?.choices ?? []).every((choice) => choice.selected < 0)
}

/**
 * A generous character budget.
 *
 * Returned positive so the write-in overflow warning stays off unless a test
 * asks for it, which it does by stubbing this call for that case. Zero here
 * would light the warning on every ballot with a write-in and quietly change
 * what half these tests observe.
 */
export const get_write_in_available_characters_js = (): number => 240

export const get_layout_properties_from_contest_js = (
    contest: unknown
): {columns: number; maxVotes: number} => {
    const presentation = (contest as {presentation?: {columns?: number}} | undefined)?.presentation
    return {columns: presentation?.columns ?? 1, maxVotes: 1}
}

export const check_voting_not_allowed_next = (): boolean => false
export const check_voting_error_dialog = (): boolean => false

// The rest of the surface `ui-core`'s barrel imports. Present so the module
// loads; each throws rather than lying, because a ballot render that reaches
// encryption or signing is a test that has strayed and should say so.
const outOfScope =
    (name: string) =>
    (...__: Array<unknown>): never => {
        throw new Error(
            `${name} is not stubbed: it is encryption or signing, which a ballot render does not reach. ` +
                `If a test needs it, it needs the real core, not this file.`
        )
    }

export const generate_sample_auditable_ballot_js = outOfScope("generate_sample_auditable_ballot_js")
export const get_candidate_points_js = outOfScope("get_candidate_points_js")
export const decode_auditable_ballot_js = outOfScope("decode_auditable_ballot_js")
export const decode_auditable_multi_ballot_js = outOfScope("decode_auditable_multi_ballot_js")
export const to_hashable_ballot_js = outOfScope("to_hashable_ballot_js")
export const to_hashable_multi_ballot_js = outOfScope("to_hashable_multi_ballot_js")
export const hash_auditable_ballot_js = outOfScope("hash_auditable_ballot_js")
export const hash_auditable_multi_ballot_js = outOfScope("hash_auditable_multi_ballot_js")
export const encrypt_decoded_contest_js = outOfScope("encrypt_decoded_contest_js")
export const encrypt_decoded_multi_contest_js = outOfScope("encrypt_decoded_multi_contest_js")
export const test_contest_reencoding_js = outOfScope("test_contest_reencoding_js")
export const test_multi_contest_reencoding_js = outOfScope("test_multi_contest_reencoding_js")
export const sign_hashable_ballot_with_ephemeral_voter_signing_key_js = outOfScope(
    "sign_hashable_ballot_with_ephemeral_voter_signing_key_js"
)
export const sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js = outOfScope(
    "sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js"
)
export const verify_ballot_signature_js = outOfScope("verify_ballot_signature_js")
export const verify_multi_ballot_signature_js = outOfScope("verify_multi_ballot_signature_js")

// Defaults the portal reads at start-up. Real values, so a component that asks
// gets something the platform would actually send.
export const get_default_consolidated_report_policy_js = (): string => "no-report"
export const get_default_language_detection_policy_js = (): string => "disabled"
export const get_default_decline_to_vote_policy_js = (): string => "not-allowed"
export const get_default_voting_screen_back_policy_js = (): string => "allowed"
export const get_voting_screen_back_policy_values_js = (): Array<string> => [
    "allowed",
    "not-allowed",
]
export const get_default_duplicated_rank_policy_js = (): string => "not-allowed"
export const get_default_preference_gaps_policy_js = (): string => "not-allowed"
/**
 * The slate configuration as written, unchecked: validation is Rust's and is
 * covered by `cargo test -p sequent-core`. Unreadable JSON throws a problem
 * list, as the real export does.
 */
export const get_ballot_style_slates_js = (ballotStyle: unknown): unknown => {
    const annotations = (
        ballotStyle as {election_annotations?: Record<string, string> | null} | undefined
    )?.election_annotations
    const text = annotations?.["sequent.slates"]
    if (text === undefined) {
        return null
    }
    try {
        return JSON.parse(text)
    } catch (error) {
        throw [
            {
                severity: "error",
                code: "unreadable",
                path: 'election_annotations["sequent.slates"]',
                message: `the slate configuration is not valid JSON: ${String(error)}`,
            },
        ]
    }
}

interface IStubCandidate {
    id: string
    presentation?: Record<string, unknown> | null
}

interface IStubContest {
    id: string
    max_votes: number
    is_acclaimed?: boolean | null
    candidates: Array<IStubCandidate>
}

const UNSELECTABLE_FLAGS = [
    "is_disabled",
    "is_explicit_blank",
    "is_explicit_invalid",
    "is_write_in",
    "is_category_list",
]

/**
 * What each slate covers of the ballot style's contests, as
 * `election_config::slates::coverage` derives it.
 */
export const get_ballot_style_slates_coverage_js = (ballotStyle: unknown): unknown => {
    const config = get_ballot_style_slates_js(ballotStyle) as {
        slates: Array<{id: string; members: Record<string, Array<string>>}>
    } | null
    if (!config) {
        return null
    }
    const contests = ((ballotStyle as {contests?: Array<IStubContest>}).contests ?? []).filter(
        (contest) => !contest.is_acclaimed
    )
    const seats = contests.reduce((total, contest) => total + contest.max_votes, 0)

    return config.slates.flatMap((slate) => {
        const covered = contests.flatMap((contest) => {
            const candidateIds = (slate.members[contest.id] ?? []).filter((candidateId) =>
                contest.candidates.some(
                    (candidate) =>
                        candidate.id === candidateId &&
                        !UNSELECTABLE_FLAGS.some((flag) => candidate.presentation?.[flag])
                )
            )
            return candidateIds.length > 0
                ? [{contest_id: contest.id, candidate_ids: candidateIds, seats: contest.max_votes}]
                : []
        })
        if (covered.length === 0) {
            return []
        }
        const uncovered = contests
            .filter((contest) => !covered.some((entry) => entry.contest_id === contest.id))
            .map((contest) => contest.id)
        const isComplete =
            uncovered.length === 0 &&
            covered.every((entry) => entry.candidate_ids.length === entry.seats)
        return [
            {
                slate_id: slate.id,
                kind: isComplete ? "complete" : "partial",
                covered,
                uncovered_contest_ids: uncovered,
                members: covered.reduce((total, entry) => total + entry.candidate_ids.length, 0),
                seats,
            },
        ]
    })
}
interface StubChoice {
    id: string
    selected: number
}
interface StubDecodedContest {
    contest_id: string
    is_explicit_invalid: boolean
    choices: Array<StubChoice>
}
interface StubContest {
    id: string
    max_votes: number
    candidates: Array<{id: string; presentation?: {is_explicit_invalid?: boolean} | null}>
}

const slateProblem = (code: string, contestId: string, message: string) => ({
    severity: "error",
    code,
    path: `members["${contestId}"]`,
    message,
})

/**
 * Choosing a slate, following `election_config::slates::selection::apply_slate`.
 *
 * Unlike the other stand-ins this one carries the rule, because the chooser's
 * confirmation and the reducer are only observable through its result. The
 * rule itself is Rust's and is pinned by `cargo test -p sequent-core`; a change
 * there has to be mirrored here.
 */
export const apply_slate_js = (slateJson: unknown, contestsJson: unknown, currentJson: unknown) => {
    const slate = slateJson as {id: string; members: Record<string, Array<string>>}
    const contests = contestsJson as Array<StubContest>
    const selection = structuredClone(currentJson) as Array<StubDecodedContest>
    const problems: Array<ReturnType<typeof slateProblem>> = []
    const changes: Array<{contest_id: string; added: Array<string>; removed: Array<string>}> = []
    let covered = 0

    for (const contest of contests) {
        const members = slate.members[contest.id]
        if (members === undefined) {
            continue
        }
        covered += 1
        const entries = selection.filter((entry) => entry.contest_id === contest.id)
        if (entries.length !== 1) {
            problems.push(
                slateProblem(
                    "dangling_reference",
                    contest.id,
                    `contest '${contest.id}' of slate '${slate.id}' is missing from the ballot selection or repeated in it`
                )
            )
            continue
        }
        const [entry] = entries
        const memberIds = new Set(members)
        const found = problems.length
        if (members.length === 0) {
            problems.push(
                slateProblem(
                    "missing_field",
                    contest.id,
                    `slate '${slate.id}' lists contest '${contest.id}' without candidates`
                )
            )
        }
        if (memberIds.size !== members.length) {
            problems.push(
                slateProblem(
                    "duplicate_id",
                    contest.id,
                    `slate '${slate.id}' lists a candidate more than once in contest '${contest.id}'`
                )
            )
        }
        if (memberIds.size > contest.max_votes) {
            problems.push(
                slateProblem(
                    "contest_arithmetic",
                    contest.id,
                    `slate '${slate.id}' has ${memberIds.size} candidates in contest '${contest.id}', which allows ${contest.max_votes}`
                )
            )
        }
        for (const member of members) {
            const isCandidate = contest.candidates.some((candidate) => candidate.id === member)
            const isChoice = entry.choices.some((choice) => choice.id === member)
            if (!isCandidate || !isChoice) {
                problems.push(
                    slateProblem(
                        "dangling_reference",
                        contest.id,
                        `candidate '${member}' of slate '${slate.id}' is not a choice of contest '${contest.id}'`
                    )
                )
            }
        }
        if (problems.length > found) {
            continue
        }

        const wasSelected = new Set(
            entry.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
        )
        const removed = entry.choices
            .filter((choice) => wasSelected.has(choice.id) && !memberIds.has(choice.id))
            .map((choice) => choice.id)
        const invalidCandidate = contest.candidates.find(
            (candidate) => candidate.presentation?.is_explicit_invalid
        )
        if (
            entry.is_explicit_invalid &&
            invalidCandidate &&
            !removed.includes(invalidCandidate.id)
        ) {
            removed.push(invalidCandidate.id)
        }
        const added = members.filter((member) => !wasSelected.has(member))
        if (added.length > 0 || removed.length > 0) {
            changes.push({contest_id: contest.id, added, removed})
        }
        Object.assign(entry, {
            is_explicit_invalid: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: entry.choices.map((choice) => ({
                id: choice.id,
                selected: memberIds.has(choice.id) ? 0 : -1,
                write_in_text: null,
            })),
        })
    }

    if (covered === 0) {
        problems.push({
            severity: "error",
            code: "missing_field",
            path: "members",
            message: `slate '${slate.id}' has no candidates on this ballot`,
        })
    }
    if (problems.length > 0) {
        throw problems
    }
    return {
        selection: selection.map((entry) => ({
            ...entry,
            is_blank_ballot: false,
            is_decline_to_vote: false,
        })),
        changes,
    }
}

export const iso_639_2t_to_bcp47_js = (code: string): string => code
export const locale_to_internal_language_code_js = (locale: string): string => locale
