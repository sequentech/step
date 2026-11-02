// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useId, useState} from "react"
import IconButton from "@mui/material/IconButton"
import SvgIcon from "@mui/material/SvgIcon"
import Tooltip from "@mui/material/Tooltip"
import {
    EAudioInstructionsSource,
    findSpeechVoice,
    resolveAudioInstructionsSource,
} from "../../../../ui-core/src/services/audioInstructions"
import type {EAudioInstructionsPolicy} from "../../../../ui-core/src/types/ElectionEventPresentation"
import type {AccessibilityCopy} from "./copy"

type Playback = "idle" | "playing" | "paused"

// The Material icons of the Voting Portal's controls; this theme does not bundle the icon set.
const ICONS = {
    play: "M3 9v6h4l5 5V4L7 9zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02M14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77",
    pause: "M6 19h4V5H6zm8-14v14h4V5z",
    stop: "M6 6h12v12H6z",
    help: "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2m1 17h-2v-2h2zm2.07-7.75-.9.92C13.45 12.9 13 13.5 13 15h-2v-.5c0-1.1.45-2.1 1.17-2.83l1.24-1.26c.37-.36.59-.86.59-1.41 0-1.1-.9-2-2-2s-2 .9-2 2H8c0-2.21 1.79-4 4-4s4 1.79 4 4c0 .88-.36 1.68-.93 2.25",
    close: "M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z",
}

/** A small icon button, named by its tooltip. */
function IconControl(props: {
    label: string
    icon: keyof typeof ICONS
    onClick: () => void
    expanded?: boolean
    controls?: string
}) {
    return (
        <Tooltip title={props.label}>
            <IconButton
                size="small"
                aria-label={props.label}
                aria-expanded={props.expanded}
                aria-controls={props.controls}
                onClick={props.onClick}
            >
                <SvgIcon aria-hidden>
                    <path d={ICONS[props.icon]} />
                </SvgIcon>
            </IconButton>
        </Tooltip>
    )
}

const getSpeech = (): SpeechSynthesis | undefined =>
    "speechSynthesis" in window ? window.speechSynthesis : undefined

function useSpeechVoices(): SpeechSynthesisVoice[] {
    const [voices, setVoices] = useState<SpeechSynthesisVoice[]>(
        () => getSpeech()?.getVoices() ?? []
    )

    useEffect(() => {
        const speech = getSpeech()
        if (!speech) {
            return
        }
        // Browsers load their voices after the page; the list is empty until then.
        const update = () => setVoices(speech.getVoices())
        speech.addEventListener("voiceschanged", update)
        // The one "voiceschanged" can fire between the first read and this subscription.
        update()
        return () => speech.removeEventListener("voiceschanged", update)
    }, [])

    return voices
}

/**
 * Spoken instructions for a login page, with the same text to read.
 *
 * The login pages have no recordings: the browser's voice reads the text where it has one for
 * the language. It never starts by itself, or it would talk over a screen reader.
 */
export function AudioInstructions(props: {
    policy: EAudioInstructionsPolicy
    text: string
    copy: AccessibilityCopy
}) {
    const {policy, text, copy} = props
    const words = copy.audioInstructions
    const [playback, setPlayback] = useState<Playback>("idle")
    const [announcement, setAnnouncement] = useState("")
    const [transcriptOpen, setTranscriptOpen] = useState(false)
    const transcriptId = useId()
    const voice = findSpeechVoice(useSpeechVoices(), copy.portalLanguage)
    const source = resolveAudioInstructionsSource(policy, false, Boolean(voice))

    const silence = useCallback(() => {
        getSpeech()?.cancel()
        setPlayback("idle")
    }, [])

    useEffect(() => silence, [silence, text])

    if (source === EAudioInstructionsSource.NONE) {
        return null
    }

    const start = () => {
        const speech = getSpeech()
        if (!speech || !voice) {
            return
        }
        const utterance = new SpeechSynthesisUtterance(text)
        utterance.voice = voice
        utterance.lang = voice.lang
        utterance.onend = () => setPlayback("idle")
        utterance.onerror = () => setPlayback("idle")
        speech.cancel()
        speech.speak(utterance)
        setPlayback("playing")
        setAnnouncement(words.playing)
    }

    const toggle = {
        idle: {label: words.play, icon: "play" as const, action: start},
        playing: {
            label: words.pause,
            icon: "pause" as const,
            action: () => {
                getSpeech()?.pause()
                setPlayback("paused")
                setAnnouncement(words.paused)
            },
        },
        paused: {
            label: words.resume,
            icon: "play" as const,
            action: () => {
                getSpeech()?.resume()
                setPlayback("playing")
                setAnnouncement(words.playing)
            },
        },
    }[playback]

    return (
        <section
            className="auth-audio-instructions"
            aria-label={words.label}
            lang={copy.languageTag}
        >
            {source === EAudioInstructionsSource.SYNTHESIS && (
                <IconControl label={toggle.label} icon={toggle.icon} onClick={toggle.action} />
            )}
            {source === EAudioInstructionsSource.SYNTHESIS && playback !== "idle" && (
                <IconControl
                    label={words.stop}
                    icon="stop"
                    onClick={() => {
                        silence()
                        setAnnouncement(words.stopped)
                    }}
                />
            )}
            <IconControl
                label={transcriptOpen ? words.hideTranscript : words.showTranscript}
                icon={transcriptOpen ? "close" : "help"}
                expanded={transcriptOpen}
                controls={transcriptId}
                onClick={() => setTranscriptOpen((open) => !open)}
            />
            <div id={transcriptId} className="auth-audio-instructions-transcript">
                {transcriptOpen && <p>{text}</p>}
            </div>
            <span className="auth-visually-hidden" role="status">
                {announcement}
            </span>
        </section>
    )
}
