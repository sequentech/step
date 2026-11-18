// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The steps of the capture page: Scanovate's own pages, embedded one after the
// other, and then, when Keycloak asks for it, the page's own photo of the voter
// holding their document.
export enum CaptureStage {
    // Scanovate's OCR page scans the document.
    Document = "DOCUMENT",
    // Scanovate's Liveness Plus page checks the voter's face.
    Face = "FACE",
    // The voter takes a photo of themselves holding their document.
    Holding = "HOLDING",
    // All done: the page hands over to Keycloak, which has the results.
    Checking = "CHECKING",
}

// What the page tells the voter after a Scanovate page reports an error.
export enum CaptureProblem {
    Camera = "CAMERA",
    Expired = "EXPIRED",
    // The one-time token is no longer valid: only new tokens from Keycloak help.
    InvalidToken = "INVALID_TOKEN",
    // The voter closed the Scanovate page.
    Closed = "CLOSED",
    // The holding photo couldn't be sent to Keycloak.
    Upload = "UPLOAD",
    Generic = "GENERIC",
}

// The client error codes of Scanovate's OCR and Liveness Plus web pages.
export enum ScanovateErrorCode {
    CouldNotInitAnalytics = 1000,
    CouldNotCreateSession = 1001,
    MaxActiveSessionsReached = 1002,
    CameraPermissionBlocked = 1003,
    CameraNotFound = 1004,
    CouldNotOpenCamera = 1005,
    SessionExpired = 1006,
    ServerError = 1007,
    SlowNetwork = 1008,
    UserPressedClose = 1009,
    UserLeftPage = 1010,
    NoNetwork = 1011,
    ClientError = 1012,
    CameraPermissionDismissed = 1013,
    InvalidToken = 1014,
    UserClosedPage = 1015,
    CouldNotStartMediaRecorder = 1016,
}

// The messages of a Scanovate page that the capture page acts on; it ignores the rest.
export enum ScanovateMessageKind {
    Done = "DONE",
    Error = "ERROR",
}

export type ScanovateMessage =
    | {kind: ScanovateMessageKind.Done}
    | {kind: ScanovateMessageKind.Error; code: number | null}
