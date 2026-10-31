// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    BASE_ROLES,
    CONTENT_IDS,
    candidateRow,
    contestRow,
    electionRow,
    eventPage,
    expectRole,
    names,
    table,
} from "./data"

const DOCUMENT_ID = "a0000000-0000-4000-8000-000000000007"
const imageBytes = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=",
    "base64"
)
const originalName = "ballot image.png"
const candidatePresentation = {
    ...names("Alice Adams"),
    urls: [{url: "https://candidate.example.test/", is_image: false}],
}
const resources = [
    {name: "election", id: IDS.election, role: "election-write"},
    {name: "contest", id: IDS.contest, role: "contest-write"},
    {name: "candidate", id: CONTENT_IDS.candidate, role: "candidate-write"},
] as const

test.use({
    roles: [
        ...BASE_ROLES,
        "election-data-tab",
        "election-write",
        "contest-read",
        "contest-write",
        "candidate-read",
        "candidate-write",
        "document-read",
        "document-write",
    ],
})

function ballot(portal: PortalServices, fileName: string) {
    const election = electionRow()
    const contest = contestRow()
    eventPage(portal, undefined, [election])
    table(portal, "sequent_backend_election", [election])
    table(portal, "sequent_backend_contest", [contest])
    const candidates = table(portal, "sequent_backend_candidate", [
        candidateRow({presentation: candidatePresentation}),
    ])
    table(portal, "sequent_backend_document", [
        {
            id: DOCUMENT_ID,
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
            name: fileName,
            media_type: "image/png",
            size: imageBytes.length,
            labels: {},
            annotations: {},
        },
    ])
    portal.graphql.on("contest_tree", () => ({data: {sequent_backend_contest: [contest]}}))
    portal.graphql.on("candidate_tree", () => ({data: {sequent_backend_candidate: candidates}}))
    const key = `tenant-${TENANT_ID}/document-${DOCUMENT_ID}/${fileName}`
    const imageUrl = portal.s3.putBytes("public", key, imageBytes, "image/png")
    const uploadUrl = portal.s3.presign(key, "ballot-image-upload")
    portal.s3.override(
        (request) => request.method === "PUT" && request.url === uploadUrl,
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url: uploadUrl, document_id: DOCUMENT_ID}},
    }))
    return {key, imageUrl, uploadUrl}
}

for (const resource of resources) {
    test(`uploads an ${resource.name} image with the exact bytes and document association`, async ({
        page,
        portal,
    }) => {
        const fileName = resource.name === "candidate" ? "ballotimage.png" : originalName
        const {key, imageUrl, uploadUrl} = ballot(portal, fileName)
        await page.goto(`${portal.origin}/sequent_backend_${resource.name}/${resource.id}?lang=en`)
        await page.getByRole("button", {name: "Image", exact: true}).click()
        const uploaded = page.waitForRequest(
            (request) => request.url() === uploadUrl && request.method() === "PUT"
        )
        await page
            .getByLabel("Drop Input File")
            .setInputFiles({name: originalName, mimeType: "image/png", buffer: imageBytes})
        const request = await uploaded
        expect(request.headers()["content-type"]).toBe("image/*")
        expect(request.postDataBuffer()).toEqual(imageBytes)
        await expect
            .poll(() => portal.graphql.callsTo(`update_sequent_backend_${resource.name}`).length)
            .toBe(1)
        expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
            {name: fileName, media_type: "image/png", size: imageBytes.length, is_public: true},
        ])
        expect(
            portal.graphql
                .callsTo(`update_sequent_backend_${resource.name}`)
                .map(({variables}) => variables)
        ).toEqual([
            {
                where: {id: {_eq: resource.id}},
                _set:
                    resource.name === "candidate"
                        ? {
                              image_document_id: DOCUMENT_ID,
                              presentation: {
                                  ...candidatePresentation,
                                  urls: [...candidatePresentation.urls, {url: key, is_image: true}],
                              },
                          }
                        : {image_document_id: DOCUMENT_ID},
            },
        ])
        expectRole(portal, `update_sequent_backend_${resource.name}`, resource.role)
        await page.reload()
        await page.getByRole("button", {name: "Image", exact: true}).click()
        await expect(page.getByRole("img", {name: key, exact: true})).toHaveAttribute(
            "src",
            decodeURI(imageUrl)
        )
        if (resource.name === "candidate") {
            // The delete icon has no label, so scope it to its visible Image section.
            await page
                .getByRole("button", {name: "Image", exact: true})
                .getByRole("button", {name: "", exact: true})
                .click()
            await expect
                .poll(() => portal.graphql.callsTo("update_sequent_backend_candidate").length)
                .toBe(2)
            expect(portal.graphql.callsTo("update_sequent_backend_candidate")[1].variables).toEqual(
                {
                    where: {id: {_eq: CONTENT_IDS.candidate}},
                    _set: {image_document_id: null, presentation: candidatePresentation},
                }
            )
            await expect(page.getByRole("img", {name: key, exact: true})).toHaveCount(0)
        }
    })
}

for (const resource of resources) {
    test(`waits for ${resource.name} image metadata before rendering its URL`, async ({
        page,
        portal,
    }) => {
        const {uploadUrl, key, imageUrl} = ballot(
            portal,
            resource.name === "candidate" ? "ballotimage.png" : originalName
        )
        const malformedKey = `tenant-${TENANT_ID}/document-${DOCUMENT_ID}/undefined`
        let releaseMetadata!: () => void
        const heldMetadata = new Promise<void>((resolve) => {
            releaseMetadata = resolve
        })
        let metadataWaiting = false
        await page.route("**/v1/graphql", async (route) => {
            const body = route.request().postDataJSON()
            if (
                body.operationName === "sequent_backend_document" &&
                JSON.stringify(body.variables).includes(DOCUMENT_ID)
            ) {
                metadataWaiting = true
                await heldMetadata
            }
            await route.fallback()
        })
        try {
            await page.goto(
                `${portal.origin}/sequent_backend_${resource.name}/${resource.id}?lang=en`
            )
            await page.getByRole("button", {name: "Image", exact: true}).click()
            const uploaded = page.waitForRequest(
                (request) => request.url() === uploadUrl && request.method() === "PUT"
            )
            await page
                .getByLabel("Drop Input File")
                .setInputFiles({name: originalName, mimeType: "image/png", buffer: imageBytes})
            expect((await uploaded).postDataBuffer()).toEqual(imageBytes)
            await expect
                .poll(
                    () => portal.graphql.callsTo(`update_sequent_backend_${resource.name}`).length
                )
                .toBe(1)
            await expect.poll(() => metadataWaiting).toBe(true)

            await expect(page.getByRole("img", {name: malformedKey, exact: true})).toHaveCount(0, {
                timeout: 2000,
            })
        } finally {
            releaseMetadata()
        }
        await expect(page.getByRole("img", {name: key, exact: true})).toHaveAttribute(
            "src",
            decodeURI(imageUrl)
        )
    })
}
