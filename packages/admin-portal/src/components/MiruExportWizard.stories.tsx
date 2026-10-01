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
import {SigningProvider} from "./signing/SigningProvider"
import {CODE, JOSE, MARIA, REQUEST_ID, fakeApi, makePanel} from "./signing/__stories__/fixtures"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
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
    /** Whether the transmit-results rule needs signatures now (read with signing-rules-read). */
    transmitRuleRequired?: boolean
    /** The transmit-results rule needs signatures: the package's request is in this state. */
    signingRequest?:
        | SigningRequestStatus.Waiting
        | SigningRequestStatus.Executed
        | SigningRequestStatus.Cancelled
}

/** The transmit-results rule asks for two signatures. */
const REQUIRED = 2
const SIGNING_REQUEST = {id: REQUEST_ID, code: CODE, required: REQUIRED}

const packageOf = ({signed, sentTo, signingRequest}: Scenario) =>
    miruPackage({
        documents: miruDocuments(signed, sentTo),
        signing_request: signingRequest ? SIGNING_REQUEST : undefined,
    })

/** The transmission request as the signing panel shows it. */
const transmissionPanel = async (status: SigningRequestStatus): Promise<ISigningPanelData> => {
    const eml = new TextEncoder().encode("%PDF-1.7 synthetic election returns")
    const digest = await crypto.subtle.digest("SHA-256", eml)
    const emlSha256 = Array.from(new Uint8Array(digest), (b) =>
        b.toString(16).padStart(2, "0")
    ).join("")
    const signed = status === SigningRequestStatus.Executed ? [MARIA, JOSE] : [MARIA]
    const panel = await makePanel({
        action: SigningAction.TransmitResults,
        status,
        required: REQUIRED,
        signed,
        subject: {
            tally_session_id: STORY_IDS.tallySession,
            package_sha256: "c0a8b41e".padEnd(64, "0"),
            eml_sha256: emlSha256,
            destinations: ["ccs-east", "ccs-north", "ccs-south"],
        },
    })
    return {
        ...panel,
        request: {...panel.request, document_sha256: emlSha256},
        document_url: "https://documents.admin-story.invalid/er.xml",
        document_name: "er_tx-0002.xml",
    }
}

let signingApi: ReturnType<typeof fakeApi>

let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let uploads: ReturnType<typeof storyFetch>

function Fixture(scenario: Scenario) {
    const {roles, onBack} = scenario
    const {permissions} = useStoryGlobals()
    const selected = packageOf(scenario)
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
                    <SigningProvider api={signingApi}>
                        <MiruExportWizard />
                    </SigningProvider>
                </RecordContextProvider>
            </TallyStoryContext>
        </AdminStoryProvider>
    )
}

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
    beforeEach: async ({args}) => {
        const fail = () => {
            if (args.service === "failure") throw new Error("Synthetic Miru service unavailable")
        }
        const packageData = packageOf(args)
        signingApi = fakeApi(
            await transmissionPanel(args.signingRequest ?? SigningRequestStatus.Waiting)
        )
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
                GetSigningRules: () => ({
                    data: {
                        sequent_backend_signing_rule: args.transmitRuleRequired
                            ? [
                                  {
                                      action: SigningAction.TransmitResults,
                                      requirement: "required",
                                      signatures: REQUIRED,
                                      requester_signing: "not-allowed",
                                      expires_minutes: 60,
                                      revision: 1,
                                      updated_by: "configuration-manager",
                                      updated_by_name: "Configuration Manager",
                                      updated_at: "2028-05-01T09:00:00Z",
                                  },
                              ]
                            : [],
                    },
                }),
                SigningGetRequest: async () => ({
                    data: {signingGetRequest: {panel: await signingApi.getRequest(REQUEST_ID)}},
                }),
            },
            // The signing actions are not in the stories' schema, as in the
            // Signatures tab's stories.
            {schema: !args.signingRequest}
        )
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
        await boundary.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const action = (key: string) => i18n.t(`tally.transmissionPackage.actions.${key}`)
/** The calls the wizard makes, besides reading its package's signing request. */
const mutations = () =>
    boundary.calls.map(({name}) => name).filter((name) => name !== "SigningGetRequest")
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
        reason: "The expanded signature upload and logs accordions expose two regions without distinct names.",
        a11y: ["landmark-unique"],
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

const signingTitle = () => i18n.t("signing.results.transmission.title")

export const SignaturesNeededReplaceTheCertificateUpload: Story = {
    args: {roles: MIRU_ROLES, signed: [], signingRequest: SigningRequestStatus.Waiting},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(signingTitle())).toBeVisible()
        await expect(
            canvas.getByText(i18n.t("signing.results.transmission.description", {n: REQUIRED}))
        ).toBeVisible()
        // The 2025 upload of a certificate file and its password is gone.
        expect(canvas.queryByText(i18n.t("tally.uploadTransmissionPackage"))).toBeNull()
        expect(canvasElement.querySelector('input[type="file"]')).toBeNull()
        await canvas.findByText(
            i18n.t("tally.transmissionPackage.signatures.status", {
                signed: 0,
                total: 2,
                minimum: REQUIRED,
            }),
            {exact: false}
        )
        await expect(canvas.getByRole("button", {name: "send transmission package"})).toBeDisabled()
        await expect(canvas.getByText(i18n.t("signing.results.transmission.waiting"))).toBeVisible()

        // The panel of the package's request opens from the wizard.
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("signing.results.openRequest")})
        )
        await waitFor(() => expect(signingApi.getRequest).toHaveBeenCalledWith(REQUEST_ID))
        await expect(await within(document.body).findByText(CODE)).toBeVisible()
        // Not signed yet: nothing to send from the panel.
        expect(
            within(document.body).queryByRole("button", {
                name: i18n.t("signing.results.sendTo", {count: 3}),
            })
        ).toBeNull()
        expect(mutations()).toEqual([])
    },
}

export const SignedPackageIsSentFromThePanel: Story = {
    args: {
        roles: MIRU_ROLES,
        signed: ["sbei-1", "sbei-2"],
        signingRequest: SigningRequestStatus.Executed,
    },
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("signing.results.transmission.signed"))
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "send transmission package"})).toBeEnabled()
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("signing.results.openRequest")})
        )
        const send = await within(document.body).findByRole("button", {
            name: i18n.t("signing.results.sendTo", {count: 3}),
        })
        await userEvent.click(send)
        const dialog = await within(document.body).findByRole("dialog", {
            name: action("send.dialog.title"),
        })
        await userEvent.click(
            within(dialog).getByRole("button", {name: action("send.dialog.confirm")})
        )
        await notified(i18n.t("miruExport.send.success"))
        expect(mutations()).toEqual(["SendTransmissionPackage"])
    },
}

export const AnEndedRequestBringsBackTheCertificateUploadOnceTheRuleIsOff: Story = {
    parameters: signingRegions,
    args: {
        roles: [...MIRU_ROLES, "signing-rules-read"],
        signed: [],
        signingRequest: SigningRequestStatus.Cancelled,
        transmitRuleRequired: false,
    },
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The package's request was cancelled and signatures are no longer
        // needed: the 2025 upload is offered again.
        await expect(
            await canvas.findByText(i18n.t("tally.uploadTransmissionPackage"))
        ).toBeVisible()
        expect(canvas.queryByText(signingTitle())).toBeNull()
        expect(canvas.queryByText(i18n.t("signing.results.transmission.ended"))).toBeNull()
    },
}

export const AnEndedRequestWhileSignaturesAreNeededAsksToRecreate: Story = {
    args: {
        roles: [...MIRU_ROLES, "signing-rules-read"],
        signed: [],
        signingRequest: SigningRequestStatus.Cancelled,
        transmitRuleRequired: true,
    },
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("signing.results.transmission.ended"))
        ).toBeVisible()
        expect(canvas.queryByText(i18n.t("tally.uploadTransmissionPackage"))).toBeNull()
        expect(canvasElement.querySelector('input[type="file"]')).toBeNull()
    },
}
