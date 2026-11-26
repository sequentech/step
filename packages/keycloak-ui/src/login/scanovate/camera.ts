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

/**
 * The steps whose camera is remembered on the device, each by itself: a
 * tablet on a stand may read documents with one camera and take the photo of
 * the voter holding their ID with the other. The values are the storage keys.
 */
export enum CameraChoice {
    Document = "sequent.scanovate.documentCamera",
    Holding = "sequent.scanovate.holdingCamera",
}

type ChoiceStorage = Pick<Storage, "getItem" | "setItem">

/**
 * The camera a realm starts the document scan with: its authenticator's
 * document-camera setting as the page receives it. The rear one, which
 * Scanovate's page opens by itself, unless the realm names the front one.
 */
export function startingFacing(configured: unknown): CameraFacing {
    return configured === CameraFacing.Front ? CameraFacing.Front : CameraFacing.Rear
}

/**
 * The camera last chosen on this device; until one is, the one the realm
 * starts with. A choice made on the device wins: whoever set the tablet up
 * saw which camera reads documents there.
 */
export function storedFacing(
    storage: Pick<Storage, "getItem"> | undefined,
    starting: CameraFacing = CameraFacing.Rear,
    choice: CameraChoice = CameraChoice.Document
): CameraFacing {
    try {
        const chosen = storage?.getItem(choice)
        return chosen === CameraFacing.Front || chosen === CameraFacing.Rear ? chosen : starting
    } catch {
        return starting
    }
}

/** Keeps the choice on the device: a tablet set up once stays as it was set. */
export function storeFacing(
    storage: ChoiceStorage | undefined,
    facing: CameraFacing,
    choice: CameraChoice = CameraChoice.Document
): void {
    try {
        storage?.setItem(choice, facing)
    } catch {
        // A browser that refuses storage keeps the choice for this page only.
    }
}

export function otherFacing(facing: CameraFacing): CameraFacing {
    return facing === CameraFacing.Rear ? CameraFacing.Front : CameraFacing.Rear
}

/** The browser's storage for this site, where there is one. */
export function deviceStorage(): Storage | undefined {
    try {
        return window.localStorage
    } catch {
        return undefined
    }
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

/**
 * The camera access of the page a frame shows; nothing for a page of another
 * origin, which the browser keeps out of reach.
 */
export function frameDevices(
    frame: Pick<HTMLIFrameElement, "contentWindow"> | null | undefined
): MediaDevices | undefined {
    try {
        return frame?.contentWindow?.navigator.mediaDevices
    } catch {
        return undefined
    }
}

/**
 * The most cameras any of the windows is shown. Safari lists them all only
 * to the frame that was allowed the camera, which is Scanovate's page: this
 * page is shown one, whatever the device has.
 */
export async function mostCameras(
    sources: (Pick<MediaDevices, "enumerateDevices"> | undefined)[]
): Promise<number> {
    return Math.max(0, ...(await Promise.all(sources.map(cameraCount))))
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
