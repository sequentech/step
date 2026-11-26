// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it, vi} from "vitest"
import {
    CameraChoice,
    CameraFacing,
    cameraCount,
    canChooseCamera,
    chooseCamera,
    frameDevices,
    mostCameras,
    otherFacing,
    startingFacing,
    storeFacing,
    storedFacing,
    watchFrame,
    withFacing,
} from "./camera"

// What Scanovate's document page asks the browser for on a phone or tablet:
// the rear camera it found, and nothing but a rear camera.
const scanovateOpen = {
    video: {
        width: 1280,
        height: 720,
        facingMode: {exact: "environment"},
        deviceId: {exact: "rear-autofocus"},
    },
}
// How it looks at each camera first, to find one with autofocus.
const scanovateProbe = {video: {deviceId: "rear-autofocus"}}

function memory(initial: Record<string, string> = {}) {
    const values = new Map(Object.entries(initial))
    return {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => void values.set(key, value),
    }
}

describe("withFacing", () => {
    it("leaves everything as Scanovate asks for the rear camera", () => {
        expect(withFacing(scanovateOpen, CameraFacing.Rear)).toBe(scanovateOpen)
        expect(withFacing(scanovateProbe, CameraFacing.Rear)).toBe(scanovateProbe)
    })

    it("asks for the front camera in place of the rear one, at the same size", () => {
        expect(withFacing(scanovateOpen, CameraFacing.Front)).toEqual({
            video: {width: 1280, height: 720, facingMode: {ideal: "user"}},
        })
        // The page's own request is not changed.
        expect(scanovateOpen.video.deviceId).toEqual({exact: "rear-autofocus"})
    })

    it("leaves alone what names no side: the look at each camera, sound, nothing", () => {
        expect(withFacing(scanovateProbe, CameraFacing.Front)).toBe(scanovateProbe)
        const audio = {audio: true}
        expect(withFacing(audio, CameraFacing.Front)).toBe(audio)
        const any = {video: true}
        expect(withFacing(any, CameraFacing.Front)).toBe(any)
        expect(withFacing(undefined, CameraFacing.Front)).toBeUndefined()
    })
})

describe("the remembered choice", () => {
    it("is the rear camera until the voter's device says otherwise", () => {
        expect(storedFacing(memory())).toBe(CameraFacing.Rear)
        expect(storedFacing(memory({"sequent.scanovate.documentCamera": "sideways"}))).toBe(
            CameraFacing.Rear
        )
        expect(storedFacing(undefined)).toBe(CameraFacing.Rear)
    })

    it("starts with the camera the realm names", () => {
        expect(startingFacing("front")).toBe(CameraFacing.Front)
        expect(startingFacing("rear")).toBe(CameraFacing.Rear)
        // An authenticator that predates the setting sends nothing.
        expect(startingFacing(undefined)).toBe(CameraFacing.Rear)
        expect(startingFacing("sideways")).toBe(CameraFacing.Rear)

        expect(storedFacing(memory(), CameraFacing.Front)).toBe(CameraFacing.Front)
        expect(storedFacing(undefined, CameraFacing.Front)).toBe(CameraFacing.Front)
        expect(
            storedFacing(
                memory({"sequent.scanovate.documentCamera": "sideways"}),
                CameraFacing.Front
            )
        ).toBe(CameraFacing.Front)
    })

    it("lets the choice made on the device win over the realm's", () => {
        const storage = memory()
        storeFacing(storage, CameraFacing.Rear)
        expect(storedFacing(storage, CameraFacing.Front)).toBe(CameraFacing.Rear)
        storeFacing(storage, CameraFacing.Front)
        expect(storedFacing(storage, CameraFacing.Rear)).toBe(CameraFacing.Front)
    })

    it("keeps the holding photo's camera apart from the document scan's", () => {
        const storage = memory()
        // A selfie: the front camera until one is chosen on the device.
        expect(storedFacing(storage, CameraFacing.Front, CameraChoice.Holding)).toBe(
            CameraFacing.Front
        )
        storeFacing(storage, CameraFacing.Rear, CameraChoice.Holding)
        expect(storedFacing(storage, CameraFacing.Front, CameraChoice.Holding)).toBe(
            CameraFacing.Rear
        )
        // The document scan's choice is its own.
        expect(storedFacing(storage, CameraFacing.Front)).toBe(CameraFacing.Front)
        storeFacing(storage, CameraFacing.Front)
        expect(storedFacing(storage, CameraFacing.Front, CameraChoice.Holding)).toBe(
            CameraFacing.Rear
        )
    })

    it("is kept on the device", () => {
        const storage = memory()
        storeFacing(storage, CameraFacing.Front)
        expect(storedFacing(storage)).toBe(CameraFacing.Front)
        storeFacing(storage, otherFacing(CameraFacing.Front))
        expect(storedFacing(storage)).toBe(CameraFacing.Rear)
        expect(otherFacing(CameraFacing.Rear)).toBe(CameraFacing.Front)
    })

    it("survives a browser that refuses storage", () => {
        const refusing = {
            getItem: () => {
                throw new Error("denied")
            },
            setItem: () => {
                throw new Error("denied")
            },
        }
        expect(storedFacing(refusing)).toBe(CameraFacing.Rear)
        expect(storedFacing(refusing, CameraFacing.Front)).toBe(CameraFacing.Front)
        expect(() => storeFacing(refusing, CameraFacing.Front)).not.toThrow()
        expect(() => storeFacing(undefined, CameraFacing.Front)).not.toThrow()
    })
})

describe("canChooseCamera", () => {
    const page = "https://portal.example.test/realms/event/login-actions/registration"

    it("is for a Scanovate page of the page's own origin", () => {
        expect(canChooseCamera("https://portal.example.test/biometric/ocr/?token=t", page)).toBe(
            true
        )
        expect(canChooseCamera("/biometric/ocr/?token=t", page)).toBe(true)
    })

    it("is not for another origin, whose page the browser keeps out of reach", () => {
        expect(canChooseCamera("https://ocr.example.test/?token=t", page)).toBe(false)
        expect(canChooseCamera("http://portal.example.test/biometric/ocr/", page)).toBe(false)
        expect(canChooseCamera("https://", page)).toBe(false)
    })
})

describe("cameraCount", () => {
    const device = (kind: MediaDeviceKind) => ({kind}) as MediaDeviceInfo

    it("counts the cameras among the devices the browser lists", async () => {
        const two = {
            enumerateDevices: async () => [
                device("videoinput"),
                device("audioinput"),
                device("videoinput"),
            ],
        }
        expect(await cameraCount(two)).toBe(2)
    })

    it("is none where the browser lists nothing or refuses", async () => {
        expect(await cameraCount(undefined)).toBe(0)
        expect(
            await cameraCount({
                enumerateDevices: async () => {
                    throw new DOMException("denied", "NotAllowedError")
                },
            })
        ).toBe(0)
    })
})

describe("mostCameras", () => {
    const lists = (cameras: number) => ({
        enumerateDevices: async () =>
            Array.from({length: cameras}, () => ({kind: "videoinput"}) as MediaDeviceInfo),
    })

    it("is what the window that was allowed the camera is shown", async () => {
        // Safari lists every camera only to the frame that was allowed one:
        // this page is shown a single camera while Scanovate's page has two.
        expect(await mostCameras([lists(1), lists(2)])).toBe(2)
        expect(await mostCameras([lists(2), undefined])).toBe(2)
    })

    it("is none with nowhere to look", async () => {
        expect(await mostCameras([])).toBe(0)
        expect(await mostCameras([undefined])).toBe(0)
    })
})

describe("frameDevices", () => {
    it("is the camera access of the page the frame shows", () => {
        const {target, mediaDevices} = frameWindow()
        expect(frameDevices({contentWindow: target})).toBe(mediaDevices)
    })

    it("is nothing for no frame, or one out of reach", () => {
        expect(frameDevices(null)).toBeUndefined()
        expect(frameDevices({contentWindow: null})).toBeUndefined()
        const closed = {
            get navigator(): Navigator {
                throw new DOMException("cross-origin", "SecurityError")
            },
        } as unknown as Window
        expect(frameDevices({contentWindow: closed})).toBeUndefined()
    })
})

function frameWindow() {
    const getUserMedia = vi.fn(async (constraints?: MediaStreamConstraints) => constraints)
    const mediaDevices = {getUserMedia}
    return {target: {navigator: {mediaDevices}} as unknown as Window, getUserMedia, mediaDevices}
}

describe("chooseCamera", () => {
    it("has the frame's page open the front camera when it opens the rear one", async () => {
        const {target, getUserMedia, mediaDevices} = frameWindow()
        expect(chooseCamera(target, CameraFacing.Front)).toBe(true)
        await mediaDevices.getUserMedia(scanovateOpen)
        expect(getUserMedia).toHaveBeenLastCalledWith({
            video: {width: 1280, height: 720, facingMode: {ideal: "user"}},
        })
        await mediaDevices.getUserMedia(scanovateProbe)
        expect(getUserMedia).toHaveBeenLastCalledWith(scanovateProbe)
    })

    it("changes the choice without wrapping the page's camera twice", async () => {
        const {target, getUserMedia, mediaDevices} = frameWindow()
        chooseCamera(target, CameraFacing.Front)
        const wrapped = mediaDevices.getUserMedia
        chooseCamera(target, CameraFacing.Rear)
        expect(mediaDevices.getUserMedia).toBe(wrapped)
        await mediaDevices.getUserMedia(scanovateOpen)
        expect(getUserMedia).toHaveBeenCalledTimes(1)
        expect(getUserMedia).toHaveBeenLastCalledWith(scanovateOpen)
    })

    it("says so when the frame cannot be reached or has no camera access", () => {
        const crossOrigin = {
            get navigator(): Navigator {
                throw new DOMException("Blocked a frame", "SecurityError")
            },
        } as unknown as Window
        expect(chooseCamera(crossOrigin, CameraFacing.Front)).toBe(false)
        expect(chooseCamera({navigator: {}} as unknown as Window, CameraFacing.Front)).toBe(false)
        expect(chooseCamera(null, CameraFacing.Front)).toBe(false)
    })
})

describe("watchFrame", () => {
    it("chooses the camera in every document the frame loads, until stopped", async () => {
        const first = frameWindow()
        const second = frameWindow()
        const frame = {contentWindow: first.target as Window | null}
        let tick = () => {}
        const cancel = vi.fn()
        const stop = watchFrame(frame, CameraFacing.Front, (callback) => {
            tick = callback
            return cancel
        })
        // The document that is there now, at once.
        await first.mediaDevices.getUserMedia(scanovateOpen)
        expect(first.getUserMedia).toHaveBeenLastCalledWith({
            video: {width: 1280, height: 720, facingMode: {ideal: "user"}},
        })
        // The page the frame then loads has a window of its own.
        frame.contentWindow = second.target
        tick()
        await second.mediaDevices.getUserMedia(scanovateOpen)
        expect(second.getUserMedia).toHaveBeenLastCalledWith({
            video: {width: 1280, height: 720, facingMode: {ideal: "user"}},
        })
        frame.contentWindow = null
        expect(() => tick()).not.toThrow()
        stop()
        expect(cancel).toHaveBeenCalledOnce()
    })
})
