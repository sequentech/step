// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// A recording is thrown away when the face or the ID is out of view this long.
export const LOST_MS = 1000

export enum VideoCommand {
    None = "NONE",
    Start = "START",
    Restart = "RESTART",
    Finish = "FINISH",
}

export type VideoTracker = {recordingSince: number | null; lostSince: number | null}

export type VideoSignals = {
    // Face READY: stable inside the oval.
    faceReady: boolean
    // Face READY or HOLD_STILL.
    faceInView: boolean
    documentInView: boolean
    now: number
    seconds: number
}

export const idleVideo: VideoTracker = {recordingSince: null, lostSince: null}

export function trackVideo(
    tracker: VideoTracker,
    signals: VideoSignals
): {tracker: VideoTracker; command: VideoCommand} {
    const {faceReady, faceInView, documentInView, now, seconds} = signals
    if (tracker.recordingSince === null) {
        return faceReady && documentInView
            ? {tracker: {recordingSince: now, lostSince: null}, command: VideoCommand.Start}
            : {tracker, command: VideoCommand.None}
    }
    if (now - tracker.recordingSince >= seconds * 1000) {
        return {tracker: idleVideo, command: VideoCommand.Finish}
    }
    if (faceInView && documentInView) {
        return {tracker: {...tracker, lostSince: null}, command: VideoCommand.None}
    }
    const lostSince = tracker.lostSince ?? now
    if (now - lostSince > LOST_MS) {
        return {tracker: idleVideo, command: VideoCommand.Restart}
    }
    return {tracker: {...tracker, lostSince}, command: VideoCommand.None}
}
