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
export const iso_639_2t_to_bcp47_js = (code: string): string => code
export const locale_to_internal_language_code_js = (locale: string): string => locale
