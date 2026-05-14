// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import init, {
    decrypt_ballot_content,
    encode_ballot,
    generate_keypair,
    get_sample_ballots_json,
    get_sample_contest_json,
    get_sample_decoded_vote_contest_json,
    tally_plaintext_ballots,
} from "velvet-wasm"

// Single shared init promise so callers can `await ensureWasm()` from
// anywhere without re-initialising the module.
let initPromise: Promise<void> | null = null
export function ensureWasm(): Promise<void> {
    if (!initPromise) {
        initPromise = init().then(() => undefined)
    }
    return initPromise
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
