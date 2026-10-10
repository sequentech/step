// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    test,
    expect,
    fixture,
    eventPath,
    output,
    login,
    review,
    cast,
    db,
    contain,
    step,
    telemetryUrl,
} from "./fixtures"
import {readFile, writeFile} from "node:fs/promises"
import {resolve} from "node:path"
import {loadCore} from "@sequentech/ui-test-kit/wasm/node"

test.describe.configure({mode: "serial"})

test("audit downloads independently decode the selected candidate without storing a cast", async ({
    page,
}) => {
    await review(page, fixture.voters.A[0], "Alice")
    const ballotId = (await page.getByText(/^Your Ballot ID:/).textContent())?.match(
        /[0-9a-f]{64}/
    )?.[0]
    expect(ballotId).toBeTruthy()
    await writeFile(resolve(output, "audit-ballot-id.txt"), ballotId!)
    await page.getByRole("button", {name: "Your vote has not been cast", exact: true}).click()
    await page.getByRole("button", {name: "Audit ballot", exact: true}).click()
    await page.getByRole("button", {name: "Yes, discard my ballot to audit"}).click()
    await expect(page.locator(".ballot-verifier-link")).toHaveAttribute(
        "href",
        `${fixture.origins.verifier}${eventPath}/start?lang=en`
    )
    const pending = page.waitForEvent("download")
    await page.getByRole("button", {name: "Download", exact: true}).click()
    await (await pending).saveAs(resolve(output, "auditable-ballot.json"))
    const core = await loadCore()
    const ballot = JSON.parse(await readFile(resolve(output, "auditable-ballot.json"), "utf8"))
    const decoded = core.decode_auditable_ballot_js(ballot)
    expect(decoded).toMatchObject([
        {contest_id: fixture.contests.A, is_blank_ballot: false, is_explicit_invalid: false},
    ])
    expect(decoded[0].choices.filter(({selected}: {selected: number}) => selected >= 0)).toEqual([
        {id: fixture.candidates.A.Alice, selected: 0, write_in_text: null},
    ])
    const data = await db<{sequent_backend_cast_vote: unknown[]}>(
        `query($event: uuid!) { sequent_backend_cast_vote(where:{election_event_id:{_eq:$event}}) { id } }`,
        {event: fixture.eventId}
    )
    expect(data.sequent_backend_cast_vote).toEqual([])
})

test("real PKCE login, signed ballot files, selection, cast and receipt agree with the database", async ({
    page,
}) => {
    const privateRequests: Promise<{url: string; headers: Record<string, string>}>[] = []
    const authorizations: URL[] = []
    page.on("request", (request) => {
        if (
            new URL(request.url()).origin === "http://minio:9000" &&
            new URL(request.url()).searchParams.has("X-Amz-Signature")
        )
            privateRequests.push(
                request.allHeaders().then((headers) => ({url: request.url(), headers}))
            )
        if (request.url().includes("/protocol/openid-connect/auth?"))
            authorizations.push(new URL(request.url()))
    })
    await review(page, fixture.voters.A[0], "Alice")
    expect(authorizations.length).toBeGreaterThan(0)
    expect(authorizations[0].searchParams.get("code_challenge_method")).toBe("S256")
    expect(privateRequests.length).toBeGreaterThanOrEqual(3)
    for (const request of await Promise.all(privateRequests)) {
        expect(new URL(request.url).searchParams.get("X-Amz-Signature")).toBeTruthy()
        expect(request.headers).not.toHaveProperty("authorization")
        expect(request.headers).not.toHaveProperty("cookie")
        const unsigned = await fetch(request.url.split("?")[0])
        expect(unsigned.status).toBe(403)
    }
    const response = await cast(page)
    const body = await response.json()
    expect(body.errors).toBeUndefined()
    const receipt = body.data.insert_cast_vote
    await expect(page.getByTestId("ballot-id").first()).toHaveText(receipt.ballot_id)
    const data = await db<{sequent_backend_cast_vote: unknown[]}>(
        `query($id: uuid!) { sequent_backend_cast_vote(where:{id:{_eq:$id}}) { ballot_id content election_id election_event_id area_id cast_ballot_signature voter_id_string } }`,
        {id: receipt.id}
    )
    expect(data.sequent_backend_cast_vote).toEqual([
        {
            ballot_id: receipt.ballot_id,
            content: receipt.content,
            election_id: fixture.elections.main,
            election_event_id: fixture.eventId,
            area_id: fixture.areas.A,
            cast_ballot_signature: `\\x${Buffer.from(receipt.cast_ballot_signature).toString("hex")}`,
            voter_id_string: receipt.voter_id_string,
        },
    ])
    expect(receipt.cast_ballot_signature).toHaveLength(64)
    expect(response.request().postDataJSON().variables).toMatchObject({
        ballotId: receipt.ballot_id,
        electionId: fixture.elections.main,
        content: receipt.content,
    })
    await writeFile(resolve(output, "receipt.json"), JSON.stringify(receipt))
})

test("the ballot locator finds the real cast receipt", async ({page}) => {
    const receipt = JSON.parse(await readFile(resolve(output, "receipt.json"), "utf8"))
    await login(page, fixture.voters.A[0])
    await expect(page.locator(".election-item").first()).toBeVisible()
    await page.goto(
        `${fixture.origins.voting}${eventPath}/election/${fixture.elections.main}/ballot-locator/${receipt.ballot_id}?lang=en`
    )
    await expect(page.locator(".ballot-locator-success")).toContainText(receipt.ballot_id)
})

test("the verifier imports the actual audit download and displays its decoded selection", async ({
    page,
}) => {
    await login(page, fixture.voters.A[1], "verifier")
    await page.locator('input[type="file"]').setInputFiles(resolve(output, "auditable-ballot.json"))
    await page
        .getByRole("textbox", {name: "Ballot ID", exact: true})
        .fill(await readFile(resolve(output, "audit-ballot-id.txt"), "utf8"))
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await expect(page.getByText("Alice", {exact: true})).toBeVisible()
    const ballotId = await readFile(resolve(output, "audit-ballot-id.txt"), "utf8")
    await expect(page.getByText(ballotId, {exact: true})).toHaveCount(2)
    await expect(page.getByText("Does’t match the decoded ballot ID", {exact: true})).toHaveCount(0)
    await expect(page.getByRole("button", {name: "Print", exact: true})).toBeVisible()
})

test("a permitted revote replaces the choice and a stale third ballot is rejected by the server", async ({
    page,
    browser,
    violations,
}) => {
    await review(page, fixture.voters.A[0], "Carol")
    const secondContext = await browser.newContext({
        locale: "en-US",
        timezoneId: "UTC",
        serviceWorkers: "block",
    })
    await contain(secondContext, violations)
    try {
        const second = await secondContext.newPage()
        await review(second, fixture.voters.A[0], "Bob")
        const accepted = await (await cast(second)).json()
        expect(accepted.errors).toBeUndefined()
        await expect(second.getByTestId("ballot-id").first()).toHaveText(
            accepted.data.insert_cast_vote.ballot_id
        )
        const rejected = await (await cast(page)).json()
        expect(rejected.errors).toEqual([
            expect.objectContaining({
                extensions: expect.objectContaining({code: "InsertFailedExceedsAllowedRevotes"}),
            }),
        ])
        await expect(page.getByRole("alert")).toContainText("You have exceeded the revote limit")
        await expect(page).toHaveURL(/\/review/)
        const receipt = JSON.parse(await readFile(resolve(output, "receipt.json"), "utf8"))
        const rows = await db<{sequent_backend_cast_vote: unknown[]}>(
            `query($event: uuid!, $voter: String!) { sequent_backend_cast_vote(where:{election_event_id:{_eq:$event}, voter_id_string:{_eq:$voter}}) { id } }`,
            {event: fixture.eventId, voter: receipt.voter_id_string}
        )
        expect(rows.sequent_backend_cast_vote).toHaveLength(2)
    } finally {
        await secondContext.close()
    }
})

test("closing the event rejects a ballot already prepared in the browser", async ({page}) => {
    await review(page, fixture.voters.A[2], "Carol")
    await step(
        "update-event-voting-status",
        "--election-event-id",
        fixture.eventId,
        "--voting-status",
        "CLOSED",
        "--voting-channel",
        "ONLINE"
    )
    const rejected = await (await cast(page)).json()
    expect(rejected.errors).toEqual([
        expect.objectContaining({extensions: expect.objectContaining({code: "CheckStatusFailed"})}),
    ])
    await expect(page.getByRole("alert")).toContainText(
        "This election does not allow casting a vote"
    )
    const rows = await db<{sequent_backend_cast_vote: unknown[]}>(
        `query($event: uuid!) { sequent_backend_cast_vote(where:{election_event_id:{_eq:$event}}) { id } }`,
        {event: fixture.eventId}
    )
    expect(rows.sequent_backend_cast_vote).toHaveLength(2)
})

test("an administrator signs in with actual two-factor authentication and reads the imported event", async ({
    page,
    telemetry,
}) => {
    telemetry.enabled = true
    await page.goto(fixture.origins.admin)
    await page.locator('input[name="username"]').fill(process.env.ADMIN_USERNAME!)
    await page.locator('input[name="password"]').fill(process.env.ADMIN_PASSWORD!)
    await page.locator("#kc-login").click()
    for (const [index, digit] of Array.from(fixture.adminOtp).entries()) {
        await page.locator(`#otp-${index + 1}`).fill(digit)
    }
    await page.locator('input[type="submit"]').click()
    await expect(page.getByText(fixture.eventName, {exact: true}).first()).toBeVisible()
    expect(telemetry.requests).toEqual([telemetryUrl])
})

test("published results show the last accepted browser ballot and revocation removes access", async ({
    page,
}) => {
    test.setTimeout(900000)
    const started = await step(
        "start-tally",
        "--election-event-id",
        fixture.eventId,
        "--tally-type",
        "ELECTORAL_RESULTS"
    )
    const tallyId = started
        .match(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/g)
        ?.at(-1)
    expect(tallyId).toBeTruthy()
    await expect
        .poll(
            async () => {
                const data = await db<{
                    sequent_backend_tally_session: {execution_status: string}[]
                }>(
                    `query($id: uuid!) { sequent_backend_tally_session(where:{id:{_eq:$id}}) { execution_status } }`,
                    {id: tallyId}
                )
                const status = data.sequent_backend_tally_session[0]?.execution_status
                expect(["FAILED", "CANCELLED"]).not.toContain(status)
                return status
            },
            {timeout: 600000, intervals: [2000, 5000]}
        )
        .toBe("SUCCESS")
    const data = await db<{
        sequent_backend_tally_session_execution: {id: string; results_event_id: string}[]
    }>(
        `query($id: uuid!) { sequent_backend_tally_session_execution(where:{tally_session_id:{_eq:$id}, results_event_id:{_is_null:false}},order_by:{current_message_id:desc},limit:1) { id results_event_id } }`,
        {id: tallyId}
    )
    const [execution] = data.sequent_backend_tally_session_execution
    expect(execution).toBeTruthy()
    const counts = await db<{
        sequent_backend_results_contest_candidate: {candidate_id: string; cast_votes: number}[]
    }>(
        `query($id: uuid!, $contest: uuid!) { sequent_backend_results_contest_candidate(where:{results_event_id:{_eq:$id},contest_id:{_eq:$contest}}) { candidate_id cast_votes } }`,
        {id: execution.results_event_id, contest: fixture.contests.A}
    )
    expect(
        Object.fromEntries(
            counts.sequent_backend_results_contest_candidate.map((row) => [
                row.candidate_id,
                row.cast_votes,
            ])
        )
    ).toEqual({
        [fixture.candidates.A.Alice]: 0,
        [fixture.candidates.A.Bob]: 1,
        [fixture.candidates.A.Carol]: 0,
    })
    await step(
        "configure-results-website",
        "--election-event-id",
        fixture.eventId,
        "--status",
        "enabled",
        "--access",
        "public",
        "--visibility-scope",
        "full_event"
    )
    await step(
        "publish-results",
        "--election-event-id",
        fixture.eventId,
        "--tally-session-id",
        tallyId!,
        "--tally-session-execution-id",
        execution.id,
        "--results-event-id",
        execution.results_event_id,
        ...Object.values(fixture.elections).flatMap((id) => ["--election-id", id]),
        ...Object.values(fixture.contests).flatMap((id) => ["--contest-id", id])
    )
    let publicationId = ""
    await expect
        .poll(
            async () => {
                const data = await db<{
                    sequent_backend_tally_results_publication: {
                        id: string
                        publication_status: string
                        error_message: string | null
                    }[]
                }>(
                    `query($event: uuid!) { sequent_backend_tally_results_publication(where:{election_event_id:{_eq:$event}},order_by:{created_at:desc},limit:1) { id publication_status error_message } }`,
                    {event: fixture.eventId}
                )
                const publication = data.sequent_backend_tally_results_publication[0]
                expect(publication?.error_message).toBeNull()
                publicationId = publication?.id
                return publication?.publication_status
            },
            {timeout: 180000, intervals: [2000]}
        )
        .toBe("Published")
    await page.goto(`${fixture.origins.results}/${fixture.eventId}?lang=en`)
    await expect(page.getByText("E2E main election", {exact: true}).first()).toBeVisible()
    await page.getByRole("tab", {name: "E2E main election", exact: true}).click()
    await page.getByRole("tab", {name: "Contest A", exact: true}).click()
    await expect(page.getByText("Bob", {exact: true}).first()).toBeVisible()
    for (const [name, votes] of [
        ["Alice", "0"],
        ["Bob", "1"],
        ["Carol", "0"],
    ]) {
        await expect(
            page.getByRole("row").filter({hasText: name}).getByRole("gridcell").nth(1)
        ).toHaveText(votes)
    }
    await page.getByRole("tab", {name: "E2E area A", exact: true}).click()
    await expect(
        page.getByRole("row").filter({hasText: "Bob"}).getByRole("gridcell").nth(1)
    ).toHaveText("1")
    await step(
        "revoke-results-publication",
        "--election-event-id",
        fixture.eventId,
        "--publication-id",
        publicationId
    )
    await page.reload()
    await expect(page.getByText("Bob", {exact: true})).toHaveCount(0)
    await expect(
        page.getByRole("heading", {name: "Results not published yet", exact: true})
    ).toBeVisible()
})
