// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {randomUUID} from "node:crypto"
import {writeFile} from "node:fs/promises"
import {resolve} from "node:path"
import {test, expect, fixture, login, db, output} from "./fixtures"

test("the authenticated verifier can read only active published ballot styles in its voter scope", async ({
    page,
}) => {
    let authorization = ""
    page.on("request", (request) => {
        if (request.url() === "http://graphql-engine:8080/v1/graphql")
            authorization = request.headers().authorization ?? authorization
    })
    await login(page, fixture.voters.A[3])
    await expect(page.locator(".election-item").first()).toBeVisible()
    expect(authorization).toMatch(/^Bearer /)
    type Style = {id: string; area_id: string; ballot_publication_id: string} & Record<
        string,
        unknown
    >
    const {sequent_backend_ballot_style: styles} = await db<{
        sequent_backend_ballot_style: Style[]
    }>(
        `query($event: uuid!) { sequent_backend_ballot_style(where:{election_event_id:{_eq:$event}}) { id tenant_id election_event_id election_id area_id ballot_publication_id ballot_eml ballot_signature status } }`,
        {event: fixture.eventId}
    )
    const original = styles.find((style) => style.area_id === fixture.areas.A)!
    const otherArea = styles.find((style) => style.area_id === fixture.areas.B)!
    expect(original).toBeTruthy()
    const unpublished = randomUUID(),
        deletedPublication = randomUUID()
    const hiddenStyles = [randomUUID(), randomUUID(), randomUUID()]
    const publishedAt = "2024-01-01T00:00:00Z"
    try {
        await db(
            `mutation($objects:[sequent_backend_ballot_publication_insert_input!]!) { insert_sequent_backend_ballot_publication(objects:$objects) { affected_rows } }`,
            {
                objects: [
                    {
                        id: unpublished,
                        tenant_id: fixture.tenantId,
                        election_event_id: fixture.eventId,
                        election_ids: [fixture.elections.main],
                        published_at: null,
                    },
                    {
                        id: deletedPublication,
                        tenant_id: fixture.tenantId,
                        election_event_id: fixture.eventId,
                        election_ids: [fixture.elections.main],
                        published_at: publishedAt,
                        deleted_at: publishedAt,
                    },
                ],
            }
        )
        await db(
            `mutation($objects:[sequent_backend_ballot_style_insert_input!]!) { insert_sequent_backend_ballot_style(objects:$objects) { affected_rows } }`,
            {
                objects: [
                    {...original, id: hiddenStyles[0], ballot_publication_id: unpublished},
                    {...original, id: hiddenStyles[1], ballot_publication_id: deletedPublication},
                    {...original, id: hiddenStyles[2], deleted_at: publishedAt},
                ],
            }
        )
        const response = await fetch("http://graphql-engine:8080/v1/graphql", {
            method: "POST",
            headers: {"content-type": "application/json", authorization},
            body: JSON.stringify({
                query: `query($ids:[uuid!]!) { sequent_backend_ballot_style(where:{id:{_in:$ids}}) { id ballot_eml ballot_signature } }`,
                variables: {ids: [original.id, otherArea.id, ...hiddenStyles]},
            }),
        })
        expect(response.status).toBe(200)
        const body = await response.json()
        expect(body.errors).toBeUndefined()
        const rows = body.data.sequent_backend_ballot_style as {
            id: string
            ballot_eml: unknown
            ballot_signature: unknown
        }[]
        const visible = new Set(rows.map((row) => row.id))
        const visibility = {
            published: visible.has(original.id),
            unpublished: visible.has(hiddenStyles[0]),
            deletedPublication: visible.has(hiddenStyles[1]),
            deletedStyle: visible.has(hiddenStyles[2]),
            otherArea: visible.has(otherArea.id),
        }
        await writeFile(
            resolve(output, "publication-access.json"),
            JSON.stringify(visibility, null, 2)
        )
        expect(visibility).toEqual({
            published: true,
            unpublished: false,
            deletedPublication: false,
            deletedStyle: false,
            otherArea: false,
        })
        expect(rows).toEqual([
            {
                id: original.id,
                ballot_eml: original.ballot_eml,
                ballot_signature: original.ballot_signature,
            },
        ])
    } finally {
        await db(
            `mutation($ids:[uuid!]!) { delete_sequent_backend_ballot_style(where:{id:{_in:$ids}}) { affected_rows } }`,
            {ids: hiddenStyles}
        )
        await db(
            `mutation($ids:[uuid!]!) { delete_sequent_backend_ballot_publication(where:{id:{_in:$ids}}) { affected_rows } }`,
            {ids: [unpublished, deletedPublication]}
        )
    }
})
