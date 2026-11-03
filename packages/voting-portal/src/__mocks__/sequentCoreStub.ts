// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// sequent-core is a wasm-pack `--target web` package that loads its binary
// through `import.meta.url`, which jest's CommonJS transform cannot load. It is
// mapped to this file in jest.config.cjs. Only the slate calls are stood in for;
// their rules are Rust's and are pinned by `cargo test -p sequent-core`, so a
// change there has to be mirrored here.

const SequentCoreLibInit = async (): Promise<void> => undefined
export default SequentCoreLibInit

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
 * `slates::coverage` derives it.
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
    write_in_text?: string | null
}
interface StubDecodedContest {
    contest_id: string
    is_explicit_invalid: boolean
    invalid_errors: Array<unknown>
    invalid_alerts: Array<unknown>
    choices: Array<StubChoice>
}
interface StubContest {
    id: string
    max_votes: number
    is_acclaimed?: boolean | null
    candidates: Array<{id: string; presentation?: {is_explicit_invalid?: boolean} | null}>
}
interface StubContestChange {
    contest_id: string
    added: Array<string>
    removed: Array<string>
}

const slateProblem = (code: string, contestId: string, message: string) => ({
    severity: "error",
    code,
    path: `members["${contestId}"]`,
    message,
})

/** The marks of a contest, with its explicit invalid option when it is marked. */
const selectedIds = (contest: StubContest, entry: StubDecodedContest): Array<string> => {
    const ids = entry.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
    const invalidCandidate = contest.candidates.find(
        (candidate) => candidate.presentation?.is_explicit_invalid
    )
    if (entry.is_explicit_invalid && invalidCandidate && !ids.includes(invalidCandidate.id)) {
        ids.push(invalidCandidate.id)
    }
    return ids
}

const markOnly = (entry: StubDecodedContest, memberIds: Set<string>): void => {
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

/**
 * Choosing a slate: every contest of the ballot ends with exactly the slate's
 * candidates, blank where it has none. Acclaimed contests and contests outside
 * the ballot keep their entries. It carries the rule because the chooser's
 * confirmation and the reducer are only observable through its result.
 */
export const apply_slate_js = (slateJson: unknown, contestsJson: unknown, currentJson: unknown) => {
    const slate = slateJson as {id: string; members: Record<string, Array<string>>}
    const contests = contestsJson as Array<StubContest>
    const selection = structuredClone(currentJson) as Array<StubDecodedContest>
    const problems: Array<ReturnType<typeof slateProblem>> = []
    const changes: Array<StubContestChange> = []
    let covered = 0

    for (const contest of contests.filter((entry) => !entry.is_acclaimed)) {
        const members = slate.members[contest.id]
        if (members === undefined) {
            const entry = selection.find((item) => item.contest_id === contest.id)
            const removed = entry ? selectedIds(contest, entry) : []
            if (entry && removed.length > 0) {
                markOnly(entry, new Set())
                changes.push({contest_id: contest.id, added: [], removed})
            }
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
        const added = members.filter((member) => !wasSelected.has(member))
        const removed = selectedIds(contest, entry).filter((id) => !memberIds.has(id))
        markOnly(entry, memberIds)
        if (added.length > 0 || removed.length > 0) {
            changes.push({contest_id: contest.id, added, removed})
        }
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

interface IStubSortableCandidate {
    name?: string | null
    alias?: string | null
    presentation?: {sort_order?: number | null} | null
}

/** The candidate order of `sort_candidates_list_js`, without the shuffle. */
export const sort_candidates_list_js = <Candidate extends IStubSortableCandidate>(
    candidates: Array<Candidate>,
    order?: string
): Array<Candidate> => {
    const sorted = [...candidates]
    if (order === "alphabetical") {
        const key = (candidate: Candidate) =>
            (candidate.alias ?? candidate.name ?? "").toLowerCase()
        sorted.sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : 0))
    } else if (order === "custom") {
        const key = (candidate: Candidate) => candidate.presentation?.sort_order ?? -1
        sorted.sort((a, b) => key(a) - key(b))
    }
    return sorted
}
