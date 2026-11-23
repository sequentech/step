// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Which camera Scanovate's document page opens.
//
// On a phone or tablet that page opens the rear camera and offers no other:
// right for a voter holding their own phone over a passport, wrong for a
// tablet on a stand facing the voter. Scanovate's configuration has no setting
// for it. The page is embedded from this page's own origin, so this page can
// reach its window, and it changes what that page asks the browser for when
// it opens the camera. Nothing of Scanovate's page is copied or replaced; on
// another origin the browser allows none of this, and the choice is not
// offered.

/** The side of the device the document is scanned with. */
export enum CameraFacing {
    /** Away from the voter: what Scanovate's page opens by itself. */
    Rear = "rear",
    /** Towards the voter. */
    Front = "front",
}

const STORAGE_KEY = "sequent.scanovate.documentCamera"

type ChoiceStorage = Pick<Storage, "getItem" | "setItem">

/** The camera last chosen on this device; the rear one until one is. */
export function storedFacing(storage: Pick<Storage, "getItem"> | undefined): CameraFacing {
    try {
        return storage?.getItem(STORAGE_KEY) === CameraFacing.Front
            ? CameraFacing.Front
            : CameraFacing.Rear
    } catch {
        return CameraFacing.Rear
    }
}

/** Keeps the choice on the device: a tablet set up once stays as it was set. */
export function storeFacing(storage: ChoiceStorage | undefined, facing: CameraFacing): void {
    try {
        storage?.setItem(STORAGE_KEY, facing)
    } catch {
        // A browser that refuses storage keeps the choice for this page only.
    }
}

export function otherFacing(facing: CameraFacing): CameraFacing {
    return facing === CameraFacing.Rear ? CameraFacing.Front : CameraFacing.Rear
}

/** How many cameras the browser lists: all of them once one has been allowed. */
export async function cameraCount(
    devices: Pick<MediaDevices, "enumerateDevices"> | undefined
): Promise<number> {
    try {
        const all = (await devices?.enumerateDevices()) ?? []
        return all.filter((device) => device.kind === "videoinput").length
    } catch {
        return 0
    }
}

/** Whether the embedded page is of this page's origin, and so within reach. */
export function canChooseCamera(frameUrl: string, pageUrl: string): boolean {
    try {
        return new URL(frameUrl, pageUrl).origin === new URL(pageUrl).origin
    } catch {
        return false
    }
}

/**
 * What the embedded page asks the browser for, with the chosen camera.
 *
 * Only a request that names a side changes, and only for the front camera:
 * the side becomes the front, as a preference, and the one camera the page
 * picked (a rear one) is no longer required. The size asked for stays. The
 * page's earlier looks at each camera, which name a device and no side, pass
 * as they are.
 */
export function withFacing(
    constraints: MediaStreamConstraints | undefined,
    facing: CameraFacing
): MediaStreamConstraints | undefined {
    const video = constraints?.video
    if (facing !== CameraFacing.Front || typeof video !== "object" || !("facingMode" in video)) {
        return constraints
    }
    const {deviceId: _picked, ...asked} = video
    return {...constraints, video: {...asked, facingMode: {ideal: "user"}}}
}

const CHOICE = "__sequentDocumentCamera"
type ChoosingDevices = MediaDevices & {[CHOICE]?: CameraFacing}

/**
 * Has the page in `target` open the chosen camera from now on. False when the
 * window is out of reach (another origin) or has no camera access.
 */
export function chooseCamera(target: Window | null | undefined, facing: CameraFacing): boolean {
    let devices: ChoosingDevices | undefined
    try {
        devices = target?.navigator.mediaDevices
    } catch {
        return false
    }
    if (devices === undefined || typeof devices.getUserMedia !== "function") {
        return false
    }
    if (devices[CHOICE] === undefined) {
        const chosen = devices
        const open = devices.getUserMedia.bind(devices)
        devices.getUserMedia = (constraints?: MediaStreamConstraints) =>
            open(withFacing(constraints, chosen[CHOICE] ?? CameraFacing.Rear))
    }
    devices[CHOICE] = facing
    return true
}

/** Calls back again and again; returns how to stop. */
export type Repeat = (callback: () => void) => () => void

const everyFrame: Repeat = (callback) => {
    const timer = window.setInterval(callback, 16)
    return () => window.clearInterval(timer)
}

/**
 * Keeps the chosen camera in whatever document the frame shows.
 *
 * A frame gets a new window with each page it loads, and the page opens its
 * camera some time after it starts (it first reaches its service and shows
 * its instructions): the choice has to be in the new window before then, and
 * there is no event that comes earlier than the page's own scripts. So the
 * frame is looked at on every tick; applying the choice twice changes nothing.
 * Returns how to stop.
 */
export function watchFrame(
    frame: Pick<HTMLIFrameElement, "contentWindow">,
    facing: CameraFacing,
    repeat: Repeat = everyFrame
): () => void {
    const apply = () => void chooseCamera(frame.contentWindow, facing)
    apply()
    return repeat(apply)
}
