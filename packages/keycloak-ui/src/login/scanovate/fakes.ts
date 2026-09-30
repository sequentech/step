// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Stand-ins for the camera, the WebAssembly analyzers and the recorder, for
// stories and tests. The camera paints a synthetic specimen scene.
import {CARD_ASPECT} from "./geometry"
import {
    LivenessApiError,
    LivenessRejection,
    LivenessStatus,
    type LivenessApi,
    type LivenessConnector,
} from "./livenessApi"
import type {CaptureUpload} from "./form"
import {
    UploadError,
    type CaptureUploader,
    type UploadConnector,
    type UploadRejection,
} from "./uploads"
import {
    CameraFacing,
    DocumentStatus,
    FaceStatus,
    type Analyzers,
    type CameraService,
    type CaptureServices,
    type DocumentAnalyzer,
    type DocumentFrame,
    type FaceAnalyzer,
    type FaceFrame,
    type RecorderService,
} from "./types"

// WebM's EBML magic number, so the video part looks like one.
const WEBM_HEADER = new Uint8Array([0x1a, 0x45, 0xdf, 0xa3])

export type Script<Status> = {statuses: Status[]; stability?: number}

// Plays the statuses in order after every reset, repeating the last one.
function player<Status>(script: Script<Status>) {
    let call = 0
    return {
        next(): Status {
            const status = script.statuses[Math.min(call, script.statuses.length - 1)]
            call += 1
            return status
        },
        reset() {
            call = 0
        },
    }
}

export function scriptedDocument(script: Script<DocumentStatus>): DocumentAnalyzer {
    const play = player(script)
    return {
        analyze: (): DocumentFrame => {
            const status = play.next()
            return {
                status,
                corners: null,
                fill: 0.9,
                sharpness: 0.8,
                glare: 0,
                brightness: 0.5,
                stability: status === DocumentStatus.Ready ? 1 : (script.stability ?? 0),
            }
        },
        reset: () => play.reset(),
        free: () => undefined,
    }
}

export function scriptedFace(script: Script<FaceStatus>): FaceAnalyzer {
    const play = player(script)
    return {
        analyze: (): FaceFrame => {
            const status = play.next()
            return {
                status,
                box: null,
                landmarks: [],
                yaw: 0,
                roll: 0,
                brightness: 0.5,
                sharpness: 0.8,
                stability: status === FaceStatus.Ready ? 1 : (script.stability ?? 0),
            }
        },
        reset: () => play.reset(),
        free: () => undefined,
    }
}

export enum SyntheticScene {
    Document = "DOCUMENT",
    Face = "FACE",
    // The voter holding the ID, framed as the video step asks. The front camera
    // preview is mirrored, so the face is painted right of centre.
    FaceWithDocument = "FACE_WITH_DOCUMENT",
}

function paintBackground(context: CanvasRenderingContext2D, width: number, height: number) {
    const gradient = context.createLinearGradient(0, 0, width, height)
    gradient.addColorStop(0, "#3b4c55")
    gradient.addColorStop(1, "#1b2a33")
    context.fillStyle = gradient
    context.fillRect(0, 0, width, height)
}

function paintCard(context: CanvasRenderingContext2D, width: number, height: number) {
    const cardWidth = width * (width < height ? 0.76 : 0.5)
    const cardHeight = cardWidth / CARD_ASPECT
    const x = (width - cardWidth) / 2
    const y = height * (width < height ? 0.43 : 0.45) - cardHeight / 2
    context.fillStyle = "#e4edf2"
    context.beginPath()
    context.roundRect(x, y, cardWidth, cardHeight, cardWidth * 0.04)
    context.fill()
    context.fillStyle = "#9fb0bb"
    context.fillRect(x + cardWidth * 0.06, y + cardHeight * 0.2, cardWidth * 0.24, cardHeight * 0.6)
    context.fillStyle = "#1f3340"
    for (let line = 0; line < 4; line += 1) {
        context.fillRect(
            x + cardWidth * 0.36,
            y + cardHeight * (0.24 + line * 0.16),
            cardWidth * (0.5 - line * 0.06),
            cardHeight * 0.05
        )
    }
    context.fillStyle = "#b8323280"
    context.font = `700 ${Math.round(cardHeight * 0.16)}px sans-serif`
    context.fillText("SPECIMEN", x + cardWidth * 0.3, y + cardHeight * 0.9)
}

function paintFace(context: CanvasRenderingContext2D, width: number, height: number, centre = 0.5) {
    const cx = width * centre
    const cy = height * (width < height ? 0.42 : 0.47)
    const r = Math.min(width, height) * (width < height ? 0.26 : 0.2)
    context.fillStyle = "#1f5a63"
    context.beginPath()
    context.ellipse(cx, cy + r * 2.3, r * 1.9, r * 1.1, 0, Math.PI, 0)
    context.fill()
    context.fillStyle = "#a8765a"
    context.fillRect(cx - r * 0.35, cy + r * 0.9, r * 0.7, r * 0.8)
    context.fillStyle = "#b9856a"
    context.beginPath()
    context.ellipse(cx, cy, r * 0.82, r * 1.05, 0, 0, Math.PI * 2)
    context.fill()
    context.fillStyle = "#1f1a17"
    context.beginPath()
    context.ellipse(cx, cy - r * 0.62, r * 0.86, r * 0.5, 0, Math.PI, 0)
    context.fill()
    context.fillStyle = "#2a1f1a"
    for (const side of [-1, 1]) {
        context.beginPath()
        context.ellipse(cx + side * r * 0.32, cy, r * 0.1, r * 0.07, 0, 0, Math.PI * 2)
        context.fill()
    }
    context.strokeStyle = "#7a3f35"
    context.lineWidth = r * 0.05
    context.beginPath()
    context.arc(cx, cy + r * 0.35, r * 0.25, 0.2 * Math.PI, 0.8 * Math.PI)
    context.stroke()
}

function paintHeldCard(context: CanvasRenderingContext2D, width: number, height: number) {
    const cardWidth = width * (width < height ? 0.34 : 0.2)
    const cardHeight = cardWidth / CARD_ASPECT
    const x = width * (width < height ? 0.1 : 0.22)
    const y = height * (width < height ? 0.45 : 0.55)
    context.save()
    context.translate(x + cardWidth / 2, y + cardHeight / 2)
    context.rotate(0.06)
    context.fillStyle = "#e4edf2"
    context.beginPath()
    context.roundRect(-cardWidth / 2, -cardHeight / 2, cardWidth, cardHeight, cardWidth * 0.05)
    context.fill()
    context.fillStyle = "#9fb0bb"
    context.fillRect(-cardWidth * 0.44, -cardHeight * 0.3, cardWidth * 0.26, cardHeight * 0.6)
    context.fillStyle = "#1f3340"
    for (let line = 0; line < 3; line += 1) {
        context.fillRect(
            -cardWidth * 0.1,
            -cardHeight * (0.25 - line * 0.22),
            cardWidth * 0.5,
            cardHeight * 0.07
        )
    }
    context.restore()
}

// A portrait stream on phone-sized windows, like a phone camera; landscape otherwise.
export function syntheticCamera(sceneFor: (facing: CameraFacing) => SyntheticScene): CameraService {
    return {
        async open(facing) {
            const portrait = window.innerWidth <= 720
            const canvas = document.createElement("canvas")
            canvas.width = portrait ? 720 : 1280
            canvas.height = portrait ? 1280 : 720
            const context = canvas.getContext("2d")
            if (context === null) {
                throw new Error("Canvas 2D is not available")
            }
            const scene = sceneFor(facing)
            const paint = () => {
                paintBackground(context, canvas.width, canvas.height)
                if (scene === SyntheticScene.Document) {
                    paintCard(context, canvas.width, canvas.height)
                } else if (scene === SyntheticScene.Face) {
                    paintFace(context, canvas.width, canvas.height)
                } else {
                    paintFace(context, canvas.width, canvas.height, 0.64)
                    paintHeldCard(context, canvas.width, canvas.height)
                }
            }
            paint()
            const stream = canvas.captureStream(15)
            const timer = setInterval(paint, 200)
            for (const track of stream.getTracks()) {
                track.addEventListener("ended", () => clearInterval(timer))
                const stop = track.stop.bind(track)
                track.stop = () => {
                    clearInterval(timer)
                    stop()
                }
            }
            return stream
        },
    }
}

export const sceneForFacing = (facing: CameraFacing): SyntheticScene =>
    facing === CameraFacing.Environment ? SyntheticScene.Document : SyntheticScene.Face

export function failingCamera(name: string): CameraService {
    return {
        open: async () => {
            throw new DOMException("Synthetic camera failure", name)
        },
    }
}

export const fakeRecorder: RecorderService = {
    supported: () => true,
    start: () => ({
        stop: async () => new Blob([WEBM_HEADER], {type: "video/webm"}),
        cancel: () => undefined,
    }),
}

// Answers the face frames with the given statuses in order, repeating the last
// one, or fails to open a session with the given rejection.
export function fakeLiveness(
    statuses: LivenessStatus[] = [LivenessStatus.ScanCompleted],
    rejection?: LivenessRejection
): LivenessConnector {
    return (): LivenessApi => {
        let frame = 0
        return {
            createSession: async () => {
                if (rejection !== undefined) throw new LivenessApiError(rejection)
                return "fake-session"
            },
            checkFrame: async () => {
                // Long enough for the checking state to show.
                await new Promise((resolve) => setTimeout(resolve, 300))
                const status = statuses[Math.min(frame, statuses.length - 1)]
                frame += 1
                return status
            },
            completeSession: async () => LivenessStatus.SessionCompleted,
            abort: async () => undefined,
        }
    }
}

// Records every upload; with a rejection, fails them all.
export function fakeUploads(
    uploaded: CaptureUpload[] = [],
    rejection?: UploadRejection
): UploadConnector {
    return (): CaptureUploader => ({
        upload: async (part, blob) => {
            if (rejection !== undefined) throw new UploadError(rejection)
            uploaded.push({part, blob})
        },
    })
}

export function fakeServices(options: {
    document?: Script<DocumentStatus>
    face?: Script<FaceStatus>
    camera?: CameraService
    recorder?: RecorderService
    liveness?: LivenessConnector
    uploads?: UploadConnector
    analyzersFail?: boolean
}): CaptureServices {
    return {
        camera: options.camera ?? syntheticCamera(sceneForFacing),
        recorder: options.recorder ?? fakeRecorder,
        liveness: options.liveness ?? fakeLiveness(),
        uploads: options.uploads ?? fakeUploads(),
        loadAnalyzers: async (): Promise<Analyzers> => {
            if (options.analyzersFail) {
                throw new Error("Synthetic analyzer failure")
            }
            return {
                document: scriptedDocument(
                    options.document ?? {statuses: [DocumentStatus.NoDocument]}
                ),
                face: scriptedFace(options.face ?? {statuses: [FaceStatus.NoFace]}),
            }
        },
        vibrate: () => undefined,
    }
}
