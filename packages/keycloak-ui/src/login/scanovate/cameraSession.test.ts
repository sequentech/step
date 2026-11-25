// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, describe, expect, it} from "vitest"
import {
    CameraFacing,
    CameraRequestKind,
    CameraSession,
    CameraSwitching,
    HOLDING_CAMERA,
    SwitchOutcome,
    canSwitch,
    facingOf,
    fallbackRequests,
    holdingCamera,
    mirrorsPreview,
    switchRequests,
    usableCameras,
    videoConstraints,
    type Camera,
    type CameraPolicy,
    type MediaDevicesLike,
    type PermissionsLike,
} from "./cameraSession"

const FRONT: Camera = {deviceId: "front", facing: CameraFacing.Front}
const BACK: Camera = {deviceId: "back", facing: CameraFacing.Rear}
const BACK_WIDE: Camera = {deviceId: "back-wide", facing: CameraFacing.Rear}
const BUILT_IN: Camera = {deviceId: "built-in", facing: null}
const EXTERNAL: Camera = {deviceId: "external", facing: null}

const SWITCHABLE: CameraPolicy = {preferred: CameraFacing.Front, switching: CameraSwitching.Allowed}
const FIXED: CameraPolicy = {preferred: CameraFacing.Front, switching: CameraSwitching.Fixed}

describe("the camera policy of the holding photo", () => {
    it("prefers the front camera and lets the voter switch", () => {
        expect(HOLDING_CAMERA).toEqual({
            preferred: CameraFacing.Front,
            switching: CameraSwitching.Allowed,
        })
    })

    it("starts with the camera last chosen on the device, and still lets the voter switch", () => {
        expect(holdingCamera(CameraFacing.Rear)).toEqual({
            preferred: CameraFacing.Rear,
            switching: CameraSwitching.Allowed,
        })
        expect(holdingCamera(CameraFacing.Front)).toEqual(HOLDING_CAMERA)
    })
})

describe("facingOf", () => {
    it("reads the two facings a camera can report", () => {
        expect(facingOf("user")).toBe(CameraFacing.Front)
        expect(facingOf("environment")).toBe(CameraFacing.Rear)
    })

    it("knows nothing of side cameras, missing or other values", () => {
        for (const value of ["left", "right", "", undefined, null, 3, ["user"]]) {
            expect(facingOf(value)).toBeNull()
        }
    })
})

describe("usableCameras", () => {
    const device = (
        kind: MediaDeviceKind,
        deviceId: string,
        facingMode?: string[]
    ): MediaDeviceInfo =>
        ({
            kind,
            deviceId,
            groupId: "",
            label: "",
            toJSON: () => ({}),
            ...(facingMode === undefined ? {} : {getCapabilities: () => ({facingMode})}),
        }) as unknown as MediaDeviceInfo

    it("keeps the video inputs, with the facing the browser tells", () => {
        expect(
            usableCameras([
                device("audioinput", "mic"),
                device("videoinput", "front", ["user"]),
                device("videoinput", "back", ["environment"]),
                device("videoinput", "usb"),
                device("audiooutput", "speaker"),
            ])
        ).toEqual([FRONT, BACK, {deviceId: "usb", facing: null}])
    })

    it("leaves out cameras hidden before permission and repeated ones", () => {
        // Safari and Firefox list cameras without ids until the page may use one.
        expect(
            usableCameras([
                device("videoinput", ""),
                device("videoinput", ""),
                device("videoinput", "front", ["user"]),
                device("videoinput", "front", ["user"]),
            ])
        ).toEqual([FRONT])
    })

    it("takes the facing of the camera that is open when the list doesn't tell it", () => {
        expect(
            usableCameras([device("videoinput", "front"), device("videoinput", "back")], FRONT)
        ).toEqual([FRONT, {deviceId: "back", facing: null}])
    })
})

describe("canSwitch", () => {
    it("offers the switch wherever the browser gives more than one camera", () => {
        expect(canSwitch(SWITCHABLE, [FRONT, BACK])).toBe(true)
        expect(canSwitch(SWITCHABLE, [BUILT_IN, EXTERNAL])).toBe(true)
    })

    it("never offers a switch that leads nowhere", () => {
        expect(canSwitch(SWITCHABLE, [])).toBe(false)
        expect(canSwitch(SWITCHABLE, [FRONT])).toBe(false)
    })

    it("keeps the camera of a step that allows only one", () => {
        expect(canSwitch(FIXED, [FRONT, BACK])).toBe(false)
    })
})

describe("switchRequests", () => {
    it("turns a phone or tablet to its other side, with its other cameras as fallbacks", () => {
        expect(switchRequests(FRONT, [FRONT, BACK, BACK_WIDE])).toEqual([
            {kind: CameraRequestKind.Facing, facing: CameraFacing.Rear},
            {kind: CameraRequestKind.Device, deviceId: "back", facing: CameraFacing.Rear},
            {kind: CameraRequestKind.Device, deviceId: "back-wide", facing: CameraFacing.Rear},
        ])
        expect(switchRequests(BACK, [FRONT, BACK, BACK_WIDE])).toEqual([
            {kind: CameraRequestKind.Facing, facing: CameraFacing.Front},
            {kind: CameraRequestKind.Device, deviceId: "front", facing: CameraFacing.Front},
            {kind: CameraRequestKind.Device, deviceId: "back-wide", facing: CameraFacing.Rear},
        ])
    })

    it("turns to the other side when only the open camera tells its facing", () => {
        // iOS Safari tells the facing of the open track, not of the list.
        const back = {deviceId: "back", facing: null}
        expect(switchRequests(FRONT, [{deviceId: "front", facing: null}, back])).toEqual([
            {kind: CameraRequestKind.Facing, facing: CameraFacing.Rear},
            {kind: CameraRequestKind.Device, deviceId: "back", facing: null},
        ])
    })

    it("goes round the cameras of a computer, which tell no facing", () => {
        const third = {deviceId: "third", facing: null}
        expect(switchRequests(BUILT_IN, [BUILT_IN, EXTERNAL, third])).toEqual([
            {kind: CameraRequestKind.Device, deviceId: "external", facing: null},
            {kind: CameraRequestKind.Device, deviceId: "third", facing: null},
        ])
        expect(switchRequests(third, [BUILT_IN, EXTERNAL, third])).toEqual([
            {kind: CameraRequestKind.Device, deviceId: "built-in", facing: null},
            {kind: CameraRequestKind.Device, deviceId: "external", facing: null},
        ])
    })

    it("doesn't ask for a side no other camera has", () => {
        const otherFront = {deviceId: "front-2", facing: CameraFacing.Front}
        expect(switchRequests(FRONT, [FRONT, otherFront])).toEqual([
            {kind: CameraRequestKind.Device, deviceId: "front-2", facing: CameraFacing.Front},
        ])
    })

    it("tries every listed camera when the open one has no id", () => {
        expect(switchRequests({deviceId: "", facing: null}, [BUILT_IN, EXTERNAL])).toEqual([
            {kind: CameraRequestKind.Device, deviceId: "built-in", facing: null},
            {kind: CameraRequestKind.Device, deviceId: "external", facing: null},
        ])
    })
})

describe("fallbackRequests", () => {
    it("tries the other side first when the preferred camera didn't open", () => {
        expect(fallbackRequests(CameraFacing.Front, [FRONT, BACK, BUILT_IN])).toEqual([
            {kind: CameraRequestKind.Device, deviceId: "back", facing: CameraFacing.Rear},
            {kind: CameraRequestKind.Device, deviceId: "built-in", facing: null},
            {kind: CameraRequestKind.Device, deviceId: "front", facing: CameraFacing.Front},
        ])
    })

    it("has nothing to try before the camera is allowed", () => {
        expect(fallbackRequests(CameraFacing.Front, [])).toEqual([])
    })
})

describe("videoConstraints", () => {
    it("asks for the preferred side without requiring it, at the photo's resolution", () => {
        expect(
            videoConstraints({kind: CameraRequestKind.Preferred, facing: CameraFacing.Front})
        ).toEqual({facingMode: {ideal: "user"}, width: {ideal: 1280}, height: {ideal: 720}})
    })

    it("requires the side or the camera the voter switched to", () => {
        expect(
            videoConstraints({kind: CameraRequestKind.Facing, facing: CameraFacing.Rear})
        ).toEqual({facingMode: {exact: "environment"}, width: {ideal: 1280}, height: {ideal: 720}})
        expect(videoConstraints({kind: CameraRequestKind.Device, deviceId: "usb"})).toEqual({
            deviceId: {exact: "usb"},
            width: {ideal: 1280},
            height: {ideal: 720},
        })
    })
})

describe("mirrorsPreview", () => {
    it("mirrors the front camera and cameras of unknown side, as a mirror would", () => {
        expect(mirrorsPreview(FRONT)).toBe(true)
        expect(mirrorsPreview(BUILT_IN)).toBe(true)
        expect(mirrorsPreview(null)).toBe(true)
    })

    it("shows the rear camera as it sees", () => {
        expect(mirrorsPreview(BACK)).toBe(false)
    })
})

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

/**
 * Cameras that deliver real frames (a canvas stream each) and report their
 * settings as a browser would, like a phone that can't open two at once.
 */
type FakeCamera = Camera & {fails?: boolean; reportsFacing?: boolean; delayMs?: number}
class FakeDevices implements MediaDevicesLike {
    readonly opened: {constraints: MediaTrackConstraints; stream: MediaStream}[] = []
    readonly cameras: FakeCamera[]
    // Whether the browser can open a camera while another is open.
    readonly onlyOneAtATime: boolean

    constructor(cameras: FakeCamera[], onlyOneAtATime = true) {
        this.cameras = cameras
        this.onlyOneAtATime = onlyOneAtATime
    }

    live() {
        return this.opened.filter(({stream}) =>
            stream.getTracks().some((track) => track.readyState === "live")
        )
    }

    async enumerateDevices() {
        return this.cameras.map(
            ({deviceId}) =>
                ({kind: "videoinput", deviceId, groupId: "", label: ""}) as MediaDeviceInfo
        )
    }

    async getUserMedia(constraints?: MediaStreamConstraints) {
        const video = constraints?.video as MediaTrackConstraints
        if (this.onlyOneAtATime && this.live().length > 0) {
            throw new DOMException("Could not start video source", "NotReadableError")
        }
        const camera = this.pick(video)
        if (camera === undefined) {
            throw new DOMException("No such camera", "OverconstrainedError")
        }
        if (camera.fails) throw new DOMException("Could not start", "NotReadableError")
        if (camera.delayMs !== undefined) await wait(camera.delayMs)
        const canvas = document.createElement("canvas")
        canvas.width = 64
        canvas.height = 48
        canvas.getContext("2d")!.fillRect(0, 0, 64, 48)
        const stream = canvas.captureStream(5)
        const [track] = stream.getVideoTracks()
        const settings = track.getSettings()
        track.getSettings = () => ({
            ...settings,
            deviceId: camera.deviceId,
            ...(camera.reportsFacing === false || camera.facing === null
                ? {}
                : {facingMode: camera.facing === CameraFacing.Front ? "user" : "environment"}),
        })
        this.opened.push({constraints: video, stream})
        return stream
    }

    private pick(video: MediaTrackConstraints): FakeCamera | undefined {
        const exactId = (video.deviceId as ConstrainDOMStringParameters | undefined)?.exact
        if (exactId !== undefined) return this.cameras.find((c) => c.deviceId === exactId)
        const facing = video.facingMode as ConstrainDOMStringParameters | undefined
        const want = facingOf(facing?.exact ?? facing?.ideal)
        const match = this.cameras.find((c) => c.facing === want)
        if (facing?.exact !== undefined) return match
        return match ?? this.cameras[0]
    }
}

/**
 * The browser's answer about the camera, which the test changes as a voter
 * answering the prompt would.
 */
class FakePermission extends EventTarget {
    constructor(public state: PermissionState) {
        super()
    }

    set(state: PermissionState) {
        this.state = state
        this.dispatchEvent(new Event("change"))
    }

    asPermissions(): PermissionsLike {
        return {query: async () => this}
    }
}

describe("CameraSession", () => {
    let session: CameraSession | null = null
    afterEach(() => session?.close())

    it("opens the preferred camera first", async () => {
        const devices = new FakeDevices([BACK, FRONT])
        session = new CameraSession(devices, SWITCHABLE)
        const opened = await session.start()
        expect(opened?.camera).toEqual(FRONT)
        expect(devices.opened[0].constraints.facingMode).toEqual({ideal: "user"})
    })

    it("opens whichever camera there is when none faces the preferred side", async () => {
        const devices = new FakeDevices([BUILT_IN])
        session = new CameraSession(devices, SWITCHABLE)
        expect((await session.start())?.camera).toEqual(BUILT_IN)
    })

    it("lists the cameras with the facing of the open one", async () => {
        const devices = new FakeDevices([
            {...FRONT, reportsFacing: true},
            {...BACK, reportsFacing: true},
        ])
        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        // The fake list tells no facing, as iOS Safari's.
        expect(await session.cameras()).toEqual([FRONT, {deviceId: "back", facing: null}])
    })

    it("switches both ways, stopping the camera it leaves", async () => {
        const devices = new FakeDevices([FRONT, BACK])
        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        const cameras = await session.cameras()

        const toBack = await session.switch(cameras)
        expect(toBack.outcome).toBe(SwitchOutcome.Switched)
        expect(toBack.opened?.camera).toEqual(BACK)
        expect(devices.live()).toHaveLength(1)
        expect(devices.live()[0].stream).toBe(toBack.opened?.stream)

        const toFront = await session.switch(await session.cameras())
        expect(toFront.outcome).toBe(SwitchOutcome.Switched)
        expect(toFront.opened?.camera).toEqual(FRONT)
        expect(devices.live()).toHaveLength(1)
        expect(devices.live()[0].stream).toBe(toFront.opened?.stream)
    })

    it("goes round a computer's cameras by id", async () => {
        const devices = new FakeDevices([BUILT_IN, EXTERNAL], false)
        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        const switched = await session.switch(await session.cameras())
        expect(switched.opened?.camera).toEqual(EXTERNAL)
        expect(devices.opened[1].constraints.deviceId).toEqual({exact: "external"})
        expect(devices.live()).toHaveLength(1)
    })

    it("tries the next camera when the first choice won't open", async () => {
        const devices = new FakeDevices([FRONT, {...BACK, fails: true}, BACK_WIDE])
        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        const switched = await session.switch(await session.cameras())
        expect(switched.outcome).toBe(SwitchOutcome.Switched)
        expect(switched.opened?.camera).toEqual(BACK_WIDE)
    })

    it("goes back to the camera it had when no other opens", async () => {
        const devices = new FakeDevices([FRONT, {...BACK, fails: true}])
        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        const kept = await session.switch(await session.cameras())
        expect(kept.outcome).toBe(SwitchOutcome.Kept)
        expect(kept.opened?.camera).toEqual(FRONT)
        expect(devices.live()).toHaveLength(1)
    })

    it("reports the camera lost when not even the old one opens again", async () => {
        const cameras: FakeCamera[] = [{...FRONT}, {...BACK, fails: true}]
        const devices = new FakeDevices(cameras)
        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        const listed = await session.cameras()
        cameras[0].fails = true
        const lost = await session.switch(listed)
        expect(lost.outcome).toBe(SwitchOutcome.Lost)
        expect(lost.opened).toBeNull()
        expect(devices.live()).toHaveLength(0)
    })

    it("doesn't touch the camera of a step that allows only one, or of a single camera", async () => {
        const devices = new FakeDevices([FRONT, BACK])
        session = new CameraSession(devices, FIXED)
        const first = await session.start()
        const fixed = await session.switch(await session.cameras())
        expect(fixed.outcome).toBe(SwitchOutcome.Kept)
        expect(fixed.opened).toBeNull()
        expect(devices.live()[0].stream).toBe(first?.stream)
        session.close()

        const single = new FakeDevices([FRONT])
        session = new CameraSession(single, SWITCHABLE)
        await session.start()
        expect((await session.switch(await session.cameras())).outcome).toBe(SwitchOutcome.Kept)
        expect(single.opened).toHaveLength(1)
    })

    it("reports a camera the voter won't allow", async () => {
        const devices = new FakeDevices([FRONT])
        devices.getUserMedia = async () => {
            throw new DOMException("Permission denied", "NotAllowedError")
        }
        session = new CameraSession(devices, SWITCHABLE)
        expect(await session.start()).toBeNull()
    })

    it("stops the camera when the step closes, even one still opening", async () => {
        const devices = new FakeDevices([FRONT])
        session = new CameraSession(devices, SWITCHABLE)
        const starting = session.start()
        session.close()
        expect(await starting).toBeNull()
        expect(devices.live()).toHaveLength(0)

        session = new CameraSession(devices, SWITCHABLE)
        await session.start()
        session.close()
        expect(devices.live()).toHaveLength(0)
    })

    it("gives up on a camera that doesn't answer, and stops it when it answers late", async () => {
        const devices = new FakeDevices([FRONT, {...BACK, delayMs: 150}], false)
        session = new CameraSession(devices, SWITCHABLE, 40)
        await session.start()
        const kept = await session.switch(await session.cameras())
        expect(kept.outcome).toBe(SwitchOutcome.Kept)
        expect(kept.opened?.camera).toEqual(FRONT)
        await wait(300)
        // The rear camera answered after all: it is off, and only the front one is on.
        expect(devices.opened.map(({stream}) => stream)).toContain(kept.opened?.stream)
        expect(devices.opened.length).toBeGreaterThan(2)
        expect(devices.live()).toHaveLength(1)
        expect(devices.live()[0].stream).toBe(kept.opened?.stream)
    })

    it("waits as long as the voter takes to allow the camera", async () => {
        const devices = new FakeDevices([{...FRONT, delayMs: 150}])
        const permission = new FakePermission("prompt")
        session = new CameraSession(devices, SWITCHABLE, 40, permission.asPermissions())
        expect((await session.start())?.camera).toEqual(FRONT)

        // A browser that doesn't tell whether the camera is allowed: no limit either.
        session.close()
        session = new CameraSession(
            new FakeDevices([{...FRONT, delayMs: 150}]),
            SWITCHABLE,
            40,
            null
        )
        expect((await session.start())?.camera).toEqual(FRONT)
    })

    it("opens another camera when the step's camera doesn't answer although allowed", async () => {
        const devices = new FakeDevices([{...FRONT, delayMs: 300}, BACK], false)
        const permission = new FakePermission("granted")
        session = new CameraSession(devices, SWITCHABLE, 40, permission.asPermissions())
        const opened = await session.start()
        expect(opened?.camera).toEqual(BACK)
        await wait(400)
        // The front camera answered after all: it is off.
        expect(devices.live()).toHaveLength(1)
        expect(devices.live()[0].stream).toBe(opened?.stream)
    })

    it("starts the time once the voter allows the camera", async () => {
        const devices = new FakeDevices([{...FRONT, delayMs: 400}, BACK], false)
        const permission = new FakePermission("prompt")
        session = new CameraSession(devices, SWITCHABLE, 40, permission.asPermissions())
        const starting = session.start()
        await wait(100)
        // Still the prompt: nothing given up yet.
        expect(devices.opened).toHaveLength(0)
        permission.set("granted")
        expect((await starting)?.camera).toEqual(BACK)
    })

    it("asks a side that didn't answer only once", async () => {
        const devices = new FakeDevices([FRONT, {...BACK, delayMs: 150}, BACK_WIDE], false)
        session = new CameraSession(devices, SWITCHABLE, 40, null)
        await session.start()
        const asked: MediaTrackConstraints[] = []
        const getUserMedia = devices.getUserMedia.bind(devices)
        devices.getUserMedia = (constraints) => {
            asked.push(constraints?.video as MediaTrackConstraints)
            return getUserMedia(constraints)
        }
        // Chrome's list, which tells each camera's side.
        const switched = await session.switch([FRONT, BACK, BACK_WIDE])
        // The rear side by facing hung; the main rear camera isn't asked again by id.
        expect(switched.opened?.camera).toEqual(BACK_WIDE)
        expect(asked.map((video) => video.facingMode ?? video.deviceId)).toEqual([
            {exact: "environment"},
            {exact: "back-wide"},
        ])
        await wait(300)
        expect(devices.live()).toHaveLength(1)
    })

    it("opens another camera when the one in use goes away", async () => {
        const cameras: FakeCamera[] = [{...FRONT}, {...BACK}]
        const devices = new FakeDevices(cameras)
        session = new CameraSession(devices, SWITCHABLE)
        const first = await session.start()
        // Unplugged: gone from the list, and its track has ended.
        cameras.splice(0, 1)
        first?.stream.getTracks().forEach((track) => track.stop())
        const again = await session.reopen()
        expect(again?.camera).toEqual(BACK)
        expect(devices.live()).toHaveLength(1)
        expect(await session.cameras()).toEqual([BACK])
    })

    it("reports the camera lost when none answers after the one in use went away", async () => {
        const cameras: FakeCamera[] = [{...FRONT}]
        const devices = new FakeDevices(cameras)
        session = new CameraSession(devices, SWITCHABLE, 40)
        const first = await session.start()
        cameras[0].delayMs = 150
        first?.stream.getTracks().forEach((track) => track.stop())
        expect(await session.reopen()).toBeNull()
        await wait(300)
        expect(devices.live()).toHaveLength(0)
    })
})
