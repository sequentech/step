// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {LOST_MS, VideoCommand, idleVideo, trackVideo, type VideoSignals} from "./video"

const good: VideoSignals = {
    faceReady: true,
    faceInView: true,
    documentInView: true,
    now: 0,
    seconds: 5,
}

describe("trackVideo", () => {
    it("starts only when the face is ready and the ID is in view", () => {
        expect(trackVideo(idleVideo, {...good, documentInView: false}).command).toBe(
            VideoCommand.None
        )
        expect(trackVideo(idleVideo, {...good, faceReady: false}).command).toBe(VideoCommand.None)
        const started = trackVideo(idleVideo, {...good, now: 100})
        expect(started.command).toBe(VideoCommand.Start)
        expect(started.tracker.recordingSince).toBe(100)
    })

    it("finishes after the configured seconds", () => {
        const {tracker} = trackVideo(idleVideo, good)
        expect(trackVideo(tracker, {...good, now: 4999}).command).toBe(VideoCommand.None)
        const done = trackVideo(tracker, {...good, now: 5000})
        expect(done.command).toBe(VideoCommand.Finish)
        expect(done.tracker).toEqual(idleVideo)
    })

    it("tolerates a short loss of the face", () => {
        let {tracker} = trackVideo(idleVideo, good)
        const lost = {...good, faceReady: false, faceInView: false}
        ;({tracker} = trackVideo(tracker, {...lost, now: 1000}))
        expect(tracker.lostSince).toBe(1000)
        const back = trackVideo(tracker, {...good, now: 1000 + LOST_MS})
        expect(back.command).toBe(VideoCommand.None)
        expect(back.tracker.lostSince).toBeNull()
    })

    it("restarts when the face or the ID is lost for over a second", () => {
        let {tracker} = trackVideo(idleVideo, good)
        const noId = {...good, documentInView: false}
        ;({tracker} = trackVideo(tracker, {...noId, now: 1000}))
        const restart = trackVideo(tracker, {...noId, now: 1001 + LOST_MS})
        expect(restart.command).toBe(VideoCommand.Restart)
        expect(restart.tracker).toEqual(idleVideo)
    })

    it("keeps recording through a still, not yet stable face", () => {
        const {tracker} = trackVideo(idleVideo, good)
        const next = trackVideo(tracker, {...good, faceReady: false, now: 3000})
        expect(next.command).toBe(VideoCommand.None)
        expect(next.tracker.lostSince).toBeNull()
    })
})
