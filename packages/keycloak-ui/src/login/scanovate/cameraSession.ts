// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The cameras of the page's own capture step. Whether the voter can switch
// depends only on what the browser exposes (more than one camera it can
// open), never on the device or browser name, so the same code serves
// Android, iOS and iPadOS, and computers with external webcams.

import {CameraFacing} from "./camera"

export {CameraFacing}

/** Whether a step lets the voter change camera. */
export enum CameraSwitching {
    Allowed = "ALLOWED",
    Fixed = "FIXED",
}

/** The camera a step opens first, and whether the voter may change it. */
export type CameraPolicy = {preferred: CameraFacing; switching: CameraSwitching}

/**
 * The photo of the voter holding their ID is a selfie: the front camera
 * first, and the rear one for whoever takes it for the voter.
 */
export const HOLDING_CAMERA: CameraPolicy = {
    preferred: CameraFacing.Front,
    switching: CameraSwitching.Allowed,
}

/**
 * The holding photo's policy on a device where a camera was chosen for it:
 * that camera first. A tablet whose operator turned to the rear camera opens
 * it for the next voter too.
 */
export function holdingCamera(preferred: CameraFacing): CameraPolicy {
    return {...HOLDING_CAMERA, preferred}
}

/**
 * How long an allowed camera may take to open. Past it the page tries the
 * next one, and stops this one if it answers later. While the browser asks
 * the voter for the camera there is no limit: the time starts once it is
 * allowed, where the browser tells (the Permissions API); a browser that
 * doesn't tell leaves the step's first camera without a limit.
 */
export const OPEN_TIMEOUT_MS = 10_000

// The holding photo's resolution, as Scanovate's pages ask for theirs.
const WIDTH = 1280
const HEIGHT = 720

/** A camera the page can open; `facing` is null when the browser doesn't tell it. */
export type Camera = {deviceId: string; facing: CameraFacing | null}

export enum CameraRequestKind {
    // The step's camera: that side if there is one, any camera otherwise.
    Preferred = "PREFERRED",
    // Only a camera on that side.
    Facing = "FACING",
    // Only that camera.
    Device = "DEVICE",
}

export type CameraRequest =
    | {kind: CameraRequestKind.Preferred; facing: CameraFacing}
    | {kind: CameraRequestKind.Facing; facing: CameraFacing}
    // `facing`, when known, lets a side that didn't answer be skipped.
    | {kind: CameraRequestKind.Device; deviceId: string; facing?: CameraFacing | null}

export type MediaDevicesLike = Pick<MediaDevices, "getUserMedia" | "enumerateDevices">

/** What the page needs of the Permissions API: whether the camera is allowed, and when it becomes so. */
export type PermissionsLike = {
    query(descriptor: {
        name: "camera"
    }): Promise<
        Pick<PermissionStatus, "state"> &
            Partial<Pick<EventTarget, "addEventListener" | "removeEventListener">>
    >
}

/** The browser's Permissions API, where it has one. */
export function browserPermissions(): PermissionsLike | null {
    const permissions = typeof navigator === "undefined" ? undefined : navigator.permissions
    return permissions === undefined ? null : (permissions as unknown as PermissionsLike)
}

const FACING_MODES: Record<CameraFacing, string> = {
    [CameraFacing.Front]: "user",
    [CameraFacing.Rear]: "environment",
}

/** The facing of a `facingMode` value; null for side cameras and anything else. */
export function facingOf(value: unknown): CameraFacing | null {
    switch (value) {
        case "user":
            return CameraFacing.Front
        case "environment":
            return CameraFacing.Rear
        default:
            return null
    }
}

function opposite(facing: CameraFacing): CameraFacing {
    return facing === CameraFacing.Front ? CameraFacing.Rear : CameraFacing.Front
}

/**
 * The cameras the browser lists, in its order. Chrome tells their facing
 * (`getCapabilities`); where it doesn't, the open camera's own is used.
 * Cameras listed without an id (before permission) can't be chosen.
 */
export function usableCameras(devices: MediaDeviceInfo[], current: Camera | null = null): Camera[] {
    const cameras: Camera[] = []
    for (const device of devices) {
        if (device.kind !== "videoinput" || device.deviceId === "") continue
        if (cameras.some((camera) => camera.deviceId === device.deviceId)) continue
        let facing: CameraFacing | null = null
        const capabilities = (device as Partial<InputDeviceInfo>).getCapabilities?.call(device) as
            | MediaTrackCapabilities
            | undefined
        for (const mode of capabilities?.facingMode ?? []) {
            facing = facing ?? facingOf(mode)
        }
        if (facing === null && current !== null && current.deviceId === device.deviceId) {
            facing = current.facing
        }
        cameras.push({deviceId: device.deviceId, facing})
    }
    return cameras
}

/** Whether to offer the switch: only when it leads to another camera. */
export function canSwitch(policy: CameraPolicy, cameras: Camera[]): boolean {
    return policy.switching === CameraSwitching.Allowed && cameras.length > 1
}

/**
 * What to try, in order, to leave the open camera. A camera that tells its
 * side turns to the other side (the browser picks that side's main camera,
 * not a phone's ultra wide or telephoto), and then each other camera by id;
 * cameras that tell no side, as most computers' do, are taken in turn.
 */
export function switchRequests(current: Camera, cameras: Camera[]): CameraRequest[] {
    const at = cameras.findIndex((camera) => camera.deviceId === current.deviceId)
    const after = at < 0 ? cameras : [...cameras.slice(at + 1), ...cameras.slice(0, at)]
    let others = after.filter((camera) => camera.deviceId !== current.deviceId)
    const requests: CameraRequest[] = []
    if (current.facing !== null) {
        const other = opposite(current.facing)
        if (others.some((camera) => camera.facing !== current.facing)) {
            requests.push({kind: CameraRequestKind.Facing, facing: other})
        }
        // The other side first, then cameras on the same side.
        others = [
            ...others.filter((camera) => camera.facing !== current.facing),
            ...others.filter((camera) => camera.facing === current.facing),
        ]
    }
    for (const camera of others) {
        requests.push({
            kind: CameraRequestKind.Device,
            deviceId: camera.deviceId,
            facing: camera.facing,
        })
    }
    return requests
}

/**
 * What to try when the step's camera didn't open (it hung, or is taken by
 * another app): each camera by id, the other side first, since the side
 * that was asked for is the one that failed.
 */
export function fallbackRequests(preferred: CameraFacing, cameras: Camera[]): CameraRequest[] {
    return [
        ...cameras.filter((camera) => camera.facing !== preferred),
        ...cameras.filter((camera) => camera.facing === preferred),
    ].map((camera) => ({
        kind: CameraRequestKind.Device,
        deviceId: camera.deviceId,
        facing: camera.facing,
    }))
}

function isTimeout(error: unknown): boolean {
    return error instanceof DOMException && error.name === "TimeoutError"
}

export function videoConstraints(request: CameraRequest): MediaTrackConstraints {
    const size = {width: {ideal: WIDTH}, height: {ideal: HEIGHT}}
    switch (request.kind) {
        case CameraRequestKind.Preferred:
            return {facingMode: {ideal: FACING_MODES[request.facing]}, ...size}
        case CameraRequestKind.Facing:
            return {facingMode: {exact: FACING_MODES[request.facing]}, ...size}
        case CameraRequestKind.Device:
            return {deviceId: {exact: request.deviceId}, ...size}
    }
}

/**
 * A front camera shows as a mirror, as voters expect; so does a computer's
 * webcam, which tells no side. A rear camera shows what it sees. Only the
 * preview: the photo is never mirrored.
 */
export function mirrorsPreview(camera: Camera | null): boolean {
    return camera?.facing !== CameraFacing.Rear
}

export function stopStream(stream: MediaStream) {
    for (const track of stream.getTracks()) track.stop()
}

function cameraOf(stream: MediaStream, request: CameraRequest): Camera {
    const settings = stream.getVideoTracks()[0]?.getSettings() ?? {}
    const deviceId =
        settings.deviceId ?? (request.kind === CameraRequestKind.Device ? request.deviceId : "")
    const facing =
        facingOf(settings.facingMode) ??
        (request.kind === CameraRequestKind.Facing ? request.facing : null)
    return {deviceId, facing}
}

export type OpenedCamera = {stream: MediaStream; camera: Camera}

export enum SwitchOutcome {
    // Another camera is open.
    Switched = "SWITCHED",
    // The same camera as before: no other opened, or there was none to switch to.
    Kept = "KEPT",
    // No camera is open any more: not even the old one opened again.
    Lost = "LOST",
}

/** `opened` is the stream to show now, or null when nothing changed or nothing is open. */
export type SwitchResult = {outcome: SwitchOutcome; opened: OpenedCamera | null}

/**
 * The open camera of a step. Only one camera is open at a time: phones
 * can't open a second one, and a camera left open keeps its light on. Each
 * attempt stops the camera before it, and a step that closes stops the
 * camera, even one still opening.
 */
export class CameraSession {
    private stream: MediaStream | null = null
    private current: Camera | null = null
    private generation = 0
    private closed = false

    constructor(
        private readonly devices: MediaDevicesLike,
        private readonly policy: CameraPolicy,
        private readonly openTimeoutMs = OPEN_TIMEOUT_MS,
        private readonly permissions: PermissionsLike | null = browserPermissions()
    ) {}

    get camera(): Camera | null {
        return this.current
    }

    /**
     * Opens the step's camera, or, if it doesn't open, any other camera the
     * voter can switch from; null when none opens (denied, missing or busy).
     */
    async start(): Promise<OpenedCamera | null> {
        const opened = await this.open(
            [{kind: CameraRequestKind.Preferred, facing: this.policy.preferred}],
            false
        )
        if (opened !== null || this.closed) return opened
        // Listed with ids only once the camera is allowed: nothing to try otherwise.
        const others = fallbackRequests(this.policy.preferred, await this.cameras())
        if (others.length === 0 || this.closed) return null
        return this.open(others, true)
    }

    /**
     * Opens the step's camera again after the one in use went away (unplugged,
     * or taken by another app); null when none opens in time.
     */
    reopen(): Promise<OpenedCamera | null> {
        return this.open([{kind: CameraRequestKind.Preferred, facing: this.policy.preferred}], true)
    }

    /** The cameras the voter can switch between, once one is open. */
    async cameras(): Promise<Camera[]> {
        try {
            return usableCameras(await this.devices.enumerateDevices(), this.current)
        } catch {
            return []
        }
    }

    async switch(cameras: Camera[]): Promise<SwitchResult> {
        const previous = this.current
        if (this.closed || previous === null || !canSwitch(this.policy, cameras)) {
            return {outcome: SwitchOutcome.Kept, opened: null}
        }
        const requests = switchRequests(previous, cameras)
        if (requests.length === 0) return {outcome: SwitchOutcome.Kept, opened: null}
        const switched = await this.open(requests, true)
        if (switched !== null) return {outcome: SwitchOutcome.Switched, opened: switched}
        if (this.closed) return {outcome: SwitchOutcome.Kept, opened: null}
        const restored = await this.open(
            [
                previous.deviceId === ""
                    ? {
                          kind: CameraRequestKind.Preferred,
                          facing: previous.facing ?? this.policy.preferred,
                      }
                    : {kind: CameraRequestKind.Device, deviceId: previous.deviceId},
            ],
            true
        )
        return restored === null
            ? {outcome: SwitchOutcome.Lost, opened: null}
            : {outcome: SwitchOutcome.Kept, opened: restored}
    }

    close() {
        this.closed = true
        this.generation++
        this.release()
    }

    private release() {
        if (this.stream !== null) stopStream(this.stream)
        this.stream = null
        this.current = null
    }

    /**
     * Calls `allowed` once the camera is allowed: now, or when the voter
     * answers the browser's prompt. Returns what stops watching. Nothing is
     * called where the browser doesn't tell.
     */
    private whenAllowed(allowed: () => void): () => void {
        let watching = true
        let status: Awaited<ReturnType<PermissionsLike["query"]>> | null = null
        const check = () => {
            if (watching && status?.state === "granted") allowed()
        }
        let query: ReturnType<PermissionsLike["query"]> | undefined
        try {
            query = this.permissions?.query({name: "camera"})
        } catch {
            query = undefined
        }
        void query?.then(
            (answer) => {
                status = answer
                if (!watching) return
                answer.addEventListener?.("change", check)
                check()
            },
            () => undefined
        )
        return () => {
            watching = false
            status?.removeEventListener?.("change", check)
        }
    }

    /**
     * A camera, within the time limit: from the start when `timed`, from when
     * it is allowed otherwise. One that answers late is stopped.
     */
    private async request(constraints: MediaStreamConstraints, timed: boolean) {
        const opening = this.devices.getUserMedia(constraints)
        let timer: ReturnType<typeof setTimeout> | undefined
        let stopWatching = () => {}
        const late = new Promise<never>((_, reject) => {
            const limit = () => {
                if (timer !== undefined) return
                timer = setTimeout(
                    () => reject(new DOMException("The camera took too long", "TimeoutError")),
                    this.openTimeoutMs
                )
            }
            if (timed) {
                limit()
            } else {
                stopWatching = this.whenAllowed(limit)
            }
        })
        try {
            return await Promise.race([opening, late])
        } catch (error) {
            opening.then(stopStream, () => undefined)
            throw error
        } finally {
            clearTimeout(timer)
            stopWatching()
        }
    }

    private async open(requests: CameraRequest[], timed: boolean): Promise<OpenedCamera | null> {
        const generation = ++this.generation
        // A side that didn't answer in time: the browser asked its first camera,
        // so asking that camera again by id would only make the voter wait twice.
        const hung = new Set<CameraFacing>()
        for (const request of requests) {
            if (this.closed || generation !== this.generation) return null
            if (
                request.kind === CameraRequestKind.Device &&
                request.facing != null &&
                hung.delete(request.facing)
            ) {
                continue
            }
            this.release()
            let stream: MediaStream
            try {
                stream = await this.request({video: videoConstraints(request), audio: false}, timed)
            } catch (error) {
                if (isTimeout(error) && request.kind === CameraRequestKind.Facing) {
                    hung.add(request.facing)
                }
                continue
            }
            if (this.closed || generation !== this.generation) {
                stopStream(stream)
                return null
            }
            this.stream = stream
            this.current = cameraOf(stream, request)
            return {stream, camera: this.current}
        }
        return null
    }
}
