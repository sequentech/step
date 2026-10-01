// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {DeepPartial} from "keycloakify/tools/DeepPartial"
import {createKcPageStory} from "../KcPageStory"
import {ScanovateSide, type KcContext} from "../KcContext"
import {CaptureEnvironmentContext} from "../scanovate/environment"
import {
    SyntheticScene,
    failingCamera,
    fakeLiveness,
    fakeServices,
    fakeUploads,
    sceneForFacing,
    syntheticCamera,
} from "../scanovate/fakes"
import {CapturePart, type CaptureUpload} from "../scanovate/form"
import {LivenessRejection, LivenessStatus, type LivenessConnector} from "../scanovate/livenessApi"
import {expectStickyActions} from "../scanovate/stickyActions"
import {UploadRejection} from "../scanovate/uploads"
import {CaptureStep, DocumentStatus, FaceStatus, type CaptureServices} from "../scanovate/types"

const {KcPageStory} = createKcPageStory({pageId: "scanovate-capture.ftl"})

type CaptureContext = DeepPartial<Extract<KcContext, {pageId: "scanovate-capture.ftl"}>>

type Args = {
    locale?: "en" | "es"
    kcContext?: CaptureContext
    services?: CaptureServices
    startAt?: CaptureStep
}

function CaptureStory({locale, kcContext, services = fakeServices({}), startAt}: Args) {
    return (
        <CaptureEnvironmentContext.Provider value={{services, startAt}}>
            <KcPageStory locale={locale} kcContext={kcContext} />
        </CaptureEnvironmentContext.Provider>
    )
}

const meta = {
    title: "Keycloak/Scanovate capture",
    component: CaptureStory,
} satisfies Meta<typeof CaptureStory>

export default meta

type Story = StoryObj<typeof meta>

// Form submissions are kept in the page so the checking card stays visible,
// and the uploads before them are recorded.
const submissions: FormData[] = []
const uploads: CaptureUpload[] = []
function keepSubmissions() {
    submissions.length = 0
    uploads.length = 0
    const keep = (event: SubmitEvent) => {
        event.preventDefault()
        if (event.target instanceof HTMLFormElement) {
            submissions.push(new FormData(event.target))
        }
    }
    document.addEventListener("submit", keep, true)
    return () => document.removeEventListener("submit", keep, true)
}

const phone = () => window.matchMedia("(max-width: 720px)").matches

// Phones list the finished parts as chips, larger screens as a list of steps.
async function expectDone(canvasElement: HTMLElement, chip: string, step: string) {
    const canvas = within(canvasElement)
    if (phone()) {
        await expect(canvas.getByText(chip, {selector: ".capture-chips li"})).toBeVisible()
    } else {
        await expect(canvas.getByText(step, {selector: ".desk-steps li.done"})).toBeVisible()
    }
}

async function expectCounter(canvasElement: HTMLElement, counter: string) {
    if (phone()) {
        await expect(within(canvasElement).getByText(counter)).toBeVisible()
    }
}

const holdingId = syntheticCamera((facing) => {
    const scene = sceneForFacing(facing)
    return scene === SyntheticScene.Face ? SyntheticScene.FaceWithDocument : scene
})

async function pill(canvasElement: HTMLElement, text: string) {
    await waitFor(
        () => expect(canvasElement.querySelector(".capture-pill")).toHaveTextContent(text),
        {timeout: 5000}
    )
}

export const Intro: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Verify your identity"})
        ).toBeVisible()
        await expect(canvas.getByText("Step 3 of 4 · Verify identity")).toBeVisible()
        await expect(canvas.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "3")
        await expect(
            canvas.getByText(
                "We will take photos of your Driver’s License and a short video of your face to confirm it is you. It takes about 2 minutes."
            )
        ).toBeVisible()
        await expect(canvas.getByText("Front and back of your ID")).toBeVisible()
        await expect(canvas.getByText("Have your Driver’s License with you.")).toBeVisible()
        await expect(canvas.queryByText(/more time/)).not.toBeInTheDocument()
        await expectStickyActions(canvas.getByRole("button", {name: "Start"}))
    },
}

export const IntroAfterInvalidCapture: Story = {
    args: {
        kcContext: {
            message: {
                type: "error",
                summary: "We couldn’t use the photos or the video. Please take them again.",
            },
            scanovate: {attemptsLeft: 2},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "We couldn’t use the photos or the video."
        )
        await expect(canvas.getByText("You can try 2 more times.")).toBeVisible()
    },
}

export const IntroGenericDocument: Story = {
    args: {kcContext: {scanovate: {documentType: "default", sides: [ScanovateSide.Front]}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText("Your ID")).toBeVisible()
        await expect(canvas.getByText("Have your ID with you.")).toBeVisible()
    },
}

export const IntroSpanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Verifique su identidad"})
        ).toBeVisible()
        await expect(canvas.getByText("Paso 3 de 4 · Verificar identidad")).toBeVisible()
    },
}

export const FrontGlare: Story = {
    args: {
        startAt: CaptureStep.Front,
        services: fakeServices({document: {statuses: [DocumentStatus.Glare]}}),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Front of your ID"})
        ).toBeVisible()
        await pill(canvasElement, "Tilt your ID to remove the glare")
        await expect(canvas.getByRole("button", {name: "Take photo"})).toBeDisabled()
        await expectCounter(canvasElement, "Driver’s License · 1 of 4")
    },
}

const holdingStill = fakeServices({
    document: {statuses: [DocumentStatus.HoldStill], stability: 0.6},
})

export const FrontHoldStill: Story = {
    args: {startAt: CaptureStep.Front, services: holdingStill},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Front of your ID"})
        await pill(canvasElement, "Hold still, capturing…")
        await waitFor(() => expect(canvas.getByRole("button", {name: "Take photo"})).toBeEnabled())
    },
}

export const ShutterTakesPhoto: Story = {
    args: {startAt: CaptureStep.Front, services: holdingStill},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Front of your ID"})
        const shutter = canvas.getByRole("button", {name: "Take photo"})
        await waitFor(() => expect(shutter).toBeEnabled())
        await userEvent.click(shutter)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Back of your ID"})
        ).toBeVisible()
        await expectDone(canvasElement, "Front", "Front of your ID")
    },
}

export const BackTurnOver: Story = {
    args: {
        startAt: CaptureStep.Back,
        services: fakeServices({document: {statuses: [DocumentStatus.NoDocument]}}),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Back of your ID"})
        await pill(canvasElement, "Turn your ID over")
        await expectCounter(canvasElement, "Driver’s License · 2 of 4")
    },
}

export const FaceHoldStill: Story = {
    args: {
        startAt: CaptureStep.Face,
        services: fakeServices({face: {statuses: [FaceStatus.HoldStill], stability: 0.62}}),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Your face"})
        await pill(canvasElement, "Look at the camera and hold still")
        await expectDone(canvasElement, "Back", "Back of your ID")
        await expect(canvasElement.querySelector(".capture-video")).toHaveClass("mirrored")
    },
}

export const FaceTooDark: Story = {
    args: {
        startAt: CaptureStep.Face,
        services: fakeServices({face: {statuses: [FaceStatus.TooDark]}}),
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1, name: "Your face"})
        await pill(canvasElement, "Find a brighter place")
    },
}

export const VideoNeedsId: Story = {
    args: {
        startAt: CaptureStep.Video,
        services: fakeServices({
            camera: holdingId,
            face: {statuses: [FaceStatus.Ready]},
            document: {statuses: [DocumentStatus.NoDocument]},
        }),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "You holding your ID"})
        await pill(canvasElement, "Hold your ID next to your face")
        await expect(canvasElement.querySelector(".capture-rec")).toBeNull()
    },
}

export const VideoRecording: Story = {
    args: {
        startAt: CaptureStep.Video,
        kcContext: {scanovate: {videoSeconds: 60}},
        services: fakeServices({
            camera: holdingId,
            face: {statuses: [FaceStatus.Ready]},
            document: {statuses: [DocumentStatus.HoldStill]},
        }),
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await pill(canvasElement, "Recording, keep still")
        await waitFor(() =>
            expect(canvasElement.querySelector(".capture-rec")).toHaveTextContent("REC 0:0")
        )
        await expectDone(canvasElement, "Face", "Your face")
    },
}

export const HelpSheet: Story = {
    args: {startAt: CaptureStep.Front},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Front of your ID"})
        await userEvent.click(canvas.getByRole("button", {name: "Help"}))
        const dialog = await within(document.body).findByRole("dialog", {
            name: "Tips for this step",
        })
        await waitFor(() =>
            expect(
                within(dialog).getByText("Keep all four corners inside the frame.")
            ).toBeVisible()
        )
        await userEvent.click(within(dialog).getByRole("button", {name: "Got it"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const StopReturnsToIntro: Story = {
    args: {startAt: CaptureStep.Face},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Your face"})
        await userEvent.click(canvas.getByRole("button", {name: "Stop and go back"}))
        const dialog = await within(document.body).findByRole("dialog", {
            name: "Stop verifying your identity?",
        })
        await userEvent.click(within(dialog).getByRole("button", {name: "Stop"}))
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Verify your identity"})
        ).toBeVisible()
    },
}

export const CameraDenied: Story = {
    args: {services: fakeServices({camera: failingCamera("NotAllowedError")})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Start"}))
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Allow access to your camera"})
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Back to start"}))
        await canvas.findByRole("heading", {level: 1, name: "Verify your identity"})
    },
}

export const CameraNotFound: Story = {
    args: {services: fakeServices({camera: failingCamera("NotFoundError")})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Start"}))
        await canvas.findByRole("heading", {level: 1, name: "We couldn’t find a camera"})
        await expect(canvas.getByRole("button", {name: "Try again"})).toBeVisible()
        await expectStickyActions(canvas.getByRole("button", {name: "Try again"}))
    },
}

export const CameraInUse: Story = {
    args: {services: fakeServices({camera: failingCamera("NotReadableError")})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Start"}))
        await canvas.findByRole("heading", {level: 1, name: "Your camera is in use"})
    },
}

export const InsecureContext: Story = {
    args: {services: fakeServices({camera: failingCamera("InsecureContextError")})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Start"}))
        await canvas.findByRole("heading", {
            level: 1,
            name: "The camera can’t be used on this page",
        })
    },
}

export const AnalyzerFailed: Story = {
    args: {services: fakeServices({analyzersFail: true})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Start"}))
        await canvas.findByRole("heading", {
            level: 1,
            name: "We couldn’t prepare the camera check",
        })
    },
}

export const Checking: Story = {
    args: {
        startAt: CaptureStep.Video,
        kcContext: {scanovate: {videoSeconds: 1}},
        services: fakeServices({
            face: {statuses: [FaceStatus.Ready]},
            document: {statuses: [DocumentStatus.HoldStill]},
        }),
    },
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "Checking your identity"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await expect(canvas.getByText("Checking your ID and your face")).toBeVisible()
        await waitFor(() => expect(submissions).toHaveLength(1))
    },
}

/** The whole flow with automatic captures, up to the uploads and the submission. */
export const AutomaticCapture: Story = {
    args: {
        kcContext: {scanovate: {videoSeconds: 1}},
        services: fakeServices({
            uploads: fakeUploads(uploads),
            document: {
                statuses: [
                    DocumentStatus.NoDocument,
                    DocumentStatus.NoDocument,
                    DocumentStatus.Ready,
                ],
            },
            face: {statuses: [FaceStatus.Ready]},
        }),
    },
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Start"}))
        await canvas.findByRole(
            "heading",
            {level: 1, name: "Checking your identity"},
            {timeout: 12000}
        )
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect([...submissions[0].keys()]).toEqual(["action"])
        await expect(submissions[0].get("action")).toBe("capture")
        // The voter's face reaches Keycloak from Liveness Plus, never from the page
        await expect(uploads.map(({part}) => part)).toEqual([
            CapturePart.Front,
            CapturePart.Back,
            CapturePart.Holding,
        ])
        for (const {blob} of uploads) await expect(blob.size).toBeGreaterThan(0)
        await expect(uploads[2].blob.type).toBe("image/jpeg")
    },
}

export const UploadFailed: Story = {
    args: {
        startAt: CaptureStep.Video,
        kcContext: {scanovate: {videoSeconds: 1}},
        services: fakeServices({
            face: {statuses: [FaceStatus.Ready]},
            document: {statuses: [DocumentStatus.HoldStill]},
            uploads: fakeUploads([], UploadRejection.Failed),
        }),
    },
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "We couldn’t send your photos"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await expect(canvas.getByText("Check your connection and try again.")).toBeVisible()
        await expect(submissions).toHaveLength(0)
        await expect(canvas.getByRole("button", {name: "Try again"})).toBeVisible()
    },
}

export const UploadExpired: Story = {
    args: {
        startAt: CaptureStep.Video,
        kcContext: {scanovate: {videoSeconds: 1}},
        services: fakeServices({
            face: {statuses: [FaceStatus.Ready]},
            document: {statuses: [DocumentStatus.HoldStill]},
            uploads: fakeUploads([], UploadRejection.InvalidToken),
        }),
    },
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "Start the verification again"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start over"})).toBeVisible()
        await expect(submissions).toHaveLength(0)
    },
}

// Liveness Plus is faked: it answers the face frames with the given statuses.
const livenessContext = (): CaptureContext => ({
    scanovate: {
        liveness: {url: "https://localhost/biometric/liveness", token: "token", caseId: "case"},
    },
})

const faceReady = (liveness: LivenessConnector) =>
    fakeServices({face: {statuses: [FaceStatus.Ready]}, liveness})

export const LivenessIntro: Story = {
    args: {kcContext: livenessContext()},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "We will take photos of your Driver’s License and a short video of your face to confirm it is you. It takes about 2 minutes."
            )
        ).toBeVisible()
        await expect(canvas.getByText("Your face")).toBeVisible()
        await expect(canvas.getByText("You holding your ID")).toBeVisible()
    },
}

export const LivenessAsksForABetterFrame: Story = {
    args: {
        kcContext: livenessContext(),
        startAt: CaptureStep.Face,
        services: faceReady(fakeLiveness([LivenessStatus.FaceTooSmall])),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Your face"})
        await pill(canvasElement, "Checking your face, hold still…")
        await pill(canvasElement, "Move a little closer")
        await expect(canvas.getByRole("heading", {level: 1, name: "Your face"})).toBeVisible()
    },
}

export const LivenessScannedMovesOn: Story = {
    args: {
        kcContext: livenessContext(),
        startAt: CaptureStep.Face,
        services: faceReady(fakeLiveness([LivenessStatus.Ok, LivenessStatus.ScanCompleted])),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "You holding your ID"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await expectDone(canvasElement, "Face", "Your face")
    },
}

export const LivenessHoldingSubmitsPhoto: Story = {
    args: {
        kcContext: {
            ...livenessContext(),
            scanovate: {...livenessContext().scanovate, videoSeconds: 1},
        },
        startAt: CaptureStep.Video,
        services: fakeServices({
            camera: holdingId,
            face: {statuses: [FaceStatus.Ready]},
            document: {statuses: [DocumentStatus.HoldStill]},
            uploads: fakeUploads(uploads),
        }),
    },
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "Checking your identity"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await expect(canvas.getByText("Photos and face check received")).toBeVisible()
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect([...submissions[0].keys()]).toEqual(["action"])
        await expect(uploads.map(({part}) => part)).toEqual([
            CapturePart.Front,
            CapturePart.Back,
            CapturePart.Holding,
        ])
        const holding = uploads[2].blob
        await expect(holding.type).toBe("image/jpeg")
        await expect(holding.size).toBeGreaterThan(0)
    },
}

export const LivenessFailed: Story = {
    args: {
        kcContext: livenessContext(),
        startAt: CaptureStep.Face,
        services: faceReady(fakeLiveness([LivenessStatus.ServerError])),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "The face check didn’t finish"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await canvas.findByRole("heading", {level: 1, name: "Your face"})
    },
}

export const LivenessTokenUsedUp: Story = {
    args: {
        kcContext: livenessContext(),
        startAt: CaptureStep.Face,
        services: faceReady(fakeLiveness([], LivenessRejection.InvalidToken)),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole(
                "heading",
                {level: 1, name: "Start the verification again"},
                {timeout: 8000}
            )
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start over"})).toBeVisible()
    },
}

// The whole liveness capture at a pace people can follow, for walkthroughs and
// reviews: the ID is found after a few hints, the face needs one better frame,
// and the voter then holds the ID next to their face.
const walkthroughDocument = [
    ...Array<DocumentStatus>(25).fill(DocumentStatus.NoDocument),
    ...Array<DocumentStatus>(25).fill(DocumentStatus.TooFar),
    ...Array<DocumentStatus>(25).fill(DocumentStatus.Glare),
    ...Array<DocumentStatus>(25).fill(DocumentStatus.HoldStill),
    DocumentStatus.Ready,
]
const walkthroughFace = [
    ...Array<FaceStatus>(35).fill(FaceStatus.NoFace),
    ...Array<FaceStatus>(35).fill(FaceStatus.TooFar),
    ...Array<FaceStatus>(35).fill(FaceStatus.HoldStill),
    FaceStatus.Ready,
]
const holding = () =>
    [...document.querySelectorAll("h1, h2")].some((heading) =>
        heading.textContent?.includes("You holding your ID")
    )

export const Walkthrough: Story = {
    args: {
        kcContext: {
            themeName: "sequent-ui-voting",
            scanovate: {...livenessContext().scanovate, videoSeconds: 4},
        },
        services: fakeServices({
            camera: syntheticCamera((facing) => {
                const scene = sceneForFacing(facing)
                return scene === SyntheticScene.Face && holding()
                    ? SyntheticScene.FaceWithDocument
                    : scene
            }),
            document: {statuses: walkthroughDocument, stability: 0.5},
            face: {statuses: walkthroughFace, stability: 0.5},
            liveness: fakeLiveness([LivenessStatus.FaceTooSmall, LivenessStatus.ScanCompleted]),
        }),
    },
    beforeEach: keepSubmissions,
}
