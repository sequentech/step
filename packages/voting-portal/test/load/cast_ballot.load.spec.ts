// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as fs from "fs"
import {test} from "@playwright/test"
import {castBallotAsVoter} from "./flow"

// One test per voter: Playwright distributes them across --workers and
// reports a per-voter outcome, which run-online-load-test.sh turns into
// results.csv. Two ways to feed voters in:
//
// - LOAD_TEST_MANIFEST: a JSON array of voter records, rendered from the
//   Stage-1 voters CSV by run-online-load-test.sh (the load-test mode). Every
//   CSV column is carried through as a field (not just username/password):
//   the login form's fields depend on the realm's configured
//   match-attributes (e.g. dateOfBirth alongside — or instead of — a voter-id
//   username), and flow.ts fills in whichever of these fields the form asks
//   for.
// - VOTER_USERNAME/VOTER_PASSWORD (and optionally VOTER_DATE_OF_BIRTH): a
//   single voter's credentials, for running one vote directly (smoke test,
//   selector debugging):
//
//     LOGIN_URL=http://127.0.0.1:3000/tenant/<t>/event/<e>/login \
//     VOTER_USERNAME=100 VOTER_PASSWORD=123456 VOTER_DATE_OF_BIRTH=1951-04-22 \
//     yarn playwright test --config playwright.load.config.ts --headed

interface Voter {
    [field: string]: string
}

function loadVoters(): Voter[] {
    const manifestPath = process.env.LOAD_TEST_MANIFEST
    if (manifestPath) {
        const voters = JSON.parse(fs.readFileSync(manifestPath, "utf-8")) as Voter[]
        if (!Array.isArray(voters) || voters.length === 0) {
            throw new Error(`No voters in manifest ${manifestPath}`)
        }
        return voters
    }
    const {
        VOTER_USERNAME: username,
        VOTER_PASSWORD: password,
        VOTER_DATE_OF_BIRTH: dateOfBirth,
    } = process.env
    if (username && password) {
        return [{username, password, ...(dateOfBirth ? {dateOfBirth} : {})}]
    }
    throw new Error(
        "Set LOAD_TEST_MANIFEST (normally done by run-online-load-test.sh) or VOTER_USERNAME/VOTER_PASSWORD"
    )
}

const loginUrl = process.env.LOGIN_URL
if (!loginUrl) {
    throw new Error(
        "LOGIN_URL is required: the voting portal login URL, e.g. http://127.0.0.1:3000/tenant/<tenant_id>/event/<election_event_id>/login"
    )
}
const castCsvPath = process.env.LOAD_TEST_CAST_CSV
const candidatesPattern = process.env.CANDIDATES_PATTERN || undefined

for (const voter of loadVoters()) {
    test(`voter ${voter.username}`, async ({page}) => {
        const startedAt = Date.now()
        const ballotIds = await castBallotAsVoter(page, {
            loginUrl,
            credentials: voter,
            candidatesPattern,
        })
        if (castCsvPath) {
            fs.appendFileSync(
                castCsvPath,
                `${voter.username},${Date.now() - startedAt},${ballotIds.join("+")}\n`
            )
        }
    })
}
