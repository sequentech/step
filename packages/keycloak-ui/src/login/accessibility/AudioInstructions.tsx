// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useCallback, useEffect, useId, useState} from "react"
import Button from "@mui/material/Button"
import {
    EAudioInstructionsSource,
    findSpeechVoice,
    resolveAudioInstructionsSource,
} from "../../../../ui-core/src/services/audioInstructions"
import type {EAudioInstructionsPolicy} from "../../../../ui-core/src/types/ElectionEventPresentation"
import type {AccessibilityCopy} from "./copy"

type Playback = "idle" | "playing" | "paused"

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
        idle: {label: words.play, action: start},
        playing: {
            label: words.pause,
            action: () => {
                getSpeech()?.pause()
                setPlayback("paused")
                setAnnouncement(words.paused)
            },
        },
        paused: {
            label: words.resume,
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
                <Button variant="outlined" size="small" onClick={toggle.action}>
                    {toggle.label}
                </Button>
            )}
            {source === EAudioInstructionsSource.SYNTHESIS && playback !== "idle" && (
                <Button
                    variant="outlined"
                    size="small"
                    onClick={() => {
                        silence()
                        setAnnouncement(words.stopped)
                    }}
                >
                    {words.stop}
                </Button>
            )}
            <Button
                variant="outlined"
                size="small"
                aria-expanded={transcriptOpen}
                aria-controls={transcriptId}
                onClick={() => setTranscriptOpen((open) => !open)}
            >
                {transcriptOpen ? words.hideTranscript : words.showTranscript}
            </Button>
            <div id={transcriptId} className="auth-audio-instructions-transcript">
                {transcriptOpen && <p>{text}</p>}
            </div>
            <span className="auth-visually-hidden" role="status">
                {announcement}
            </span>
        </section>
    )
}
