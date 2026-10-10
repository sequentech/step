// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath} from "./fixtures"
import {review} from "./flow"

test("finish returns to the chooser while an open election still allows voting", async ({
    page,
    portal,
}) => {
    await review(page, portal)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await page.getByRole("button", {name: "Finish", exact: true}).click()
    await expect(page).toHaveURL(`${portal.origin}${eventPath}/election-chooser`)
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    expect(portal.oidc.logouts).toEqual([])
    expect(portal.graphql.callsTo("InsertCastVote")).toHaveLength(1)
})

test("finishing the last kiosk vote uses the kiosk completion URL", async ({page, portal}) => {
    const finishUrl = `${portal.origin}/finished-kiosk`
    Object.assign(portal.data.event.presentation, {
        redirect_finish_url: `${portal.origin}/finished-online`,
        kiosk_redirect_finish_url: finishUrl,
    })
    portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
    portal.data.election.num_allowed_revotes = 1
    portal.publish()
    await review(page, portal, false, "?kiosk&lang=en")
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await page.getByRole("button", {name: "Finish", exact: true}).click()
    await expect(page).toHaveURL(finishUrl)
    expect(portal.oidc.logouts[0].params.post_logout_redirect_uri).toBe(finishUrl)
    expect(portal.graphql.callsTo("InsertCastVote")).toHaveLength(1)
})
