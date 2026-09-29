// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef} from "react"
import type {ScanovateLiveness} from "../KcContext"
import {LivenessEventType, livenessFrameUrl, livenessProblem, parseLivenessEvent} from "./liveness"
import type {CaptureProblem} from "./types"

// Liveness Plus drives the voter through the check itself; we only learn when it
// ends. `attempt` reloads the iframe.
export default function LivenessFrame(props: {
    liveness: ScanovateLiveness
    attempt: number
    languageTag: string
    title: string
    onDone: () => void
    onProblem: (problem: CaptureProblem | null) => void
}) {
    const {liveness, attempt, languageTag, title, onDone, onProblem} = props
    const frameRef = useRef<HTMLIFrameElement | null>(null)
    const handlers = useRef({onDone, onProblem})
    useEffect(() => {
        handlers.current = {onDone, onProblem}
    })

    useEffect(() => {
        const onMessage = (event: MessageEvent) => {
            if (event.origin !== liveness.origin) return
            if (event.source !== frameRef.current?.contentWindow) return
            const message = parseLivenessEvent(event.data)
            if (message?.type === LivenessEventType.Done) {
                handlers.current.onDone()
            } else if (message?.type === LivenessEventType.Error) {
                handlers.current.onProblem(livenessProblem(message.errorCode))
            }
        }
        window.addEventListener("message", onMessage)
        return () => window.removeEventListener("message", onMessage)
    }, [liveness.origin])

    return (
        <div className="capture-liveness">
            <iframe
                key={attempt}
                ref={frameRef}
                src={livenessFrameUrl(liveness.url, liveness.languages, languageTag)}
                title={title}
                allow="camera *; microphone *"
                onLoad={(event) => event.currentTarget.contentWindow?.focus()}
            />
        </div>
    )
}
