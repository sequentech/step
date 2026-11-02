// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useId, useRef, useState} from "react"
import CloseIcon from "@mui/icons-material/Close"
import HelpIcon from "@mui/icons-material/Help"
import PauseIcon from "@mui/icons-material/Pause"
import StopIcon from "@mui/icons-material/Stop"
import VolumeUpIcon from "@mui/icons-material/VolumeUp"
import {Box, IconButton, Popover, Tooltip, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    EAudioInstructionsPolicy,
    EAudioInstructionsSource,
    findSpeechVoice,
    resolveAudioInstructionsSource,
    toBCP47,
} from "@sequentech/ui-core"
import VisuallyHidden from "../VisuallyHidden/VisuallyHidden"
import {useInertBackground} from "../../services/useInertBackground"

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
 * Its controls are small icon buttons named by their tooltips, for the header next to the
 * accessibility button. The text opens in a popover under its icon, which Escape, a click
 * outside it or its own close button closes.
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
    const [transcriptAnchor, setTranscriptAnchor] = useState<HTMLElement | null>(null)
    const transcriptOpen = Boolean(transcriptAnchor)
    const setModalRoot = useInertBackground(transcriptOpen)
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
    const transcriptLabel = transcriptOpen
        ? t("audioInstructions.hideTranscript")
        : t("audioInstructions.showTranscript")

    return (
        <Box className="audio-instructions" sx={{display: "flex"}}>
            <Box
                className="audio-instructions-region"
                component="section"
                aria-label={t("audioInstructions.label")}
                sx={{display: "flex", alignItems: "center", gap: "4px"}}
            >
                {canPlay ? (
                    <Tooltip title={toggle.label}>
                        <IconButton
                            className="audio-instructions-play"
                            size="small"
                            sx={{color: "brandColor"}}
                            aria-label={toggle.label}
                            onClick={toggle.action}
                        >
                            <toggle.icon aria-hidden />
                        </IconButton>
                    </Tooltip>
                ) : null}
                {canPlay && playback !== EPlayback.IDLE ? (
                    <Tooltip title={t("audioInstructions.stop")}>
                        <IconButton
                            className="audio-instructions-stop"
                            size="small"
                            sx={{color: "brandColor"}}
                            aria-label={t("audioInstructions.stop")}
                            onClick={stop}
                        >
                            <StopIcon aria-hidden />
                        </IconButton>
                    </Tooltip>
                ) : null}
                {text === "" ? null : (
                    <>
                        <Tooltip title={transcriptLabel}>
                            <IconButton
                                className="audio-instructions-transcript-toggle"
                                size="small"
                                sx={{color: "brandColor"}}
                                aria-label={transcriptLabel}
                                aria-expanded={transcriptOpen}
                                aria-controls={transcriptId}
                                onClick={(event) =>
                                    setTranscriptAnchor(transcriptOpen ? null : event.currentTarget)
                                }
                            >
                                {transcriptOpen ? (
                                    <CloseIcon aria-hidden />
                                ) : (
                                    <HelpIcon aria-hidden />
                                )}
                            </IconButton>
                        </Tooltip>
                        {/* Kept mounted so that aria-controls always names an element. */}
                        <Popover
                            ref={setModalRoot}
                            className="audio-instructions-popover"
                            anchorEl={transcriptAnchor}
                            anchorOrigin={{vertical: "bottom", horizontal: "right"}}
                            keepMounted
                            transformOrigin={{vertical: "top", horizontal: "right"}}
                            slotProps={{
                                paper: {
                                    "role": "dialog",
                                    "aria-label": t("audioInstructions.label"),
                                    "aria-describedby": transcriptId,
                                    "tabIndex": -1,
                                    "sx": {
                                        display: "flex",
                                        alignItems: "flex-start",
                                        gap: "8px",
                                        padding: "12px 12px 12px 16px",
                                    },
                                },
                            }}
                            open={transcriptOpen}
                            onClose={() => setTranscriptAnchor(null)}
                        >
                            <Box
                                className="audio-instructions-transcript"
                                id={transcriptId}
                                lang={toBCP47(language)}
                                sx={{maxWidth: "30rem"}}
                            >
                                {transcriptOpen ? (
                                    <Typography className="audio-instructions-text" component="p">
                                        {text}
                                    </Typography>
                                ) : null}
                            </Box>
                            <Tooltip title={t("audioInstructions.hideTranscript")}>
                                <IconButton
                                    className="audio-instructions-transcript-close"
                                    size="small"
                                    sx={{color: "brandColor"}}
                                    aria-label={t("audioInstructions.hideTranscript")}
                                    onClick={() => setTranscriptAnchor(null)}
                                >
                                    <CloseIcon aria-hidden />
                                </IconButton>
                            </Tooltip>
                        </Popover>
                    </>
                )}
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
        </Box>
    )
}

export default AudioInstructions
