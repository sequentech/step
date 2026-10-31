// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// VOTE-FREEZE: the seal record of a published ballot box. A public record is
// a link to the public bucket; a restricted one is a private event document
// that an administrator with the document download permission downloads.
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {GraphQLError} from "graphql"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EBallotBoxSealStatus} from "@/types/ballotBoxSeal"
import {SealRecordLink} from "./SealRecordLink"
import {AREAS, RESTRICTED_RECORD_DOCUMENTS, sealRow} from "./__stories__/BallotBoxesCard.fixtures"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

/** What fetchDocument answers for a restricted record. */
enum EFetchDocumentReply {
    URL = "url",
    /** The document row or its stored file is gone. */
    MISSING = "missing",
    /** Any other failure. */
    ERROR = "error",
}

interface Scenario {
    /** Published to the public bucket, else a restricted private document. */
    isPublic: boolean
    reply: EFetchDocumentReply
    /** The signed-in administrator lacks the document download permission. */
    withoutDownload?: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>

const seal = (isPublic: boolean) =>
    sealRow(
        AREAS.first,
        EBallotBoxSealStatus.PUBLISHED,
        isPublic ? {} : {public_path: null, public_document_id: RESTRICTED_RECORD_DOCUMENTS[0]}
    )

function Fixture({isPublic, withoutDownload}: Omit<Scenario, "reply">) {
    const {permissions} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            role={permissions}
            roles={withoutDownload ? [] : undefined}
        >
            <SealRecordLink
                electionEventId={EVENT_ID}
                seal={seal(isPublic)}
                areaName={AREAS.first.name}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Dashboard/Election/SealRecordLink",
    component: SealRecordLink,
    args: {isPublic: true, reply: EFetchDocumentReply.URL},
    argTypes: {reply: {control: "inline-radio", options: Object.values(EFetchDocumentReply)}},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                FetchDocument: () => {
                    if (args.reply === EFetchDocumentReply.MISSING) {
                        return {
                            errors: [
                                new GraphQLError("Document not found", {
                                    extensions: {code: "DocumentNotFound"},
                                }),
                            ],
                        }
                    }
                    if (args.reply === EFetchDocumentReply.ERROR) {
                        return {
                            errors: [
                                new GraphQLError("Could not fetch the document.", {
                                    extensions: {code: "InternalServerError"},
                                }),
                            ],
                        }
                    }
                    return {data: {fetchDocument: {url: "data:application/json,%7B%7D"}}}
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const download = (canvasElement: HTMLElement) =>
    userEvent.click(
        within(canvasElement).getByRole("button", {name: "Download the seal record of Spain"})
    )

/** Seal Record Publication Public: a link to the record in the public bucket. */
export const PublicRecord: Story = {
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByRole("link", {name: "Open the seal record of Spain"})
        ).toHaveAttribute(
            "href",
            `https://public.admin-story.invalid/ballot-box-seals/${STORY_IDS.election}/${STORY_IDS.area}.json`
        )
        expect(graphql.calls).toEqual([])
    },
}

/** Restricted: the administrator downloads the private document through a presigned URL. */
export const RestrictedRecord: Story = {
    args: {isPublic: false},
    play: async ({canvasElement}) => {
        await download(canvasElement)
        await waitFor(() =>
            expect(graphql.calls.find(({name}) => name === "FetchDocument")?.variables).toEqual({
                electionEventId: EVENT_ID,
                documentId: RESTRICTED_RECORD_DOCUMENTS[0],
            })
        )
        expect(
            within(canvasElement).queryByText("The seal record could not be downloaded. Try again.")
        ).toBeNull()
    },
}

/** The document is gone: an incident, not something a retry fixes. */
export const RecordMissing: Story = {
    args: {isPublic: false, reply: EFetchDocumentReply.MISSING},
    play: async ({canvasElement}) => {
        await download(canvasElement)
        await expect(
            await within(canvasElement).findByText(
                "The seal record document is missing: report it as an incident."
            )
        ).toBeVisible()
    },
}

/** Any other failure may pass: the administrator can try again. */
export const DownloadFailed: Story = {
    args: {isPublic: false, reply: EFetchDocumentReply.ERROR},
    play: async ({canvasElement}) => {
        await download(canvasElement)
        await expect(
            await within(canvasElement).findByText(
                "The seal record could not be downloaded. Try again."
            )
        ).toBeVisible()
    },
}

/** Without the document download permission the record is restricted, and the cell says who can. */
export const WithoutDownloadPermission: Story = {
    args: {isPublic: false, withoutDownload: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText("Restricted: ask an administrator who can download documents.")
        ).toBeVisible()
        expect(canvas.queryByRole("button")).toBeNull()
    },
}
