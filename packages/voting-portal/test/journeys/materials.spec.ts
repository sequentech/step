// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath, IDS, FIXED_TIME} from "./fixtures"

const documentId = "a0000000-0000-4000-8000-000000000002"
const materialId = "b0000000-0000-4000-8000-000000000002"

for (const acknowledged of [false, true]) {
    test(`mandatory materials do not flash the banner while acknowledgement loads: acknowledged=${acknowledged}`, async ({
        page,
        portal,
    }) => {
        portal.data.event.presentation.materials.policy = "mandatory_for_voting"
        portal.data.style.ballot_eml = JSON.stringify(portal.data.ballot)
        portal.publish()
        portal.graphql.on("GetSupportMaterials", () => ({
            data: {
                sequent_backend_support_material: [
                    {
                        id: materialId,
                        tenant_id: IDS.tenant,
                        election_event_id: IDS.event,
                        document_id: documentId,
                        kind: "image/svg+xml",
                        data: {title: "Voting instructions"},
                        created_at: FIXED_TIME,
                        last_updated_at: FIXED_TIME,
                        annotations: {},
                        labels: {},
                    },
                ],
            },
        }))
        let release: (ids: string[]) => void = () => {
            throw new Error("Acknowledgement was not requested")
        }
        const acknowledgement = new Promise<string[]>((resolve) => {
            release = resolve
        })
        portal.graphql.on("GetSupportMaterialsAcknowledgment", async () => ({
            data: {get_support_materials_acknowledgment: {document_ids: await acknowledgement}},
        }))
        await page.goto(`${portal.origin}${eventPath}?lang=en`)
        const vote = page.getByRole("button", {name: /click to vote/i})
        await expect(vote).toBeDisabled()
        await expect(page.getByText(/You must read the/)).toHaveCount(0)
        release(acknowledged ? [documentId] : [])
        if (acknowledged) {
            await expect(vote).toBeEnabled()
            await expect(page.getByText(/You must read the/)).toHaveCount(0)
            return
        }
        await expect(page.getByText(/You must read the/)).toBeVisible()
        await expect(vote).toBeDisabled()
        portal.graphql.on("GetDocument", () => ({
            data: {
                sequent_backend_document: [
                    {
                        id: documentId,
                        tenant_id: IDS.tenant,
                        election_event_id: IDS.event,
                        name: "instructions.svg",
                        media_type: "image/svg+xml",
                        size: 115,
                        is_public: true,
                        created_at: FIXED_TIME,
                        last_updated_at: FIXED_TIME,
                        labels: {},
                        annotations: {},
                    },
                ],
            },
        }))
        portal.graphql.on("AcknowledgeSupportMaterials", ({variables}) => ({
            data: {
                acknowledge_support_materials: {
                    election_event_id: IDS.event,
                    document_ids: variables.documentIds,
                },
            },
        }))
        portal.s3.putBytes(
            "public",
            `tenant-${IDS.tenant}/document-${documentId}/instructions.svg`,
            new TextEncoder().encode(
                '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="40"><text x="2" y="25">Instructions</text></svg>'
            ),
            "image/svg+xml"
        )
        await page.getByRole("link", {name: "Support Materials", exact: true}).click()
        await expect(page.getByText("Voting instructions", {exact: true})).toBeVisible()
        const checkbox = page.getByRole("checkbox", {name: "I have read the Support Materials"})
        await expect(checkbox).toBeDisabled()
        await page.getByRole("button", {name: "Preview Voting instructions"}).click()
        await expect(page.getByRole("dialog")).toBeVisible()
        await page.getByRole("dialog").getByRole("button", {name: "Close", exact: true}).click()
        await checkbox.check()
        await page.getByRole("button", {name: "Continue", exact: true}).click()
        await expect(page).toHaveURL(/election-chooser/)
        await expect(vote).toBeEnabled()
        expect(portal.graphql.callsTo("AcknowledgeSupportMaterials")[0].variables).toEqual({
            electionEventId: IDS.event,
            documentIds: [documentId],
        })
        expect(portal.graphql.callsTo("GetSupportMaterialsAcknowledgment")).toHaveLength(1)
    })
}
