// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {DeepPartial} from "keycloakify/tools/DeepPartial"
import {createKcPageStory} from "../KcPageStory"
import type {KcContext} from "../KcContext"
import {ScanovateErrorCode} from "../scanovate/types"

const {KcPageStory} = createKcPageStory({pageId: "scanovate-capture.ftl"})

type CaptureContext = DeepPartial<Extract<KcContext, {pageId: "scanovate-capture.ftl"}>>

// Stand-ins for Scanovate's OCR and Liveness Plus pages: they post the same
// messages to the parent window as the real ones, when a button is pressed.
const FAKE_ERRORS = [
    ScanovateErrorCode.CameraPermissionBlocked,
    ScanovateErrorCode.SessionExpired,
    ScanovateErrorCode.ServerError,
    ScanovateErrorCode.UserPressedClose,
    ScanovateErrorCode.InvalidToken,
]

function fakeScanovatePage(service: string): string {
    const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Fake Scanovate ${service}</title>
</head>
<body style="font-family: sans-serif; margin: 16px">
<h1>Fake Scanovate ${service}</h1>
<button type="button" data-message="done">Finish</button>
<button type="button" data-message="hide-iframe">Hide</button>
${FAKE_ERRORS.map((code) => `<button type="button" data-code="${code}">Fail ${code}</button>`).join("\n")}
<script>
    const session = "fake-${service}-session"
    for (const message of ["js-load-success", "css-load-success", "iframe-loaded", "ui-loaded"]) {
        window.parent.postMessage(message, "*")
    }
    window.parent.postMessage({type: "init", service_session_id: session, message: "init"}, "*")
    for (const button of document.querySelectorAll("button")) {
        button.addEventListener("click", () => {
            const code = button.dataset.code
            if (code !== undefined) {
                window.parent.postMessage({
                    type: "error",
                    service_session_id: session,
                    message: "error",
                    error_code: Number(code),
                    error_message: "fake error",
                    result_path: "/result",
                }, "*")
            } else if (button.dataset.message === "done") {
                window.parent.postMessage({
                    type: "done",
                    service_session_id: session,
                    message: "process completed successfully",
                    result_path: "/result",
                    session_data: {},
                }, "*")
            } else {
                window.parent.postMessage(button.dataset.message, "*")
            }
        })
    }
</script>
</body>
</html>`
    // A blob URL keeps the page's origin, as Scanovate's pages under /biometric/ do.
    return URL.createObjectURL(new Blob([html], {type: "text/html"}))
}

const FAKE_OCR = fakeScanovatePage("OCR")
const FAKE_LIVENESS = fakeScanovatePage("Liveness Plus")

type Args = {
    locale?: "en" | "es"
    kcContext?: CaptureContext
}

function CaptureStory({locale, kcContext = {}}: Args) {
    return (
        <KcPageStory
            locale={locale}
            kcContext={{
                ...kcContext,
                scanovate: {
                    ocr: {url: FAKE_OCR},
                    liveness: {url: FAKE_LIVENESS},
                    ...kcContext.scanovate,
                },
            }}
        />
    )
}

const meta = {
    title: "Keycloak/Scanovate capture",
    component: CaptureStory,
} satisfies Meta<typeof CaptureStory>

export default meta

type Story = StoryObj<typeof meta>

// Form submissions are kept in the page so the cards stay visible.
const submissions: FormData[] = []
function keepSubmissions() {
    submissions.length = 0
    const keep = (event: SubmitEvent) => {
        event.preventDefault()
        if (event.target instanceof HTMLFormElement) {
            submissions.push(new FormData(event.target, event.submitter))
        }
    }
    document.addEventListener("submit", keep, true)
    return () => document.removeEventListener("submit", keep, true)
}

async function scanovateFrame(canvasElement: HTMLElement, url: string) {
    let frame: HTMLIFrameElement | null = null
    await waitFor(
        () => {
            frame = canvasElement.querySelector<HTMLIFrameElement>(".scanovate-frame iframe")
            expect(frame?.getAttribute("src")).toBe(url)
            expect(frame?.contentDocument?.querySelector("button")).not.toBeNull()
        },
        {timeout: 5000}
    )
    return frame as unknown as HTMLIFrameElement
}

/** Presses a button of the fake Scanovate page, which posts its message to the page. */
async function press(canvasElement: HTMLElement, url: string, name: string) {
    const frame = await scanovateFrame(canvasElement, url)
    const button = within(frame.contentDocument!.body).getByRole("button", {name})
    button.click()
    return frame
}

async function heading(canvasElement: HTMLElement, name: string) {
    return within(canvasElement).findByRole("heading", {level: 1, name}, {timeout: 5000})
}

/**
 * Every camera step covers the whole window, so that the camera is what the
 * voter sees from the start and the steps don't change layout between them.
 * ID R&D's face capture also needs it: it takes a frame much smaller than the
 * window for open DevTools and refuses to start.
 */
async function expectFillsWindow(canvasElement: HTMLElement, selector: string) {
    await waitFor(() => {
        const element = canvasElement.querySelector(selector)
        expect(element).not.toBeNull()
        const box = element!.getBoundingClientRect()
        expect([box.left, box.top, box.width, box.height]).toEqual([
            0,
            0,
            window.innerWidth,
            window.innerHeight,
        ])
    })
}

export const DocumentStep: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await heading(canvasElement, "Scan your ID")).toBeInTheDocument()
        await expect(
            canvas.getByText(
                "Follow the instructions on the screen to scan your Driver’s License. Then we will check your face."
            )
        ).toBeInTheDocument()
        await expect(canvas.queryByText(/more time/)).not.toBeInTheDocument()
        const frame = await scanovateFrame(canvasElement, FAKE_OCR)
        await expectFillsWindow(canvasElement, ".scanovate-window")
        await expectFillsWindow(canvasElement, ".scanovate-window iframe")
        // Nothing of the card is left around the camera.
        await expect(canvasElement.querySelector(".auth-card")).toBeNull()
        await expect(frame).toHaveAttribute("allow", "camera *; microphone *")
        await expect(frame).toHaveAccessibleName("ID scan")
        await waitFor(() => expect(frame.ownerDocument.activeElement).toBe(frame))
    },
}

/**
 * A device with two cameras offers the other one for the document scan:
 * Scanovate's page opens the rear camera by itself, which is the wrong side
 * on a tablet standing in front of the voter. The choice opens Scanovate's
 * page again, told which camera to open, and stays on the device.
 */
export const DocumentStepOtherCamera: Story = {
    beforeEach: () => {
        const CHOICE = "sequent.scanovate.documentCamera"
        window.localStorage.removeItem(CHOICE)
        const devices = navigator.mediaDevices
        const listed = devices.enumerateDevices
        devices.enumerateDevices = async () =>
            [{kind: "videoinput"}, {kind: "audioinput"}, {kind: "videoinput"}] as MediaDeviceInfo[]
        return () => {
            devices.enumerateDevices = listed
            window.localStorage.removeItem(CHOICE)
        }
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const opensCamera = (frame: HTMLIFrameElement) =>
            String(frame.contentWindow!.navigator.mediaDevices.getUserMedia)
        const rear = await scanovateFrame(canvasElement, FAKE_OCR)
        const toFront = await canvas.findByRole("button", {name: "Use the front camera"})
        // The rear camera is the page's own choice: its camera access is untouched.
        await expect(opensCamera(rear)).toContain("[native code]")

        await userEvent.click(toFront)
        await expect(await canvas.findByRole("button", {name: "Use the rear camera"})).toBeVisible()
        const front = await scanovateFrame(canvasElement, FAKE_OCR)
        await expect(front).not.toBe(rear)
        await waitFor(() => expect(opensCamera(front)).not.toContain("[native code]"))
        await expect(window.localStorage.getItem("sequent.scanovate.documentCamera")).toBe("front")

        await userEvent.click(canvas.getByRole("button", {name: "Use the rear camera"}))
        await canvas.findByRole("button", {name: "Use the front camera"})
        const rearAgain = await scanovateFrame(canvasElement, FAKE_OCR)
        await expect(opensCamera(rearAgain)).toContain("[native code]")
        await expect(window.localStorage.getItem("sequent.scanovate.documentCamera")).toBe("rear")
    },
}

export const DocumentStepAfterRetry: Story = {
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
        await heading(canvasElement, "Scan your ID")
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "We couldn’t use the photos or the video."
        )
        await expect(canvas.getByText("You can try 2 more times.")).toBeVisible()
    },
}

export const DocumentStepGenericDocument: Story = {
    args: {kcContext: {scanovate: {documentType: "default"}}},
    play: async ({canvasElement}) => {
        await heading(canvasElement, "Scan your ID")
        await expect(within(canvasElement).getByText(/to scan your ID\./)).toBeVisible()
    },
}

export const DocumentStepSpanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await heading(canvasElement, "Escanee su documento")).toBeInTheDocument()
        await expect(canvas.getByText(/para escanear su permiso de conducir/)).toBeInTheDocument()
    },
}

export const WithStepBack: Story = {
    args: {kcContext: {stepBack: true}},
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The camera covers the window: closing it gives the way back.
        await press(canvasElement, FAKE_OCR, `Fail ${ScanovateErrorCode.UserPressedClose}`)
        await heading(canvasElement, "The scan was closed")
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect(submissions[0].get("back")).toBe("true")
    },
}

/** The OCR page finished: the Liveness Plus page takes its place. */
export const FaceStep: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await heading(canvasElement, "Scan your ID")
        await press(canvasElement, FAKE_OCR, "Finish")
        await expect(await heading(canvasElement, "Check your face")).toBeInTheDocument()
        await expect(
            canvas.getByText(
                "Your ID is scanned. Follow the instructions on the screen so we can confirm it is you in front of the camera."
            )
        ).toBeInTheDocument()
        const frame = await scanovateFrame(canvasElement, FAKE_LIVENESS)
        await expect(frame).toHaveAccessibleName("Face check")
        await expectFillsWindow(canvasElement, ".scanovate-window")
        await expectFillsWindow(canvasElement, ".scanovate-window iframe")
    },
}

/** Both pages finished: the page only moves the flow on, Keycloak has the results. */
export const Checking: Story = {
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await press(canvasElement, FAKE_OCR, "Finish")
        await heading(canvasElement, "Check your face")
        await press(canvasElement, FAKE_LIVENESS, "Finish")
        await expect(await heading(canvasElement, "Checking your identity")).toBeVisible()
        await expect(canvas.getByText("ID scan and face check finished")).toBeVisible()
        await expect(canvas.getByText("Checking your ID and your face")).toBeVisible()
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect([...submissions[0].entries()]).toEqual([["action", "capture"]])
    },
}

/** Only the embedded page that is showing moves the capture on. */
export const IgnoresOtherMessages: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await heading(canvasElement, "Scan your ID")
        window.postMessage(
            {type: "done", service_session_id: "forged", message: "process completed"},
            "*"
        )
        await press(canvasElement, FAKE_OCR, "Hide")
        await new Promise((resolve) => setTimeout(resolve, 300))
        await expect(canvas.getByRole("heading", {level: 1})).toHaveTextContent("Scan your ID")
    },
}

export const CameraProblem: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const first = await press(
            canvasElement,
            FAKE_OCR,
            `Fail ${ScanovateErrorCode.CameraPermissionBlocked}`
        )
        await expect(await heading(canvasElement, "We couldn’t use your camera")).toBeVisible()
        await expect(canvas.getByText(/Allow camera access for this site/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await heading(canvasElement, "Scan your ID")
        const again = await scanovateFrame(canvasElement, FAKE_OCR)
        await expect(again).not.toBe(first)
    },
}

/** Closing the Liveness Plus page offers to open it again, on the same step. */
export const ClosedOnFaceStep: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await press(canvasElement, FAKE_OCR, "Finish")
        await heading(canvasElement, "Check your face")
        await press(canvasElement, FAKE_LIVENESS, `Fail ${ScanovateErrorCode.UserPressedClose}`)
        await expect(await heading(canvasElement, "The scan was closed")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await expect(await heading(canvasElement, "Check your face")).toBeVisible()
        await scanovateFrame(canvasElement, FAKE_LIVENESS)
    },
}

export const SessionExpired: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await press(canvasElement, FAKE_OCR, `Fail ${ScanovateErrorCode.SessionExpired}`)
        await expect(await heading(canvasElement, "The scan timed out")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Try again"})).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start over"})).toBeVisible()
    },
}

export const GenericProblem: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await press(canvasElement, FAKE_OCR, `Fail ${ScanovateErrorCode.ServerError}`)
        await expect(await heading(canvasElement, "The scan didn’t finish")).toBeVisible()
        await expect(
            canvas.getByText("Check your internet connection and try again.")
        ).toBeVisible()
    },
}

/** An invalid token can only be replaced by Keycloak: start over, without using an attempt. */
export const InvalidToken: Story = {
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await press(canvasElement, FAKE_OCR, `Fail ${ScanovateErrorCode.InvalidToken}`)
        await expect(await heading(canvasElement, "Start the verification again")).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Try again"})).not.toBeInTheDocument()
        await userEvent.click(canvas.getByRole("button", {name: "Start over"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect([...submissions[0].entries()]).toEqual([["action", "retry"]])
    },
}

/** A voter stuck in a Scanovate page closes it, and can ask for new tokens. */
export const StartOverDuringStep: Story = {
    beforeEach: keepSubmissions,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await press(canvasElement, FAKE_OCR, `Fail ${ScanovateErrorCode.UserPressedClose}`)
        await heading(canvasElement, "The scan was closed")
        await userEvent.click(canvas.getByRole("button", {name: "Start over"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect([...submissions[0].entries()]).toEqual([["action", "retry"]])
    },
}

// The holding photo: Keycloak's page takes it with its own camera and uploads
// it with the page's one-time token before submitting the capture.
const HOLDING = {
    url: "/realms/r/identity-verification/capture/holding",
    token: "page-token",
    seconds: 1,
}
const withHolding = {scanovate: {holding: HOLDING}}

/**
 * A front camera that films a test card, still unless `moving`, and the
 * uploads the page makes.
 */
type Uploads = {url: string; init: RequestInit}[]
const uploads: Uploads = []
function fakeCameraAndUploads(status = 200, camera = true, moving = false) {
    uploads.length = 0
    const devices = navigator.mediaDevices
    const realGetUserMedia = devices.getUserMedia
    const realFetch = window.fetch
    const canvas = document.createElement("canvas")
    canvas.width = 640
    canvas.height = 480
    const paint = canvas.getContext("2d")!
    let tick = 0
    // Repainted all the time, as a camera delivers frames; only `moving` changes them.
    const timer = setInterval(() => {
        tick++
        const x = moving ? (tick * 97) % 460 : 380
        paint.fillStyle = "#0d796a"
        paint.fillRect(0, 0, canvas.width, canvas.height)
        paint.fillStyle = "#ffffff"
        paint.fillRect(x, 160, 180, 240)
    }, 50)
    devices.getUserMedia = async () => {
        if (!camera) throw new DOMException("Permission denied", "NotAllowedError")
        return canvas.captureStream(20)
    }
    window.fetch = async (input, init) => {
        const url = typeof input === "string" ? input : input.toString()
        if (url === HOLDING.url) {
            uploads.push({url, init: init ?? {}})
            return new Response("{}", {status})
        }
        return realFetch(input, init)
    }
    const cleanSubmissions = keepSubmissions()
    return () => {
        clearInterval(timer)
        devices.getUserMedia = realGetUserMedia
        window.fetch = realFetch
        cleanSubmissions()
    }
}

async function throughScanovate(canvasElement: HTMLElement) {
    await press(canvasElement, FAKE_OCR, "Finish")
    await heading(canvasElement, "Check your face")
    await press(canvasElement, FAKE_LIVENESS, "Finish")
    return heading(canvasElement, "Take a photo holding your ID")
}

/** The photo the page takes by itself once the image held still. */
async function autoPhoto(canvasElement: HTMLElement) {
    return within(canvasElement).findByRole(
        "img",
        {name: "Your photo holding your ID"},
        {timeout: 8000}
    )
}

/**
 * After the face check, the voter holds still with their ID: the page counts
 * down, takes the photo by itself, and the voter checks it and sends it.
 */
export const HoldingStep: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await throughScanovate(canvasElement)).toBeVisible()
        await expect(
            canvas.getByText(
                "Hold your Driver’s License open next to your face, with both your face and its photo in view, and keep still: the photo is taken by itself."
            )
        ).toBeVisible()
        await expect(canvas.getByLabelText("Camera")).toBeVisible()
        // The same layout as Scanovate's two steps: the camera over the whole window.
        await expectFillsWindow(canvasElement, ".scanovate-window")
        await expectFillsWindow(canvasElement, ".scanovate-window video")
        await expect(canvasElement.querySelector(".auth-card")).toBeNull()
        await expect(canvas.queryByRole("button", {name: "Take photo"})).toBeNull()
        await expect(await canvas.findByText(/^Hold still: \d$/, {}, {timeout: 5000})).toBeVisible()
        await expect(canvas.getByRole("progressbar", {name: "Holding still"})).toBeVisible()
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))

        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect([...submissions[0].entries()]).toEqual([["action", "capture"]])
        await expect(uploads).toHaveLength(1)
        const {init} = uploads[0]
        await expect(init.method).toBe("PUT")
        await expect(init.headers).toEqual({
            "X-Capture-Token": "page-token",
            "Content-Type": "image/jpeg",
        })
        const body = init.body as Blob
        await expect(body.type).toBe("image/jpeg")
        await expect(body.size).toBeGreaterThan(0)
    },
}

/** Closing the holding photo, as Scanovate's pages allow, offers to open it again. */
export const HoldingClosed: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(200, true, true),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Close"}))
        await expect(await heading(canvasElement, "The scan was closed")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Start over"})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await expect(await heading(canvasElement, "Take a photo holding your ID")).toBeVisible()
        await expect(uploads).toHaveLength(0)
    },
}

/** The photo to check keeps the layout: the photo over the whole window. */
export const HoldingReview: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await expectFillsWindow(canvasElement, ".scanovate-window")
        await expectFillsWindow(canvasElement, ".scanovate-window img")
        await expect(canvas.getByRole("button", {name: "Use this photo"})).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Take it again"})).toBeVisible()
    },
}

/** While the image moves, the page waits and no photo is taken. */
export const HoldingWhileMoving: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(200, true, true),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await expect(
            await canvas.findByText("Hold your ID next to your face and keep still.")
        ).toBeVisible()
        await new Promise((resolve) => setTimeout(resolve, 2500))
        await expect(canvas.queryByRole("img", {name: "Your photo holding your ID"})).toBeNull()
        await expect(canvas.getByRole("progressbar", {name: "Holding still"})).toHaveAttribute(
            "aria-valuenow",
            "0"
        )
        await expect(uploads).toHaveLength(0)
    },
}

/** A photo the voter doesn't like is taken again, without uploading it. */
export const HoldingRetake: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await autoPhoto(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Take it again"}))
        await expect(canvas.getByLabelText("Camera")).toBeVisible()
        await expect(canvas.queryByRole("img", {name: "Your photo holding your ID"})).toBeNull()
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await expect(uploads).toHaveLength(0)
        await expect(submissions).toHaveLength(0)
    },
}

/** A failed upload is said plainly, and the voter takes the photo again. */
export const HoldingUploadFails: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(500),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await autoPhoto(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await expect(await heading(canvasElement, "We couldn’t send your photo")).toBeVisible()
        await expect(submissions).toHaveLength(0)
        await userEvent.click(canvas.getByRole("button", {name: "Try again"}))
        await expect(await heading(canvasElement, "Take a photo holding your ID")).toBeVisible()
    },
}

/** An expired page token needs a new page from Keycloak. */
export const HoldingTokenExpired: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(401),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await autoPhoto(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await expect(await heading(canvasElement, "Start the verification again")).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Try again"})).toBeNull()
    },
}

/** Without the camera, the voter gets the camera help. */
export const HoldingWithoutCamera: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(200, false),
    play: async ({canvasElement}) => {
        await press(canvasElement, FAKE_OCR, "Finish")
        await heading(canvasElement, "Check your face")
        await press(canvasElement, FAKE_LIVENESS, "Finish")
        await expect(await heading(canvasElement, "We couldn’t use your camera")).toBeVisible()
        await expect(uploads).toHaveLength(0)
    },
}
