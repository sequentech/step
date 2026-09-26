// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import type {IMiruTransmissionPackageData} from "@/types/miru"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords, eventRecord} from "@/__stories__/fixtures"
import {
    TallyStoryContext,
    resultsEvent,
    tallyData,
    tallyExecution,
    tallySession,
} from "@/resources/Tally/__stories__/TallyFixture"
import {
    AREA_TRUSTEES_ANNOTATION,
    SBEI_USERS_ANNOTATION,
    miruDocuments,
    miruPackage,
    miruTallyAnnotations,
} from "./__stories__/MiruFixture"
import {startedTask} from "./tally/__stories__/DownloadFixture"
import {MiruExportWizard} from "./MiruExportWizard"
import {
    EStoryPermissions,
    EStoryWorkflow,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../ui-essentials/.storybook/globals"

const SIGNATURE_ID = "d0c00000-0000-4000-8000-000000000070"
const UPLOAD_URL = "https://s3.admin-story.invalid/uploads/signature"
const TASK_ID = "7a5c0000-0000-4000-8000-000000000002"
const MIRU_ROLES = ["miru-create", "miru-download", "miru-send", "miru-sign"]

interface Scenario {
    /** SBEI members who signed the current package. */
    signed: string[]
    /** Destination servers the current package reached. */
    sentTo: string[]
    /** Explicit roles instead of the permissions global's group. */
    roles?: string[]
    /** What the send, regenerate and upload services do. */
    service: "success" | "failure"
    onBack: (data: IMiruTransmissionPackageData | null) => void
}

let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let uploads: ReturnType<typeof storyFetch>

function Fixture({signed, sentTo, roles, onBack}: Scenario) {
    const {permissions} = useStoryGlobals()
    const selected = miruPackage({documents: miruDocuments(signed, sentTo)})
    return (
        <AdminStoryProvider
            boundary={boundary}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
        >
            <TallyStoryContext
                data={tallyData()}
                selectedTallySessionData={selected}
                onSetSelectedTallySessionData={onBack}
            >
                <RecordContextProvider
                    value={eventRecord(EStoryWorkflow.RESULTS, {
                        annotations: SBEI_USERS_ANNOTATION,
                    })}
                >
                    <MiruExportWizard />
                </RecordContextProvider>
            </TallyStoryContext>
        </AdminStoryProvider>
    )
}

const CHIP_CONTRAST =
    "The signature and transmission status chips have white labels on MUI's success and info colours, below 4.5 contrast."

const meta = {
    title: "Admin/Components/MiruExportWizard",
    component: MiruExportWizard,
    args: {signed: ["sbei-1"], sentTo: [], service: "success", onBack: fn()},
    argTypes: {
        service: {control: "inline-radio", options: ["success", "failure"]},
        roles: {table: {disable: true}},
        onBack: {table: {disable: true}},
    },
    globals: {workflow: EStoryWorkflow.RESULTS},
    parameters: {expectedFailure: {reason: CHIP_CONTRAST, a11y: ["color-contrast"]}},
    beforeEach: async ({args}) => {
        const fail = () => {
            if (args.service === "failure") throw new Error("Synthetic Miru service unavailable")
        }
        const packageData = miruPackage({documents: miruDocuments(args.signed, args.sentTo)})
        data = resourceBoundary({
            sequent_backend_tally_session: [
                tallySession(EStoryWorkflow.RESULTS, {
                    annotations: miruTallyAnnotations([packageData]),
                }),
            ],
            sequent_backend_tally_session_execution: [tallyExecution(EStoryWorkflow.RESULTS)],
            sequent_backend_results_event: [resultsEvent()],
            sequent_backend_area: [
                {...areaRecords()[0], annotations: AREA_TRUSTEES_ANNOTATION},
                areaRecords()[1],
            ],
        })
        boundary = graphqlBoundary(
            {
                SendTransmissionPackage: () => {
                    fail()
                    return {data: {send_transmission_package: {id: STORY_IDS.tallySession}}}
                },
                CreateTransmissionPackage: () => {
                    fail()
                    return {
                        data: {
                            create_transmission_package: {
                                error_msg: null,
                                task_execution: startedTask(TASK_ID, "CREATE_TRANSMISSION_PACKAGE"),
                            },
                        },
                    }
                },
                GetUploadUrl: () => {
                    fail()
                    return {data: {get_upload_url: {url: UPLOAD_URL, document_id: SIGNATURE_ID}}}
                },
                UploadSignature: () => ({data: {upload_signature: {id: SIGNATURE_ID}}}),
            },
            {schema: true}
        )
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
        await boundary.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const action = (key: string) => i18n.t(`tally.transmissionPackage.actions.${key}`)
const mutations = () => boundary.calls.map(({name}) => name)
/** A notification is visible once its enter transition has ended. */
const notified = (text: string) =>
    waitFor(() => expect(within(document.body).getByText(text)).toBeVisible())

async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    // The title reads the alias from the election row itself, whose presentation the
    // SQLite results keep as a JSON string, so the election shows as "-".
    await expect(
        await canvas.findByText(
            i18n.t("tally.transmissionPackage.title", {name: "North district", eventName: "-"})
        )
    ).toBeVisible()
    // The execution reports two trustees; the package needs two signatures.
    await canvas.findByText(
        i18n.t("tally.transmissionPackage.signatures.status", {signed: 1, total: 2, minimum: 2}),
        {exact: false}
    )
    return canvas
}

async function confirm(canvasElement: HTMLElement, button: string, ok: string) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: button}))
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    expect(boundary.calls).toEqual([])
    await userEvent.click(within(dialog).getByRole("button", {name: ok}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
}

export const Populated: Story = {
    args: {signed: ["sbei-1"]},
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const {permissions} = readStoryGlobals(globals)
        await expect(
            await canvas.findByText(
                i18n.t("tally.transmissionPackage.destinationServers.status", {
                    signed: 0,
                    total: 3,
                })
            )
        ).toBeVisible()
        const admin = permissions === EStoryPermissions.ADMIN
        for (const name of ["send transmission package", "export election data"]) {
            expect(!!canvas.queryByRole("button", {name})).toBe(admin)
        }
        expect(!!canvas.queryByRole("button", {name: "regenerate transmission package"})).toBe(
            admin
        )
        expect(data.calls.filter(({method}) => method === "getOne").map(({args}) => args)).toEqual(
            expect.arrayContaining([
                ["sequent_backend_area", expect.objectContaining({id: STORY_IDS.area})],
                [
                    "sequent_backend_tally_session",
                    expect.objectContaining({id: STORY_IDS.tallySession}),
                ],
            ])
        )
    },
}

export const SendNeedsTheMinimumSignatures: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await expect(canvas.getByRole("button", {name: "send transmission package"})).toBeDisabled()
    },
}

export const SendAfterConfirmation: Story = {
    args: {signed: ["sbei-1", "sbei-2"]},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText(
            i18n.t("tally.transmissionPackage.signatures.status", {
                signed: 2,
                total: 2,
                minimum: 2,
            })
        )
        await confirm(canvasElement, "send transmission package", action("send.dialog.confirm"))
        await notified(i18n.t("miruExport.send.success"))
        expect(boundary.calls).toEqual([
            {
                name: "SendTransmissionPackage",
                variables: {
                    electionId: STORY_IDS.election,
                    tallySessionId: STORY_IDS.tallySession,
                    areaId: STORY_IDS.area,
                },
                headers: {"x-hasura-role": "miru-send"},
            },
        ])
    },
}

export const SendFailureNotifies: Story = {
    args: {signed: ["sbei-1", "sbei-2"], service: "failure"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText(
            i18n.t("tally.transmissionPackage.signatures.status", {
                signed: 2,
                total: 2,
                minimum: 2,
            })
        )
        await confirm(canvasElement, "send transmission package", action("send.dialog.confirm"))
        await notified(i18n.t("miruExport.send.error").trim())
        expect(mutations()).toEqual(["SendTransmissionPackage"])
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("button", {name: "send transmission package"})
            ).toBeEnabled()
        )
    },
}

export const FullySentPackageCannotBeSentAgain: Story = {
    args: {signed: ["sbei-1", "sbei-2"], sentTo: ["ccs-north", "ccs-south", "ccs-east"]},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                i18n.t("tally.transmissionPackage.destinationServers.status", {
                    signed: 3,
                    total: 3,
                })
            )
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "send transmission package"})).toBeDisabled()
    },
}

export const RegenerateAfterConfirmation: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await confirm(
            canvasElement,
            "regenerate transmission package",
            action("regenerate.dialog.confirm")
        )
        await notified(i18n.t("miruExport.create.success"))
        expect(boundary.calls).toEqual([
            {
                name: "CreateTransmissionPackage",
                variables: {
                    electionEventId: EVENT_ID,
                    electionId: STORY_IDS.election,
                    tallySessionId: STORY_IDS.tallySession,
                    areaId: STORY_IDS.area,
                    force: true,
                },
                headers: {"x-hasura-role": "miru-create"},
            },
        ])
    },
}

export const RegenerateFailureNotifies: Story = {
    args: {service: "failure"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await confirm(
            canvasElement,
            "regenerate transmission package",
            action("regenerate.dialog.confirm")
        )
        await notified(i18n.t("miruExport.create.error").trim())
        expect(mutations()).toEqual(["CreateTransmissionPackage"])
    },
}

/** The signature upload and the logs accordions start expanded, each an unnamed region. */
const signingRegions = {
    expectedFailure: {
        reason: `${CHIP_CONTRAST} The expanded signature upload and logs accordions expose two regions without distinct names.`,
        a11y: ["color-contrast", "landmark-unique"],
    },
}

async function uploadSignature(canvasElement: HTMLElement) {
    const canvas = await loaded(canvasElement)
    await expect(canvas.getByText(i18n.t("tally.uploadTransmissionPackage"))).toBeVisible()
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("Missing signature file chooser")
    await userEvent.upload(
        input,
        new File(["synthetic-signature"], "signature.sig", {type: "application/octet-stream"})
    )
    return canvas
}

export const SbeiMemberSignsThePackage: Story = {
    parameters: signingRegions,
    args: {roles: MIRU_ROLES},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await uploadSignature(canvasElement)
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.type(
            dialog.getByLabelText(action("sign.dialog.input.placeholder")),
            "sbei-secret"
        )
        await userEvent.click(dialog.getByRole("button", {name: action("sign.dialog.confirm")}))
        await notified("Signing Successful")
        expect(uploads.calls).toEqual([
            {
                method: "PUT",
                url: UPLOAD_URL,
                headers: {"content-type": "application/octet-stream"},
                body: "synthetic-signature",
            },
        ])
        expect(boundary.calls).toEqual([
            {
                name: "GetUploadUrl",
                variables: {
                    name: "signature.sig",
                    media_type: "application/octet-stream",
                    size: 19,
                    is_public: false,
                    election_event_id: EVENT_ID,
                },
                headers: {"x-hasura-role": "document-upload"},
            },
            {
                name: "UploadSignature",
                variables: {
                    electionId: STORY_IDS.election,
                    tallySessionId: STORY_IDS.tallySession,
                    areaId: STORY_IDS.area,
                    documentId: SIGNATURE_ID,
                    password: "sbei-secret",
                },
                headers: {"x-hasura-role": "miru-sign"},
            },
        ])
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const SigningWithoutPasswordUploadsNoSignature: Story = {
    parameters: signingRegions,
    args: {roles: MIRU_ROLES},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await uploadSignature(canvasElement)
        const dialog = within(await within(document.body).findByRole("dialog"))
        await userEvent.click(dialog.getByRole("button", {name: action("sign.dialog.confirm")}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(mutations()).toEqual(["GetUploadUrl"])
    },
}

export const UploadUrlFailureShowsAnError: Story = {
    parameters: signingRegions,
    args: {roles: MIRU_ROLES, service: "failure"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = await uploadSignature(canvasElement)
        // The signature upload reuses the keys ceremony's backup error text.
        await expect(
            await canvas.findByText(i18n.t("keysGeneration.checkStep.errorUploading"))
        ).toBeVisible()
        expect(uploads.calls).toEqual([])
        expect(within(document.body).queryByRole("dialog")).toBeNull()
    },
}

export const MembersOfOtherAreasCannotSign: Story = {
    args: {roles: MIRU_ROLES},
    globals: {permissions: EStoryPermissions.TRUSTEE},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        // trustee1 is not an SBEI member of the north district.
        expect(canvas.queryByText(i18n.t("tally.uploadTransmissionPackage"))).toBeNull()
        expect(canvasElement.querySelector('input[type="file"]')).toBeNull()
    },
}

export const BackLeavesThePackage: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.back")}))
        expect(args.onBack).toHaveBeenCalledTimes(1)
        expect(args.onBack).toHaveBeenCalledWith(null)
        expect(boundary.calls).toEqual([])
    },
}
