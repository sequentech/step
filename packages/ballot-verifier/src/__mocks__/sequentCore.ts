// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {jest} from "@jest/globals"

// Stand-in for the sequent-core WASM bindings (mapped in jest.config.cjs).
// Ballot operations have no default answer: a test records what the real
// module returns for its fixtures, so an unexpected call fails loudly.
type Binding = (...args: never[]) => unknown

const noAnswer = (name: string) => () => {
    throw new Error(`sequent-core ${name} has no recorded answer in this test`)
}
// These throw until the real module has loaded, and ui-core falls back to
// plain language codes, as it does while the app starts.
const notLoaded = () => {
    throw new Error("sequent-core is not loaded")
}
// Fixtures list contests and candidates in their display order already.
const keepOrder = <T>(items: T): T => items

const defaults = {
    init: () => Promise.resolve(),
    set_hooks: () => undefined,
    iso_639_2t_to_bcp47_js: notLoaded,
    locale_to_internal_language_code_js: notLoaded,
    sort_elections_list_js: keepOrder,
    sort_contests_list_js: keepOrder,
    sort_candidates_list_js: keepOrder,
    is_preferential_js: () => false,
    is_eligible_acclaimed_candidate_js: () => true,
    check_is_blank_js: () => false,
    get_layout_properties_from_contest_js: () => undefined,
    get_candidate_points_js: () => undefined,
    decode_auditable_ballot_js: noAnswer("decode_auditable_ballot_js"),
    decode_auditable_multi_ballot_js: noAnswer("decode_auditable_multi_ballot_js"),
    hash_auditable_ballot_js: noAnswer("hash_auditable_ballot_js"),
    hash_auditable_multi_ballot_js: noAnswer("hash_auditable_multi_ballot_js"),
    verify_ballot_signature_js: noAnswer("verify_ballot_signature_js"),
    verify_multi_ballot_signature_js: noAnswer("verify_multi_ballot_signature_js"),
    generate_sample_auditable_ballot_js: noAnswer("generate_sample_auditable_ballot_js"),
} satisfies Record<string, Binding>

type Name = keyof typeof defaults

export const sequentCore = Object.fromEntries(
    Object.keys(defaults).map((name) => [name, jest.fn<Binding>()])
) as {[Key in Name]: jest.Mock<Binding>}

/** Forgets recorded answers and calls; run before each test. */
export const resetSequentCore = () => {
    for (const name of Object.keys(defaults) as Name[]) {
        sequentCore[name].mockReset().mockImplementation(defaults[name])
    }
}
resetSequentCore()

export default sequentCore.init
export const {
    set_hooks,
    iso_639_2t_to_bcp47_js,
    locale_to_internal_language_code_js,
    sort_elections_list_js,
    sort_contests_list_js,
    sort_candidates_list_js,
    is_preferential_js,
    is_eligible_acclaimed_candidate_js,
    check_is_blank_js,
    get_layout_properties_from_contest_js,
    get_candidate_points_js,
    decode_auditable_ballot_js,
    decode_auditable_multi_ballot_js,
    hash_auditable_ballot_js,
    hash_auditable_multi_ballot_js,
    verify_ballot_signature_js,
    verify_multi_ballot_signature_js,
    generate_sample_auditable_ballot_js,
} = sequentCore
