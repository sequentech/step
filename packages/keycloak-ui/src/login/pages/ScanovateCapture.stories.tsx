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
    fakeServices,
    sceneForFacing,
    syntheticCamera,
} from "../scanovate/fakes"
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

// Form submissions are kept in the page so the checking card stays visible.
const submissions: FormData[] = []
function keepSubmissions() {
    submissions.length = 0
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

/** The whole flow with automatic captures, up to the multipart submission. */
export const AutomaticCapture: Story = {
    args: {
        kcContext: {scanovate: {videoSeconds: 1}},
        services: fakeServices({
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
        const form = submissions[0]
        await expect(form.get("action")).toBe("capture")
        for (const [part, type, name] of [
            ["front", "image/jpeg", "front.jpg"],
            ["back", "image/jpeg", "back.jpg"],
            ["face", "image/jpeg", "face.jpg"],
            ["video", "video/webm", "video.webm"],
        ]) {
            const file = form.get(part)
            await expect(file).toBeInstanceOf(File)
            await expect((file as File).type).toBe(type)
            await expect((file as File).name).toBe(name)
            await expect((file as File).size).toBeGreaterThan(0)
        }
    },
}

// A same-origin blank page stands in for Liveness Plus, and posts its messages.
const livenessContext = (): CaptureContext => ({
    scanovate: {
        liveness: {url: "about:blank", origin: window.location.origin, languages: []},
    },
})

async function postFromLiveness(canvasElement: HTMLElement, message: object) {
    const frame = await waitFor(() => {
        const found = canvasElement.querySelector<HTMLIFrameElement>(".capture-liveness iframe")
        if (found?.contentWindow === null || found === null) throw new Error("no iframe yet")
        return found
    })
    const inFrame = frame.contentWindow as unknown as {Function: FunctionConstructor}
    inFrame.Function("message", "parent.postMessage(message, '*')")(message)
}

export const LivenessIntro: Story = {
    args: {kcContext: livenessContext()},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(
                "We will take photos of your Driver’s License and then check your face with a short guided selfie to confirm it is you. It takes about 2 minutes."
            )
        ).toBeVisible()
        await expect(canvas.getByText("A short selfie check, guided on screen.")).toBeVisible()
        await expect(canvas.queryByText("You holding your ID")).not.toBeInTheDocument()
    },
}

export const LivenessCheck: Story = {
    args: {kcContext: livenessContext(), startAt: CaptureStep.Liveness},
    play: async ({canvasElement}) => {
        const frame = await waitFor(() => {
            const found = canvasElement.querySelector(".capture-liveness iframe")
            if (found === null) throw new Error("no iframe yet")
            return found
        })
        await expect(frame).toHaveAttribute("allow", "camera *; microphone *")
        await expect(frame).toHaveAttribute("title", "Face check")
    },
}

export const LivenessDoneSubmitsTheId: Story = {
    args: {kcContext: livenessContext(), startAt: CaptureStep.Liveness},
    play: async ({canvasElement}) => {
        const stop = keepSubmissions()
        try {
            await postFromLiveness(canvasElement, {type: "done", service_session_id: "s"})
            const canvas = within(canvasElement)
            await expect(
                await canvas.findByRole("heading", {level: 1, name: "Checking your identity"})
            ).toBeVisible()
            await expect(canvas.getByText("Photos and face check received")).toBeVisible()
            await waitFor(() => expect(submissions).toHaveLength(1))
            const data = submissions[0]
            await expect(data.get("action")).toBe("capture")
            await expect(data.get("front")).toBeInstanceOf(File)
            await expect(data.get("back")).toBeInstanceOf(File)
            await expect(data.has("face")).toBe(false)
            await expect(data.has("video")).toBe(false)
        } finally {
            stop()
        }
    },
}

export const LivenessFailed: Story = {
    args: {kcContext: livenessContext(), startAt: CaptureStep.Liveness},
    play: async ({canvasElement}) => {
        await postFromLiveness(canvasElement, {type: "error", error_code: 1006})
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "The face check didn’t finish"})
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await waitFor(() =>
            expect(canvasElement.querySelector(".capture-liveness iframe")).not.toBeNull()
        )
    },
}

export const LivenessMessagesFromOtherOriginsAreIgnored: Story = {
    args: {
        kcContext: {
            scanovate: {
                liveness: {
                    url: "about:blank",
                    origin: "https://liveness.example.com",
                    languages: [],
                },
            },
        },
        startAt: CaptureStep.Liveness,
    },
    play: async ({canvasElement}) => {
        await postFromLiveness(canvasElement, {type: "done"})
        await new Promise((resolve) => setTimeout(resolve, 200))
        await expect(canvasElement.querySelector(".capture-liveness iframe")).not.toBeNull()
    },
}
