// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {LivenessConnector} from "./livenessApi"
import type {UploadConnector} from "./uploads"

// Frame analysis results returned by the id-capture WebAssembly module.
export type Point = [number, number]

export enum DocumentStatus {
    NoDocument = "NO_DOCUMENT",
    TooFar = "TOO_FAR",
    TooClose = "TOO_CLOSE",
    NotAligned = "NOT_ALIGNED",
    TooDark = "TOO_DARK",
    TooBright = "TOO_BRIGHT",
    Glare = "GLARE",
    Blurry = "BLURRY",
    HoldStill = "HOLD_STILL",
    Ready = "READY",
}

export interface DocumentFrame {
    status: DocumentStatus
    corners: Point[] | null
    fill: number
    sharpness: number
    glare: number
    brightness: number
    stability: number
}

export enum FaceStatus {
    NoFace = "NO_FACE",
    MultipleFaces = "MULTIPLE_FACES",
    TooFar = "TOO_FAR",
    TooClose = "TOO_CLOSE",
    OffCenter = "OFF_CENTER",
    TurnToCamera = "TURN_TO_CAMERA",
    TooDark = "TOO_DARK",
    TooBright = "TOO_BRIGHT",
    Blurry = "BLURRY",
    HoldStill = "HOLD_STILL",
    Ready = "READY",
}

export interface FaceFrame {
    status: FaceStatus
    box: {x: number; y: number; width: number; height: number} | null
    landmarks: Point[]
    yaw: number
    roll: number
    brightness: number
    sharpness: number
    stability: number
}

export type Pixels = Uint8Array | Uint8ClampedArray

export interface DocumentAnalyzer {
    analyze(
        rgba: Pixels,
        width: number,
        height: number,
        guideX: number,
        guideY: number,
        guideWidth: number,
        guideHeight: number
    ): DocumentFrame
    reset(): void
    free(): void
}

export interface FaceAnalyzer {
    analyze(
        rgba: Pixels,
        width: number,
        height: number,
        ovalCenterX: number,
        ovalCenterY: number,
        ovalRadiusX: number,
        ovalRadiusY: number
    ): FaceFrame
    reset(): void
    free(): void
}

export interface Analyzers {
    document: DocumentAnalyzer
    face: FaceAnalyzer
}

export type Size = {width: number; height: number}
export type Rect = {x: number; y: number; width: number; height: number}
export type Ellipse = {cx: number; cy: number; rx: number; ry: number}

export enum CaptureStep {
    Front = "FRONT",
    Back = "BACK",
    Face = "FACE",
    Video = "VIDEO",
}

export enum CameraFacing {
    Environment = "environment",
    User = "user",
}

export enum CaptureProblem {
    CameraDenied = "CAMERA_DENIED",
    CameraNotFound = "CAMERA_NOT_FOUND",
    CameraInUse = "CAMERA_IN_USE",
    InsecureContext = "INSECURE_CONTEXT",
    CameraFailed = "CAMERA_FAILED",
    AnalyzerFailed = "ANALYZER_FAILED",
    LivenessFailed = "LIVENESS_FAILED",
    // Keycloak's one-time liveness token is used up: only a new page gets a new one.
    LivenessExpired = "LIVENESS_EXPIRED",
    UploadFailed = "UPLOAD_FAILED",
    // Keycloak no longer takes uploads for this page: only a new page can send them.
    CaptureExpired = "CAPTURE_EXPIRED",
}

export interface CameraService {
    open(facing: CameraFacing): Promise<MediaStream>
}

export interface CaptureServices {
    camera: CameraService
    liveness: LivenessConnector
    uploads: UploadConnector
    loadAnalyzers(): Promise<Analyzers>
    vibrate(pattern: number): void
}
