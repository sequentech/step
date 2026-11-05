// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useId, useRef, useState} from "react"
import PauseIcon from "@mui/icons-material/Pause"
import StopIcon from "@mui/icons-material/Stop"
import VolumeUpIcon from "@mui/icons-material/VolumeUp"
import {Box, Button, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    EAudioInstructionsPolicy,
    EAudioInstructionsSource,
    findSpeechVoice,
    resolveAudioInstructionsSource,
    toBCP47,
} from "@sequentech/ui-core"
import PageLimit from "../PageLimit/PageLimit"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"

export interface AudioInstructionsProps {
    /** Absent, nothing is rendered. */
    policy?: EAudioInstructionsPolicy
    /** How to use the screen, in the voter's language. Spoken, and shown as the transcript. */
    text: string
    /** The portal language code of `text`. */
    language: string
    /** The uploaded recording for this screen. It is played in preference to synthesis. */
    recordingUrl?: string
}

enum EPlayback {
    IDLE = "idle",
    PLAYING = "playing",
    PAUSED = "paused",
}

const getSpeech = (): SpeechSynthesis | undefined =>
    typeof window !== "undefined" && "speechSynthesis" in window
        ? window.speechSynthesis
        : undefined

const useSpeechVoices = (): SpeechSynthesisVoice[] => {
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
 * Spoken instructions for the screen the voter is on, with the same text to read.
 *
 * It plays the event's recording where there is one and the browser's speech synthesis
 * otherwise. It never starts by itself: a voter using a screen reader would hear both at once.
 */
const AudioInstructions: React.FC<AudioInstructionsProps> = ({
    policy,
    text,
    language,
    recordingUrl,
}) => {
    const {t} = useTranslation()
    const [playback, setPlayback] = useState(EPlayback.IDLE)
    const [announcement, setAnnouncement] = useState("")
    const [transcriptOpen, setTranscriptOpen] = useState(false)
    const audioRef = useRef<HTMLAudioElement>(null)
    const transcriptId = useId()
    const voice = findSpeechVoice(useSpeechVoices(), language)
    const source = resolveAudioInstructionsSource(
        policy,
        Boolean(recordingUrl),
        Boolean(voice) && text !== ""
    )

    const silence = useCallback(() => {
        const audio = audioRef.current
        if (audio) {
            audio.pause()
            audio.currentTime = 0
        }
        getSpeech()?.cancel()
        setPlayback(EPlayback.IDLE)
    }, [])

    // Leaving the screen, or changing what would be said, ends what is being said.
    useEffect(() => silence, [silence, text, language, recordingUrl])

    const start = async () => {
        if (source === EAudioInstructionsSource.RECORDING) {
            try {
                await audioRef.current?.play()
            } catch {
                return
            }
        } else {
            const speech = getSpeech()
            if (!speech || !voice) {
                return
            }
            const utterance = new SpeechSynthesisUtterance(text)
            utterance.voice = voice
            utterance.lang = voice.lang
            utterance.onend = () => setPlayback(EPlayback.IDLE)
            utterance.onerror = () => setPlayback(EPlayback.IDLE)
            speech.cancel()
            speech.speak(utterance)
        }
        setPlayback(EPlayback.PLAYING)
        setAnnouncement(t("audioInstructions.playing"))
    }

    const pause = () => {
        if (source === EAudioInstructionsSource.RECORDING) {
            audioRef.current?.pause()
        } else {
            getSpeech()?.pause()
        }
        setPlayback(EPlayback.PAUSED)
        setAnnouncement(t("audioInstructions.paused"))
    }

    const resume = async () => {
        if (source === EAudioInstructionsSource.RECORDING) {
            try {
                await audioRef.current?.play()
            } catch {
                return
            }
        } else {
            getSpeech()?.resume()
        }
        setPlayback(EPlayback.PLAYING)
        setAnnouncement(t("audioInstructions.playing"))
    }

    const stop = () => {
        silence()
        setAnnouncement(t("audioInstructions.stopped"))
    }

    if (source === EAudioInstructionsSource.NONE || (text === "" && !recordingUrl)) {
        return null
    }

    const canPlay = source !== EAudioInstructionsSource.TRANSCRIPT_ONLY
    const toggle = {
        [EPlayback.IDLE]: {label: t("audioInstructions.play"), action: start, icon: VolumeUpIcon},
        [EPlayback.PLAYING]: {label: t("audioInstructions.pause"), action: pause, icon: PauseIcon},
        [EPlayback.PAUSED]: {
            label: t("audioInstructions.resume"),
            action: resume,
            icon: VolumeUpIcon,
        },
    }[playback]

    return (
        <PageLimit className="audio-instructions" maxWidth="lg">
            <Box
                className="audio-instructions-region"
                component="section"
                aria-label={t("audioInstructions.label")}
                sx={{display: "flex", flexWrap: "wrap", gap: "8px", paddingTop: "16px"}}
            >
                {canPlay ? (
                    <Button
                        className="audio-instructions-play"
                        variant="secondary"
                        onClick={toggle.action}
                        startIcon={<toggle.icon aria-hidden />}
                    >
                        {toggle.label}
                    </Button>
                ) : null}
                {canPlay && playback !== EPlayback.IDLE ? (
                    <Button
                        className="audio-instructions-stop"
                        variant="secondary"
                        onClick={stop}
                        startIcon={<StopIcon aria-hidden />}
                    >
                        {t("audioInstructions.stop")}
                    </Button>
                ) : null}
                {text === "" ? null : (
                    <Button
                        className="audio-instructions-transcript-toggle"
                        variant="secondary"
                        aria-expanded={transcriptOpen}
                        aria-controls={transcriptId}
                        onClick={() => setTranscriptOpen((open) => !open)}
                    >
                        {transcriptOpen
                            ? t("audioInstructions.hideTranscript")
                            : t("audioInstructions.showTranscript")}
                    </Button>
                )}
                <Box
                    className="audio-instructions-transcript"
                    id={transcriptId}
                    lang={toBCP47(language)}
                    sx={{flexBasis: "100%"}}
                >
                    {transcriptOpen ? (
                        <Typography className="audio-instructions-text" component="p">
                            {text}
                        </Typography>
                    ) : null}
                </Box>
                <VisuallyHidden className="audio-instructions-status" role="status">
                    {announcement}
                </VisuallyHidden>
                {recordingUrl ? (
                    <audio
                        className="audio-instructions-recording"
                        ref={audioRef}
                        src={recordingUrl}
                        preload="none"
                        onEnded={() => setPlayback(EPlayback.IDLE)}
                    />
                ) : null}
            </Box>
        </PageLimit>
    )
}

export default AudioInstructions
