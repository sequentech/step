// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect} from "../../../e2e/fixtures"
import {castBallotAsVoter, login} from "../load/flow"
import {Client} from "pg"

test("a real encrypted cast agrees with the UI and persisted receipt @smoke @probe", async ({
    page,
    fixture,
}) => {
    const responsePromise = page.waitForResponse((response) => {
        const body = response.request().postData()
        return response.url().includes("/v1/graphql") && Boolean(body?.includes("insert_cast_vote"))
    })
    const ballots = await castBallotAsVoter(page, {
        loginUrl: fixture.loginUrl,
        credentials: {username: `${fixture.usernamePrefix}0`, password: fixture.password},
    })
    const response = await responsePromise
    expect(response.ok()).toBe(true)
    const body = (await response.json()) as {
        errors?: unknown[]
        data?: {
            insert_cast_vote?: {
                id: string
                ballot_id: string
                election_id: string
                election_event_id: string
            }
        }
    }
    expect(body.errors).toBeUndefined()
    const receipt = body.data?.insert_cast_vote
    expect(receipt).toBeDefined()
    expect(ballots).toContain(receipt!.ballot_id)
    expect(receipt!.election_id).toBe(fixture.electionId)
    expect(receipt!.election_event_id).toBe(fixture.eventId)
    const db = new Client({connectionString: fixture.auditDsn, statement_timeout: 5000})
    await db.connect()
    try {
        const persisted = await db.query<{ballot_id: string}>(
            "SELECT ballot_id FROM sequent_backend.cast_vote WHERE id=$1 AND tenant_id=$2 AND election_event_id=$3 AND election_id=$4",
            [receipt!.id, fixture.tenantId, fixture.eventId, fixture.electionId]
        )
        expect(persisted.rows).toEqual([{ballot_id: receipt!.ballot_id}])
    } finally {
        await db.end()
    }
})

test("invalid credentials cannot enter the ballot @smoke", async ({page, fixture}) => {
    await page.goto(fixture.loginUrl)
    await login(page, {username: `${fixture.usernamePrefix}1`, password: "incorrect-e2e-password"})
    await expect(page.locator("#kc-login")).toBeVisible()
    await expect(page.locator("#input-error, .alert-error, .pf-m-danger").first()).toBeVisible()
    await expect(page.locator(".election-item")).toHaveCount(0)
})

test("login hints prefill without leaking into the return URL", async ({page, fixture}) => {
    const url = new URL(fixture.loginUrl)
    url.searchParams.set("login_hint__username", `${fixture.usernamePrefix}2`)
    url.searchParams.set("lang", "en")
    await page.goto(url.toString())
    await expect(page.locator('input[name="username"]')).toHaveValue(`${fixture.usernamePrefix}2`)
    const redirect = new URL(page.url()).searchParams.get("redirect_uri")
    expect(redirect).not.toBeNull()
    expect(
        [...new URL(redirect!).searchParams.keys()].some((key) => key.startsWith("login_hint__"))
    ).toBe(false)
})
