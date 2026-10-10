// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath, IDS, FIXED_TIME} from "./fixtures"

for (const scenario of [
    {
        name: "online open",
        online: "OPEN",
        early: "CLOSED",
        kiosk: false,
        allowEarly: false,
        active: true,
    },
    {
        name: "all channels closed",
        online: "CLOSED",
        early: "CLOSED",
        kiosk: false,
        allowEarly: false,
        active: false,
    },
    {
        name: "early voter",
        online: "CLOSED",
        early: "OPEN",
        kiosk: false,
        allowEarly: true,
        active: true,
    },
    {
        name: "early channel outside eligible area",
        online: "CLOSED",
        early: "OPEN",
        kiosk: false,
        allowEarly: false,
        active: false,
    },
    {
        name: "kiosk open while online closed",
        online: "CLOSED",
        early: "CLOSED",
        kiosk: true,
        allowEarly: false,
        active: true,
    },
]) {
    test(`chooser channel policy: ${scenario.name}`, async ({page, portal}) => {
        Object.assign(portal.data.event.status, {
            voting_status: scenario.online,
            early_voting_status: scenario.early,
        })
        portal.data.summary.area_presentation.allow_early_voting = scenario.allowEarly
            ? "allow_early_voting"
            : "no_early_voting"
        portal.publish()
        await page.goto(`${portal.origin}${eventPath}?lang=en${scenario.kiosk ? "&kiosk" : ""}`)
        await expect(page.getByRole("heading", {name: "Community Council"})).toBeVisible()
        const vote = page.getByRole("button", {name: /click to vote/i})
        if (scenario.active) await expect(vote).toBeEnabled()
        else await expect(vote).toHaveCount(0)
    })
}

for (const [limit, active] of [
    [0, true],
    [1, false],
    [2, true],
] as const) {
    test(`one existing cast with revote limit ${limit}`, async ({page, portal}) => {
        portal.data.election.num_allowed_revotes = limit
        portal.castVotes = [
            {
                id: "90000000-0000-4000-8000-000000000001",
                tenant_id: IDS.tenant,
                election_id: IDS.election,
                election_event_id: IDS.event,
                status: "valid",
            },
        ]
        portal.publish()
        await page.goto(`${portal.origin}${eventPath}?lang=en`)
        const vote = page.getByRole("button", {name: /click to vote/i})
        if (active) await expect(vote).toBeEnabled()
        else await expect(vote).toBeDisabled()
    })
}

for (const [kind, message] of [
    ["area", "You are not listed as a voter in this election"],
    ["network", "A network problem occurred"],
    ["other", "There was a problem fetching the data"],
]) {
    test(`chooser ${kind} failure prevents ballot selection`, async ({page, portal}) => {
        portal.graphql.on("GetVoterStatus", () =>
            kind === "network"
                ? {abort: "connectionfailed"}
                : {
                      errors: [
                          {
                              message:
                                  kind === "area"
                                      ? "missing x-hasura-area-id"
                                      : "Synthetic data failure",
                          },
                      ],
                  }
        )
        await page.goto(`${portal.origin}${eventPath}?lang=en`)
        await expect(page.getByText(message, {exact: false})).toBeVisible()
        await expect(page.getByRole("button", {name: /click to vote/i})).toHaveCount(0)
    })
}

test("unpublished event explains why voting is unavailable", async ({page, portal}) => {
    portal.data.event.status.is_published = false
    portal.publish()
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByText(/The election event hasn’t been published yet/)).toBeVisible()
})

test("no eligible elections explains the empty chooser", async ({page, portal}) => {
    portal.graphql.on("GetVoterStatus", () => ({
        data: {
            get_ballot_files_urls: {
                event_id: IDS.event,
                status: portal.data.event.status,
                files: [],
            },
            sequent_backend_cast_vote: [],
        },
    }))
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByText(/There are no elections you can vote for/)).toBeVisible()
    await expect(page.getByRole("button", {name: /click to vote/i})).toHaveCount(0)
})

test("closed event exposes scoped results links", async ({page, portal}) => {
    Object.assign(portal.data.event.status, {
        voting_status: "CLOSED",
        kiosk_voting_status: "CLOSED",
        early_voting_status: "CLOSED",
    })
    Object.assign(portal.data.event.presentation, {
        results_website: {status: "enabled", access: "public", visibility_scope: "full_event"},
    })
    portal.publish()
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(
        page
            .getByRole("link", {name: /results/i})
            .filter({hasNotText: /^$/})
            .first()
    ).toHaveAttribute("href", `${portal.origin}/results/${IDS.event}`)
    await expect(
        page.locator(`a[href='${portal.origin}/results/${IDS.event}/elections/${IDS.election}']`)
    ).toBeVisible()
})

test("skip election list bypasses a single eligible ballot", async ({page, portal}) => {
    portal.data.event.presentation.skip_election_list = true
    portal.publish()
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page).toHaveURL(/\/start\?lang=en$/)
    await expect(page.getByRole("button", {name: "Start Voting", exact: true})).toBeEnabled()
})

for (const finalStatus of ["valid", "discarded"]) {
    test(`pending vote polls until ${finalStatus} and updates eligibility`, async ({
        page,
        portal,
    }) => {
        portal.settings.QUERY_POLL_INTERVAL_MS = 1000
        portal.data.election.num_allowed_revotes = 1
        const vote = {
            id: "90000000-0000-4000-8000-000000000001",
            tenant_id: IDS.tenant,
            election_id: IDS.election,
            election_event_id: IDS.event,
            status: "in-progress",
            created_at: FIXED_TIME,
        }
        portal.castVotes = [vote]
        portal.graphql.on("GetCastVotes", () => ({
            data: {sequent_backend_cast_vote: portal.castVotes},
        }))
        portal.publish()
        await page.goto(`${portal.origin}${eventPath}?lang=en`)
        await expect(page.getByRole("button", {name: /click to vote/i})).toBeDisabled()
        portal.castVotes = [{...vote, status: finalStatus}]
        await page.clock.runFor(1100)
        await expect.poll(() => portal.graphql.callsTo("GetCastVotes").length).toBeGreaterThan(1)
        const button = page.getByRole("button", {name: /click to vote/i})
        if (finalStatus === "discarded") await expect(button).toBeEnabled()
        else await expect(button).toBeDisabled()
        const settled = portal.graphql.callsTo("GetCastVotes").length
        await page.clock.runFor(3100)
        expect(portal.graphql.callsTo("GetCastVotes")).toHaveLength(settled)
    })
}

for (const [flag, code] of [
    ["is_explicit_invalid", "multipleExplicitInvalidCandidates"],
    ["is_explicit_blank", "multipleExplicitBlankCandidates"],
]) {
    test(`duplicate ${flag} choices reject the ballot configuration before voting`, async ({
        page,
        portal,
    }) => {
        for (const candidate of portal.data.ballot.contests[0].candidates)
            Object.assign(candidate.presentation, {[flag]: true})
        portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
        portal.publish()
        await page.goto(`${portal.origin}${eventPath}?lang=en`)
        await page.getByRole("button", {name: /click to vote/i}).click()
        await expect(page.getByRole("heading", {name: "Oops! Unexpected Error"})).toBeVisible()
        await expect(page.getByText(`errors.configuration.${code}`, {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Start Voting", exact: true})).toHaveCount(0)
        expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
    })
}
