// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {EAudioInstructionsPolicy} from "../types/ElectionEventPresentation"
import {
    EAudioInstructionsScreen,
    EAudioInstructionsSource,
    IAudioInstructionsMaterial,
    findAudioInstructionsRecording,
    findSpeechVoice,
    getAudioInstructionsAssignment,
    resolveAudioInstructionsSource,
} from "./audioInstructions"

const recording = (
    id: string,
    screen: unknown,
    language: unknown,
    overrides: Partial<IAudioInstructionsMaterial> = {}
): IAudioInstructionsMaterial => ({
    id,
    kind: "audio/mpeg",
    document_id: `document-${id}`,
    data: {title_i18n: {en: id}, audio_instructions: {screen, language}},
    ...overrides,
})

it("reads the screen and language a recording is assigned to", () => {
    expect(getAudioInstructionsAssignment(recording("a", "ballot", "tl"))).toEqual({
        screen: EAudioInstructionsScreen.BALLOT,
        language: "tl",
    })
})

it.each([
    ["no data", {id: "a", kind: "audio/mpeg", document_id: "d"}],
    ["no assignment", {id: "a", kind: "audio/mpeg", document_id: "d", data: {}}],
    ["an unknown screen", recording("a", "lobby", "en")],
    ["no language", recording("a", "ballot", "")],
    ["a non-string language", recording("a", "ballot", 3)],
    ["a file that is not audio", recording("a", "ballot", "en", {kind: "application/pdf"})],
    ["no uploaded file", recording("a", "ballot", "en", {document_id: null})],
])("ignores a material with %s", (_case, material) => {
    expect(getAudioInstructionsAssignment(material)).toBeUndefined()
})

it("picks the recording for the screen in the voter's language", () => {
    const materials = [
        recording("review-en", "review", "en"),
        recording("ballot-en", "ballot", "en"),
        recording("ballot-tl", "ballot", "tl"),
    ]
    expect(
        findAudioInstructionsRecording(materials, EAudioInstructionsScreen.BALLOT, "tl", "en")?.id
    ).toBe("ballot-tl")
})

it("falls back to the event's default language, and to nothing after that", () => {
    const materials = [recording("ballot-en", "ballot", "en")]
    expect(
        findAudioInstructionsRecording(materials, EAudioInstructionsScreen.BALLOT, "tl", "en")?.id
    ).toBe("ballot-en")
    expect(
        findAudioInstructionsRecording(materials, EAudioInstructionsScreen.BALLOT, "tl", "es")
    ).toBeUndefined()
    expect(
        findAudioInstructionsRecording(materials, EAudioInstructionsScreen.REVIEW, "en", "en")
    ).toBeUndefined()
    expect(
        findAudioInstructionsRecording([], EAudioInstructionsScreen.BALLOT, "en", undefined)
    ).toBeUndefined()
})

it("finds a voice by language, treating Tagalog and Filipino as one", () => {
    const voices = [{lang: "en-US"}, {lang: "fil-PH"}, {lang: "es_ES"}, {lang: "ca-ES"}]
    expect(findSpeechVoice(voices, "en")).toBe(voices[0])
    expect(findSpeechVoice(voices, "tl")).toBe(voices[1])
    expect(findSpeechVoice(voices, "es")).toBe(voices[2])
    expect(findSpeechVoice(voices, "cat")).toBe(voices[3])
    expect(findSpeechVoice(voices, "eu")).toBeUndefined()
    expect(findSpeechVoice([], "en")).toBeUndefined()
})

it.each([
    [EAudioInstructionsPolicy.DISABLED, true, true, EAudioInstructionsSource.NONE],
    [undefined, true, true, EAudioInstructionsSource.NONE],
    [EAudioInstructionsPolicy.RECORDED, true, true, EAudioInstructionsSource.RECORDING],
    [EAudioInstructionsPolicy.RECORDED, false, true, EAudioInstructionsSource.NONE],
    [
        EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED,
        true,
        true,
        EAudioInstructionsSource.RECORDING,
    ],
    [
        EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED,
        false,
        true,
        EAudioInstructionsSource.SYNTHESIS,
    ],
    [
        EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED,
        false,
        false,
        EAudioInstructionsSource.TRANSCRIPT_ONLY,
    ],
])(
    "with policy %p, recording %p and voice %p the source is %p",
    (policy, hasRecording, hasVoice, expected) => {
        expect(resolveAudioInstructionsSource(policy, hasRecording, hasVoice)).toBe(expected)
    }
)
