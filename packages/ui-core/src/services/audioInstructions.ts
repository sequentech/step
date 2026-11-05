// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EAudioInstructionsPolicy} from "../types/ElectionEventPresentation"

/** The voter screens that have their own audio instructions. */
export enum EAudioInstructionsScreen {
    ELECTION_CHOOSER = "election-chooser",
    START = "start",
    BALLOT = "ballot",
    REVIEW = "review",
    CONFIRMATION = "confirmation",
    AUDIT = "audit",
    BALLOT_LOCATOR = "ballot-locator",
    SUPPORT_MATERIALS = "support-materials",
}

/** Where the audio for a screen comes from. */
export enum EAudioInstructionsSource {
    NONE = "none",
    RECORDING = "recording",
    SYNTHESIS = "synthesis",
    /** No recording and no voice: the text can still be read on screen. */
    TRANSCRIPT_ONLY = "transcript-only",
}

/** Stored in a support material's `data.audio_instructions`. */
export interface IAudioInstructionsAssignment {
    screen: EAudioInstructionsScreen
    language: string
}

/** The fields of a support material that the instructions read. */
export interface IAudioInstructionsMaterial {
    id: string
    kind?: string | null
    document_id?: string | null
    data?: unknown
}

const SCREENS: string[] = Object.values(EAudioInstructionsScreen)

/** The screen and language an uploaded audio file is the instructions for, if any. */
export const getAudioInstructionsAssignment = (
    material: IAudioInstructionsMaterial
): IAudioInstructionsAssignment | undefined => {
    if (!material.document_id || !material.kind?.includes("audio")) {
        return undefined
    }
    const data = material.data
    if (typeof data !== "object" || data === null) {
        return undefined
    }
    const assignment = (data as {audio_instructions?: unknown}).audio_instructions
    if (typeof assignment !== "object" || assignment === null) {
        return undefined
    }
    const {screen, language} = assignment as {screen?: unknown; language?: unknown}
    if (typeof screen !== "string" || !SCREENS.includes(screen)) {
        return undefined
    }
    if (typeof language !== "string" || language === "") {
        return undefined
    }
    return {screen: screen as EAudioInstructionsScreen, language}
}

/** The recording for a screen: in the voter's language, else in the event's default one. */
export const findAudioInstructionsRecording = <T extends IAudioInstructionsMaterial>(
    materials: T[],
    screen: EAudioInstructionsScreen,
    language: string,
    defaultLanguage?: string
): T | undefined => {
    const inLanguage = (wanted?: string) =>
        wanted === undefined
            ? undefined
            : materials.find((material) => {
                  const assignment = getAudioInstructionsAssignment(material)
                  return assignment?.screen === screen && assignment.language === wanted
              })
    return inLanguage(language) ?? inLanguage(defaultLanguage)
}

/** Portal language codes whose speech voices are published under another tag. */
const VOICE_LANGUAGES: Record<string, string[]> = {
    tl: ["tl", "fil"],
    cat: ["ca"],
}

/** A speech synthesis voice that speaks the portal language, if the browser has one. */
export const findSpeechVoice = <T extends {lang: string}>(
    voices: T[],
    language: string
): T | undefined => {
    const wanted = VOICE_LANGUAGES[language] ?? [language]
    return voices.find((voice) => wanted.includes(voice.lang.toLowerCase().split(/[-_]/)[0]))
}

export const resolveAudioInstructionsSource = (
    policy: EAudioInstructionsPolicy | undefined,
    hasRecording: boolean,
    hasVoice: boolean
): EAudioInstructionsSource => {
    if (policy === undefined || policy === EAudioInstructionsPolicy.DISABLED) {
        return EAudioInstructionsSource.NONE
    }
    if (hasRecording) {
        return EAudioInstructionsSource.RECORDING
    }
    if (policy === EAudioInstructionsPolicy.RECORDED) {
        return EAudioInstructionsSource.NONE
    }
    return hasVoice ? EAudioInstructionsSource.SYNTHESIS : EAudioInstructionsSource.TRANSCRIPT_ONLY
}
