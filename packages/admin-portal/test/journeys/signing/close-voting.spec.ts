// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    EVENT_ID,
    ELECTION_ID,
    JOSE,
    LOCAL_NOTE,
    MARIA,
    POST,
    PORTUGAL,
    PORTUGAL_ID,
    SPAIN,
    SPAIN_ID,
    SigningServer,
    displayName,
    election,
    expectRole,
    fixture,
    mockSigningEvent,
    openCertificate,
    sbeiRoles,
    sealRecord,
    signInAs,
    signingDialog,
    verifyApproval,
    type SigningRequestState,
} from "./data"
import {listOf} from "../events/data"

const PUBLICATION_ID = "f2000000-0000-4000-8000-000000000001"
const REQUEST_ID = "f1000000-0000-4000-8000-000000000001"
const CODE = "7F3A-91C2"
const SUBJECT = {channels: ["ONLINE"], from: ["ONLINE=OPEN"]}

test.use({
    roles: sbeiRoles(
        "election-publish-tab",
        "publish-read",
        "election-state-write",
        "publish-stop-voting",
        "sign-close-voting"
    ),
})

/** The Post's Publish tab, whose status follows the close request. */
function mockPost(portal: AdminPortal, server: SigningServer) {
    mockSigningEvent(portal)
    const current = () =>
        election({
            voting_status:
                server.requests.get(REQUEST_ID)?.status === "executed" ? "CLOSED" : "OPEN",
        })
    portal.graphql.on(
        "sequent_backend_election",
        listOf("sequent_backend_election", () => [current()])
    )
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: [current()]}}))
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: []}}))
    portal.graphql.on(
        "sequent_backend_ballot_publication",
        listOf("sequent_backend_ballot_publication", [
            {
                id: PUBLICATION_ID,
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                election_id: ELECTION_ID,
                election_ids: [ELECTION_ID],
                is_generated: true,
                published_at: FIXED_TIME,
                created_at: FIXED_TIME,
                last_updated_at: FIXED_TIME,
                annotations: {},
                labels: {},
            },
        ])
    )
}

/** Stop voting answers the close request the route created, instead of closing. */
function guardStopVoting(
    portal: AdminPortal,
    server: SigningServer,
    seals: (state: SigningRequestState) => Array<Record<string, unknown>> = () => []
) {
    portal.graphql.on("UpdateElectionVotingStatus", ({variables}) => {
        expect(variables).toEqual({
            electionEventId: EVENT_ID,
            electionId: ELECTION_ID,
            votingStatus: "CLOSED",
            votingChannel: ["ONLINE"],
        })
        const state = server.add({
            id: REQUEST_ID,
            action: "close-voting",
            code: CODE,
            required: 2,
            requestedBy: MARIA,
            signers: [MARIA, JOSE],
            subject: SUBJECT,
            execute: (done) => sealRecord(done, seals(done)),
        })
        return {
            data: {
                update_election_voting_status: {
                    election_id: ELECTION_ID,
                    signing_request: {
                        id: REQUEST_ID,
                        code: CODE,
                        required: 2,
                        expires_at: state.expiresAt,
                    },
                },
            },
        }
    })
}

/** The request panel; the signing dialog over it hides it from the accessibility tree. */
function panel(page: Page) {
    return page.locator(".MuiDrawer-paper").filter({has: page.getByTestId("signing-status")})
}

/** Check → Certificate → Signed, with the local-signing note on each step. */
async function signWith(
    page: Page,
    person: typeof MARIA,
    expected: {count: string; next?: string}
) {
    const dialog = signingDialog(page)
    await expect(dialog).toBeVisible()
    await expect(dialog.getByText(`You are signing as ${displayName(person)}`)).toContainText(
        `${person.title}, ${POST}`
    )
    await expect(dialog.getByTestId("signing-code")).toHaveText(CODE)
    await expect(dialog.getByRole("table", {name: "Details"})).toContainText("Online")
    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    await dialog.getByRole("button", {name: "Continue", exact: true}).click()

    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    await openCertificate(page, person.certificate)
    const checks = dialog.getByRole("list", {name: "Certificate checks"})
    await expect(checks.getByRole("listitem")).toHaveCount(5)
    await expect(checks.locator('[data-ok="false"]')).toHaveCount(0)
    await expect(dialog.getByTestId("signing-certificate")).toContainText(
        fixture(person.certificate).commonName
    )
    await dialog.getByRole("button", {name: "Sign", exact: true}).click()

    await expect(dialog.getByText("Signed", {exact: true}).last()).toBeVisible()
    await expect(dialog.getByText(expected.count, {exact: true})).toBeVisible()
    if (expected.next) await expect(dialog.getByText(expected.next, {exact: true})).toBeVisible()
    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    await dialog.getByRole("button", {name: "Done", exact: true}).click()
    await expect(dialog).toHaveCount(0)
}

async function stopVoting(page: Page, portal: AdminPortal) {
    await page.goto(`${portal.origin}/sequent_backend_election/${ELECTION_ID}?lang=en`)
    await page.getByRole("button", {name: "Stop Voting", exact: true}).click()
    await page.getByRole("menuitem", {name: "Stop Online Voting", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Confirm", exact: true}).click()
}

/** Maria starts Stop voting and signs, hands the laptop over, and Jose signs last. */
async function closeWithHandover(
    page: Page,
    portal: AdminPortal,
    seals: Array<Record<string, unknown>>
) {
    signInAs(portal, MARIA)
    const server = new SigningServer(portal)
    server.register(MARIA)
    server.register(JOSE)
    mockPost(portal, server)
    guardStopVoting(portal, server, () => seals)

    await stopVoting(page, portal)
    // The route answered a signing request: the panel opens, with Maria's dialog over it.
    await expect(panel(page).getByTestId("signing-status")).toHaveText("Waiting · 0 of 2")
    await expect(panel(page)).toContainText(`Needs 2 signatures from ${POST}'s signers`)
    await signWith(page, MARIA, {
        count: "1 of 2 signatures.",
        next: `Next: ${displayName(JOSE)} sign.`,
    })
    await expect(panel(page).getByTestId("signing-status")).toHaveText("Waiting · 1 of 2")

    // Maria hands the laptop over: she is signed out and Jose signs in.
    await panel(page).getByRole("button", {name: "Next member signs in", exact: true}).click()
    signInAs(portal, JOSE)
    await page
        .getByRole("dialog", {name: "Next member signs in"})
        .getByRole("button", {name: "Sign out", exact: true})
        .click()
    await page.getByRole("button", {name: "Sign in", exact: true}).click()
    await expect(page).toHaveURL(new RegExp(`/sequent_backend_election/${ELECTION_ID}`))
    expect(portal.oidc.logouts).toHaveLength(1)

    // The request reopens for Jose with his dialog.
    await signWith(page, JOSE, {count: "All 2 signatures are in."})
    await expect(panel(page).getByTestId("signing-status")).toHaveText("Done · 2 of 2")
    return server
}

test("closing a Post takes two SBEI signatures on one laptop, with a handover", async ({
    page,
    portal,
}) => {
    const server = await closeWithHandover(page, portal, [])
    const closed = panel(page).getByTestId("closed-voting")
    await expect(closed.getByRole("heading", {level: 3})).toHaveText(
        /^Voting closed at \d\d:\d\d UTC\.$/
    )
    const record = closed.getByRole("table", {name: "Seal record"})
    await expect(record.getByRole("row").first()).toHaveText(
        `Closing signatures in the seal record2, signing code ${CODE}`
    )
    await expect(record.getByRole("row").last()).toHaveText(
        `Signed by the members${displayName(MARIA)}, ${displayName(JOSE)}`
    )
    await expect(closed.getByText(/^Seal /)).toHaveCount(0)

    // Both approvals signed the canonical payload with their certificate's key, and nothing else.
    const payload = server.requests.get(REQUEST_ID)?.canonicalPayload ?? ""
    for (const {variables} of portal.graphql.callsTo("SigningApprove")) {
        expect(verifyApproval(variables, payload)).toBe(true)
        expect(verifyApproval(variables, payload.replace(CODE, "0000-0000"))).toBe(false)
    }
    expect(
        server
            .received(REQUEST_ID)
            .map(({person, algorithm, payloadVerified}) => [
                person.username,
                algorithm,
                payloadVerified,
            ])
    ).toEqual([
        [MARIA.username, "rsa-pkcs1-sha256", true],
        [JOSE.username, "ecdsa-p256-sha256", true],
    ])
    // Each approval came with its own signer's token.
    expect(
        portal.graphql
            .callsTo("SigningApprove")
            .map(
                (call) =>
                    portal.oidc.verifyAccessToken(
                        call.headers.authorization.replace(/^Bearer /, "")
                    )?.sub
            )
    ).toEqual([MARIA.userId, JOSE.userId])
    expect(portal.graphql.callsTo("SigningHandover").map(({variables}) => variables)).toEqual([
        {request_id: REQUEST_ID},
    ])
    for (const operation of [
        "SigningGetRequest",
        "SigningCheckCertificate",
        "SigningApprove",
        "SigningHandover",
    ])
        expectRole(portal, operation, "sign-close-voting")
    // Neither the certificate files nor their passwords left the browser.
    const sent = JSON.stringify(portal.graphql.calls.map(({variables}) => variables))
    for (const person of [MARIA, JOSE])
        expect(sent).not.toContain(fixture(person.certificate).password)
})

test("the closed card shows one SHA-512 seal per country the close sealed", async ({
    page,
    portal,
}) => {
    const seals = [
        {
            area_id: SPAIN_ID,
            area_name: SPAIN,
            ballots: 412,
            hash: "ab".repeat(32) + "ef".repeat(32),
        },
        {area_id: PORTUGAL_ID, area_name: PORTUGAL, ballots: 87, hash: "cd".repeat(64)},
    ].map((seal) => ({...seal, hash_algorithm: "SHA-512", signed_by: "Seal key 2028"}))
    await closeWithHandover(page, portal, seals)
    const closed = panel(page).getByTestId("closed-voting")
    await expect(closed.getByRole("heading", {level: 3})).toHaveText(
        /^Voting closed at \d\d:\d\d UTC\. Ballots sealed\.$/
    )
    for (const seal of seals) {
        const block = closed.getByRole("table", {name: seal.area_name})
        await expect(block.getByRole("row")).toHaveText([
            `Ballots in the seal${seal.ballots}`,
            `Seal SHA-512${seal.hash.slice(0, 8)}…${seal.hash.slice(-8)}`,
            `Signed by${seal.signed_by}`,
        ])
    }
    await expect(closed.getByRole("rowheader", {name: "Seal SHA-512"})).toHaveCount(2)
    await expect(closed.getByRole("table", {name: "Seal record"})).toContainText(
        `2, signing code ${CODE}`
    )
})
