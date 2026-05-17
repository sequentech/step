// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import init, {
    decode_bigint_to_decoded_vote_contest,
    decrypt_ballot_content,
    encode_ballot,
    generate_keypair,
    get_sample_ballots_json,
    get_sample_contest_json,
    get_sample_decoded_vote_contest_json,
    tally_plaintext_ballots,
} from "velvet-wasm"
import initSequentCore, {
    encrypt_decoded_contest_js,
    to_hashable_ballot_js,
} from "sequent-core"

// Single shared init promise so callers can `await ensureWasm()` from
// anywhere without re-initialising the module.
let initPromise: Promise<void> | null = null
export function ensureWasm(): Promise<void> {
    if (!initPromise) {
        initPromise = init().then(() => undefined)
    }
    return initPromise
}

// Parallel init promise for sequent-core's wasm-bindgen surface. The
// booth's `WasmWrapper` already initialises it when a lifted screen
// mounts, but `/pipeline` lives outside that provider tree and the
// only callers there are the encrypt-stage helpers below — so they
// initialise the module on demand. Idempotent and shared.
let sequentCoreInitPromise: Promise<void> | null = null
function ensureSequentCoreWasm(): Promise<void> {
    if (!sequentCoreInitPromise) {
        sequentCoreInitPromise = initSequentCore().then(() => undefined)
    }
    return sequentCoreInitPromise
}

export interface Fixtures {
    contestJson: string
    ballotsJson: string
    decodedVoteContestJson: string
}

/// Pretty-prints the in-tree fixtures so the textareas start with
/// readable, hand-editable JSON instead of one giant line.
export async function getFixtures(): Promise<Fixtures> {
    await ensureWasm()
    const contestRaw = get_sample_contest_json()
    const ballotsRaw = get_sample_ballots_json()
    const decodedRaw = get_sample_decoded_vote_contest_json()
    return {
        contestJson: JSON.stringify(JSON.parse(contestRaw), null, 2),
        ballotsJson: JSON.stringify(JSON.parse(ballotsRaw), null, 2),
        decodedVoteContestJson: JSON.stringify(
            JSON.parse(decodedRaw),
            null,
            2
        ),
    }
}

export async function runTally(
    contestJson: string,
    ballots: string[]
): Promise<unknown> {
    await ensureWasm()
    const resultJson = tally_plaintext_ballots(contestJson, ballots)
    return JSON.parse(resultJson)
}

export async function encodeBallot(
    contestJson: string,
    decodedVoteContestJson: string
): Promise<string> {
    await ensureWasm()
    return encode_ballot(contestJson, decodedVoteContestJson)
}

/** Generate a fresh Ristretto ElGamal keypair via velvet-wasm. The two
 *  base64-no-pad strings match the format used by sequent-core (pk) and
 *  borsh-serialised `PrivateKey<RistrettoCtx>` (sk). */
export async function generateKeypair(): Promise<{
    pkB64: string
    skB64: string
}> {
    await ensureWasm()
    const raw = JSON.parse(generate_keypair()) as {
        pk_b64: string
        sk_b64: string
    }
    return {pkB64: raw.pk_b64, skB64: raw.sk_b64}
}

/** Decrypt one contest out of an `AuditableBallot` JSON (the value the
 *  portal stores in `castVote.content`) and return the resulting
 *  plaintext as a decimal-`BigUint` string — the exact same byte
 *  `encodeBallot` would produce from the matching `DecodedVoteContest`. */
export async function decryptBallotContent(
    contentJson: string,
    skB64: string,
    contestId: string
): Promise<string> {
    await ensureWasm()
    return decrypt_ballot_content(contentJson, skB64, contestId)
}

/** Encrypt one `DecodedVoteContest` selection under the workbench
 *  public key. Returns a JSON envelope with a top-level
 *  `contests: ["<base64>", ...]` array — same shape the portal
 *  stores in `castVote.content` — so it can be fed directly back
 *  into `decryptBallotContent`.
 *
 *  Internally this is the *production* path: we call sequent-core's
 *  canonical `encrypt_decoded_contest_js` to produce an
 *  `AuditableBallot`, then `to_hashable_ballot_js` to derive the
 *  signed-hashable form the booth would submit. The workbench keeps
 *  its `(contestJson, decodedVoteContestJson, pkB64)` calling shape
 *  for backwards-compat with `BallotPipeline`'s seed/row model, and
 *  this helper synthesizes the `BallotStyle` envelope sequent-core
 *  needs (single contest, workbench-generated pk as `public_key`,
 *  empty strings for the bookkeeping fields encrypt does not read). */
export async function encryptDecodedVoteContest(
    contestJson: string,
    decodedVoteContestJson: string,
    pkB64: string
): Promise<string> {
    await ensureSequentCoreWasm()
    const contest = JSON.parse(contestJson) as {id: string} & Record<
        string,
        unknown
    >
    const decoded = JSON.parse(decodedVoteContestJson) as Record<
        string,
        unknown
    >
    // Minimal BallotStyle wrapper. encrypt_decoded_contest reads only
    // `contests` and `public_key`; the other string fields are
    // required by serde but unused, so empty values are safe.
    const ballotStyle = {
        id: "",
        tenant_id: "",
        election_event_id: "",
        election_id: "",
        num_allowed_revotes: null,
        description: null,
        public_key: {public_key: pkB64, is_demo: false},
        area_id: "",
        area_presentation: null,
        contests: [contest],
        election_event_presentation: null,
        election_presentation: null,
        election_dates: null,
        election_event_annotations: null,
        election_annotations: null,
        area_annotations: null,
    }
    const auditable = encrypt_decoded_contest_js([decoded], ballotStyle)
    const hashable = to_hashable_ballot_js(auditable)
    return JSON.stringify(hashable)
}

/** Decode a decimal-`BigUint` encoded plaintext back into a structured
 *  `DecodedVoteContest`. Inverse of `encodeBallot`. */
export async function decodeBigIntToDecodedVoteContest(
    contestJson: string,
    bigintStr: string
): Promise<string> {
    await ensureWasm()
    return decode_bigint_to_decoded_vote_contest(contestJson, bigintStr)
}
