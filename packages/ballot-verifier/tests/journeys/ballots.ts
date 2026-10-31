// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {loadCore} from "@sequentech/ui-test-kit/wasm/node"
import {encryptAndSign} from "../fixtures/signedBallot"

/** Produces real encrypted and signed input independently of the verifier's wrappers. */
export async function signedBallot(multiple = false) {
    return encryptAndSign(await loadCore(), multiple)
}
