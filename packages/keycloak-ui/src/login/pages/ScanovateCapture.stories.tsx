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
 * Cameras that film a test card, still unless `moving`, and the uploads the
 * page makes. Each camera paints its own background, so a photo tells which
 * camera took it; like a phone, a second camera won't open while one is live.
 * `capabilities` lists each camera's side, as Chrome does; without it only
 * the open camera tells its side, as iOS Safari does.
 */
type FakeCamera = {
    deviceId: string
    facing?: "user" | "environment"
    color: string
    fails?: boolean
    // How long it takes to open, as a phone's camera can.
    delayMs?: number
}
const FRONT_CAMERA: FakeCamera = {deviceId: "front", facing: "user", color: "#0d796a"}
const BACK_CAMERA: FakeCamera = {deviceId: "back", facing: "environment", color: "#1f3a93"}
type Uploads = {url: string; init: RequestInit}[]
const uploads: Uploads = []
// Every camera stream the page opened, to check none is left on.
const openedStreams: MediaStream[] = []
// Whether the voter moves: a test that switches cameras keeps moving, so that
// no photo is taken before it holds still.
const motion = {moving: false}
// The cameras plugged in, which a test can change.
const rig: {cameras: FakeCamera[]} = {cameras: []}
function fakeCameraAndUploads(
    status = 200,
    camera = true,
    moving = false,
    cameras: FakeCamera[] = [FRONT_CAMERA],
    capabilities = true
) {
    uploads.length = 0
    openedStreams.length = 0
    motion.moving = moving
    rig.cameras = [...cameras]
    const devices = navigator.mediaDevices
    const realGetUserMedia = devices.getUserMedia
    const realEnumerateDevices = devices.enumerateDevices
    const realFetch = window.fetch
    const timers: ReturnType<typeof setInterval>[] = []
    const live = () =>
        openedStreams.filter((stream) =>
            stream.getTracks().some((track) => track.readyState === "live")
        )
    const film = (filmed: FakeCamera) => {
        const canvas = document.createElement("canvas")
        canvas.width = 640
        canvas.height = 480
        const paint = canvas.getContext("2d")!
        let tick = 0
        // Repainted all the time, as a camera delivers frames; only `moving` changes them.
        timers.push(
            setInterval(() => {
                tick++
                const x = motion.moving ? (tick * 97) % 460 : 380
                paint.fillStyle = filmed.color
                paint.fillRect(0, 0, canvas.width, canvas.height)
                paint.fillStyle = "#ffffff"
                paint.fillRect(x, 160, 180, 240)
            }, 50)
        )
        const stream = canvas.captureStream(20)
        const [track] = stream.getVideoTracks()
        const settings = track.getSettings()
        track.getSettings = () => ({
            ...settings,
            deviceId: filmed.deviceId,
            ...(filmed.facing === undefined ? {} : {facingMode: filmed.facing}),
        })
        return stream
    }
    devices.getUserMedia = async (constraints) => {
        if (!camera) throw new DOMException("Permission denied", "NotAllowedError")
        if (live().length > 0) {
            throw new DOMException("Could not start video source", "NotReadableError")
        }
        const video = (constraints?.video ?? {}) as MediaTrackConstraints
        const id = (video.deviceId as ConstrainDOMStringParameters | undefined)?.exact
        const facing = video.facingMode as ConstrainDOMStringParameters | undefined
        const side = facing?.exact ?? facing?.ideal
        const chosen =
            id !== undefined
                ? rig.cameras.find((c) => c.deviceId === id)
                : (rig.cameras.find((c) => c.facing === side) ??
                  (facing?.exact === undefined ? rig.cameras[0] : undefined))
        if (chosen === undefined) throw new DOMException("No camera", "OverconstrainedError")
        if (chosen.fails) throw new DOMException("Could not start", "NotReadableError")
        if (chosen.delayMs !== undefined) {
            await new Promise((resolve) => setTimeout(resolve, chosen.delayMs))
        }
        const stream = film(chosen)
        openedStreams.push(stream)
        return stream
    }
    devices.enumerateDevices = async () =>
        rig.cameras.map(
            (c) =>
                ({
                    kind: "videoinput",
                    deviceId: c.deviceId,
                    groupId: "",
                    label: "",
                    ...(capabilities && c.facing !== undefined
                        ? {getCapabilities: () => ({facingMode: [c.facing]})}
                        : {}),
                }) as unknown as MediaDeviceInfo
        )
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
        for (const timer of timers) clearInterval(timer)
        for (const stream of openedStreams) stream.getTracks().forEach((track) => track.stop())
        devices.getUserMedia = realGetUserMedia
        devices.enumerateDevices = realEnumerateDevices
        window.fetch = realFetch
        cleanSubmissions()
    }
}

/** How many camera streams are still on: never more than the one shown. */
function liveCameras() {
    return openedStreams.filter((stream) =>
        stream.getTracks().some((track) => track.readyState === "live")
    ).length
}

/** The background colour of a photo, which tells the camera that took it. */
async function photoColor(blob: Blob): Promise<string> {
    const bitmap = await createImageBitmap(blob)
    const canvas = document.createElement("canvas")
    canvas.width = bitmap.width
    canvas.height = bitmap.height
    const context = canvas.getContext("2d")!
    context.drawImage(bitmap, 0, 0)
    const [r, g, b] = context.getImageData(10, 10, 1, 1).data
    return [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")
}

/** Whether a colour read back from a JPEG is close to the one painted. */
function near(actual: string, expected: string) {
    const channels = (hex: string) =>
        [0, 2, 4].map((i) => parseInt(hex.replace("#", "").slice(i, i + 2), 16))
    const a = channels(actual)
    return channels(expected).every((v, i) => Math.abs(v - a[i]) <= 12)
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

/** One camera: no switch that leads nowhere. */
export const HoldingSingleCamera: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await expect(await canvas.findByText(/^Hold still: \d$/, {}, {timeout: 5000})).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Switch camera"})).toBeNull()
        await expect(canvasElement.querySelector(".scanovate-window video")).toHaveClass("mirrored")
    },
}

/**
 * A phone or tablet: the front camera first, as a mirror; the switch turns to
 * the rear camera, shown as it sees, and back, with one camera on at a time,
 * and the photo comes from the camera chosen. Done twice: with the cameras'
 * sides listed (Chrome) and only the open camera's (iOS Safari).
 */
function holdingSwitchesBothWays(capabilities: boolean): Story {
    return {
        args: {kcContext: withHolding},
        beforeEach: () =>
            fakeCameraAndUploads(200, true, true, [FRONT_CAMERA, BACK_CAMERA], capabilities),
        play: async ({canvasElement}) => {
            const canvas = within(canvasElement)
            await throughScanovate(canvasElement)
            const video = () => canvasElement.querySelector(".scanovate-window video")!
            const switchButton = await canvas.findByRole("button", {name: "Switch camera"})
            await expect(video()).toHaveClass("mirrored")

            await userEvent.click(switchButton)
            await waitFor(() => expect(video()).not.toHaveClass("mirrored"))
            await expect(liveCameras()).toBe(1)
            // The step is the same, with nothing lost: still the holding photo.
            await expect(await heading(canvasElement, "Take a photo holding your ID")).toBeVisible()

            await userEvent.click(canvas.getByRole("button", {name: "Switch camera"}))
            await waitFor(() => expect(video()).toHaveClass("mirrored"))
            await expect(liveCameras()).toBe(1)

            await userEvent.click(canvas.getByRole("button", {name: "Switch camera"}))
            await waitFor(() => expect(video()).not.toHaveClass("mirrored"))
            motion.moving = false
            await expect(await autoPhoto(canvasElement)).toBeVisible()
            // No switch while the voter checks the photo.
            await expect(canvas.queryByRole("button", {name: "Switch camera"})).toBeNull()
            await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
            await waitFor(() => expect(submissions).toHaveLength(1))
            const color = await photoColor(uploads[0].init.body as Blob)
            await expect(near(color, BACK_CAMERA.color)).toBe(true)
        },
    }
}
export const HoldingSwitchesCamera: Story = holdingSwitchesBothWays(true)
export const HoldingSwitchesCameraWithoutListedSides: Story = holdingSwitchesBothWays(false)

/** A computer with an external webcam: the switch goes round its cameras. */
export const HoldingSwitchesWebcam: Story = {
    args: {kcContext: withHolding},
    beforeEach: () =>
        fakeCameraAndUploads(200, true, true, [
            {deviceId: "built-in", color: "#0d796a"},
            {deviceId: "usb", color: "#8a2be2"},
        ]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Switch camera"}))
        await waitFor(() => expect(openedStreams).toHaveLength(2))
        motion.moving = false
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        // A webcam tells no side: it shows as a mirror, like the built-in one.
        await expect(canvasElement.querySelector(".scanovate-window video")).toHaveClass("mirrored")
        await expect(liveCameras()).toBe(1)
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect(near(await photoColor(uploads[0].init.body as Blob), "#8a2be2")).toBe(true)
    },
}

/** A camera that won't open: the voter is told, and carries on with the one they had. */
export const HoldingSwitchFails: Story = {
    args: {kcContext: withHolding},
    beforeEach: () =>
        fakeCameraAndUploads(200, true, true, [FRONT_CAMERA, {...BACK_CAMERA, fails: true}]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Switch camera"}))
        await expect(await canvas.findByRole("alert", {}, {timeout: 5000})).toHaveTextContent(
            "The other camera couldn’t be opened. You can carry on with this one."
        )
        await expect(canvasElement.querySelector(".scanovate-window video")).toHaveClass("mirrored")
        await expect(liveCameras()).toBe(1)
        motion.moving = false
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect(near(await photoColor(uploads[0].init.body as Blob), FRONT_CAMERA.color)).toBe(
            true
        )
    },
}

/** Leaving the step turns the camera off. */
export const HoldingClosedStopsCamera: Story = {
    args: {kcContext: withHolding},
    beforeEach: () => fakeCameraAndUploads(200, true, true, [FRONT_CAMERA, BACK_CAMERA]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Switch camera"}))
        await waitFor(() => expect(openedStreams).toHaveLength(2))
        await userEvent.click(canvas.getByRole("button", {name: "Close"}))
        await expect(await heading(canvasElement, "The scan was closed")).toBeVisible()
        await waitFor(() => expect(liveCameras()).toBe(0))
    },
}

/**
 * A camera slow to open: the voter is told the camera is switching, the last
 * frame of the one left isn't shown, as if the camera had frozen, and no
 * photo is taken of it.
 */
export const HoldingSlowSwitch: Story = {
    args: {kcContext: withHolding},
    beforeEach: () =>
        fakeCameraAndUploads(200, true, true, [FRONT_CAMERA, {...BACK_CAMERA, delayMs: 2500}]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Switch camera"}))
        motion.moving = false
        await expect(await canvas.findByText("Switching camera…")).toBeVisible()
        await expect(canvasElement.querySelector(".scanovate-window video")).not.toBeVisible()
        await expect(canvas.getByRole("button", {name: "Switch camera"})).toBeDisabled()
        await new Promise((resolve) => setTimeout(resolve, 2000))
        await expect(canvas.queryByRole("img", {name: "Your photo holding your ID"})).toBeNull()
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect(near(await photoColor(uploads[0].init.body as Blob), BACK_CAMERA.color)).toBe(
            true
        )
    },
}

/** The box of an element on the page. */
function box(canvasElement: HTMLElement, selector: string) {
    const element = canvasElement.ownerDocument.querySelector(selector)
    if (element === null) throw new Error(`No ${selector}`)
    return element.getBoundingClientRect()
}

function overlaps(a: DOMRect, b: DOMRect) {
    return a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/** The story in a window of that width, as a tablet or a computer would show it. */
function atWidth(width: number) {
    return {
        globals: {viewport: {value: `width-${width}`, isRotated: false}},
        parameters: {
            viewport: {
                options: {
                    [`width-${width}`]: {
                        name: `${width}px`,
                        styles: {width: `${width}px`, height: "800px"},
                    },
                },
            },
        },
    }
}

/**
 * Windows wider than 720 px (tablets, computers, phones held sideways) show
 * the mark in the top left corner, over the camera: the switch sits under it,
 * where it can be seen. Narrow windows show the mark at the bottom.
 */
function holdingSwitchClearOfMark(width: number): Story {
    return {
        ...atWidth(width),
        args: {kcContext: withHolding},
        beforeEach: () => {
            expect(window.innerWidth, "Actual browser viewport width").toBe(width)
            return fakeCameraAndUploads(200, true, true, [FRONT_CAMERA, BACK_CAMERA])
        },
        play: async ({canvasElement}) => {
            const canvas = within(canvasElement)
            await throughScanovate(canvasElement)
            const switchButton = await canvas.findByRole("button", {name: "Switch camera"})
            const mark = box(canvasElement, ".auth-fullscreen-brand")
            await expect(overlaps(switchButton.getBoundingClientRect(), mark)).toBe(false)
            await expect(
                overlaps(
                    switchButton.getBoundingClientRect(),
                    box(canvasElement, ".scanovate-window-heading")
                )
            ).toBe(false)
            await userEvent.click(switchButton)
            await waitFor(() =>
                expect(canvasElement.querySelector(".scanovate-window video")).not.toHaveClass(
                    "mirrored"
                )
            )
        },
    }
}
export const HoldingSwitchClearOfMarkTablet: Story = holdingSwitchClearOfMark(800)
export const HoldingSwitchClearOfMarkComputer: Story = holdingSwitchClearOfMark(1280)
export const HoldingSwitchClearOfMarkPhone: Story = holdingSwitchClearOfMark(390)

/**
 * The front camera is taken by another app, or doesn't answer: the page opens
 * the rear one instead of leaving the voter without a camera, and the voter
 * can switch back once the front one is free.
 */
export const HoldingFrontCameraBusy: Story = {
    args: {kcContext: withHolding},
    beforeEach: () =>
        fakeCameraAndUploads(200, true, true, [{...FRONT_CAMERA, fails: true}, BACK_CAMERA]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await expect(await canvas.findByRole("button", {name: "Switch camera"})).toBeVisible()
        await expect(canvasElement.querySelector(".scanovate-window video")).not.toHaveClass(
            "mirrored"
        )
        await expect(liveCameras()).toBe(1)
        motion.moving = false
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect(near(await photoColor(uploads[0].init.body as Blob), BACK_CAMERA.color)).toBe(
            true
        )
    },
}

/** Plugs a camera in or out, as the browser tells the page. */
function plug(cameras: FakeCamera[]) {
    rig.cameras = cameras
    navigator.mediaDevices.dispatchEvent(new Event("devicechange"))
}

/**
 * The webcam in use is unplugged: the page goes back to the camera that is
 * left, without leaving the step, and there is nothing to switch to any more.
 */
export const HoldingWebcamUnplugged: Story = {
    args: {kcContext: withHolding},
    beforeEach: () =>
        fakeCameraAndUploads(200, true, true, [
            {deviceId: "built-in", color: "#0d796a"},
            {deviceId: "usb", color: "#8a2be2"},
        ]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Switch camera"}))
        await waitFor(() => expect(openedStreams).toHaveLength(2))
        const [usbTrack] = openedStreams[1].getVideoTracks()
        plug([{deviceId: "built-in", color: "#0d796a"}])
        usbTrack.stop()
        usbTrack.dispatchEvent(new Event("ended"))
        await waitFor(() => expect(openedStreams).toHaveLength(3))
        await waitFor(() =>
            expect(canvas.queryByRole("button", {name: "Switch camera"})).toBeNull()
        )
        await expect(await heading(canvasElement, "Take a photo holding your ID")).toBeVisible()
        await expect(liveCameras()).toBe(1)
        motion.moving = false
        await expect(await autoPhoto(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Use this photo"}))
        await waitFor(() => expect(submissions).toHaveLength(1))
        await expect(near(await photoColor(uploads[0].init.body as Blob), "#0d796a")).toBe(true)
    },
}

/** A webcam plugged in during the step brings the switch. */
export const HoldingWebcamPluggedIn: Story = {
    args: {kcContext: withHolding},
    beforeEach: () =>
        fakeCameraAndUploads(200, true, true, [{deviceId: "built-in", color: "#0d796a"}]),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await throughScanovate(canvasElement)
        await expect(await canvas.findByRole("progressbar", {name: "Holding still"})).toBeVisible()
        await expect(canvas.queryByRole("button", {name: "Switch camera"})).toBeNull()
        plug([
            {deviceId: "built-in", color: "#0d796a"},
            {deviceId: "usb", color: "#8a2be2"},
        ])
        await userEvent.click(await canvas.findByRole("button", {name: "Switch camera"}))
        await waitFor(() => expect(openedStreams).toHaveLength(2))
        await expect(liveCameras()).toBe(1)
    },
}
