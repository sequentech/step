// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {EAudioInstructionsPolicy, EAudioInstructionsScreen} from "@sequentech/ui-core"
import {GET_DOCUMENT} from "../../queries/GetDocument"
import {GET_SUPPORT_MATERIALS} from "../../queries/GetSupportMaterials"
import {ScreenAudioInstructions, audioInstructionsScreenAt} from "./ScreenAudioInstructions"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => `text of ${key}`, i18n: {language: "tl"}}),
}))
jest.mock("@sequentech/ui-essentials", () => ({
    AudioInstructions: (props: {
        policy: string
        text: string
        language: string
        recordingUrl?: string
    }) => (
        <div
            data-testid="audio-instructions"
            data-policy={props.policy}
            data-text={props.text}
            data-language={props.language}
            data-recording={props.recordingUrl ?? ""}
        />
    ),
}))
jest.mock("../../store/hooks", () => ({
    useAppSelector: (selector: () => unknown) => selector(),
}))
jest.mock("../../store/ballotStyles/ballotStylesSlice", () => ({
    selectFirstBallotStyle: () => mockBallotStyle,
}))
jest.mock("../../store/electionEvents/electionEventsSlice", () => ({
    selectElectionEventById: () => () => mockElectionEvent,
}))
jest.mock("../../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {
            get DISABLE_AUTH() {
                return mockDisableAuth
            },
            PUBLIC_BUCKET_URL: "https://files.invalid/",
        },
    }),
}))
jest.mock("@apollo/client/react", () => ({
    useQuery: (query: unknown, options: {skip?: boolean; variables?: unknown}) => {
        mockQueries.push({query, ...options})
        if (options.skip) {
            return {data: undefined}
        }
        return {data: query === mockMaterialsQuery ? mockMaterials : mockDocuments}
    },
}))

const mockMaterialsQuery = GET_SUPPORT_MATERIALS
let mockQueries: {query: unknown; skip?: boolean; variables?: unknown}[]
let mockBallotStyle: unknown
let mockElectionEvent: unknown
let mockDisableAuth: boolean
let mockMaterials: unknown
let mockDocuments: unknown

const presentation = (policy?: EAudioInstructionsPolicy) => ({
    audio_instructions_policy: policy,
    language_conf: {default_language_code: "en"},
})

const recording = (id: string, screenName: string, language: string) => ({
    id,
    kind: "audio/mpeg",
    document_id: `document-${id}`,
    data: {audio_instructions: {screen: screenName, language}},
})

const at = (path: string) =>
    render(
        <MemoryRouter initialEntries={[path]}>
            <Routes>
                <Route
                    path="/tenant/:tenantId/event/:eventId/*"
                    element={<ScreenAudioInstructions />}
                />
            </Routes>
        </MemoryRouter>
    )

const BALLOT = "/tenant/north/event/mayor/election/council/vote"

beforeEach(() => {
    mockQueries = []
    mockBallotStyle = undefined
    mockElectionEvent = {
        presentation: presentation(EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED),
    }
    mockDisableAuth = false
    mockMaterials = {sequent_backend_support_material: []}
    mockDocuments = {sequent_backend_document: []}
})

it.each([
    ["/tenant/t/event/e/election-chooser", EAudioInstructionsScreen.ELECTION_CHOOSER],
    ["/tenant/t/event/e/materials", EAudioInstructionsScreen.SUPPORT_MATERIALS],
    ["/tenant/t/event/e/election/x/start", EAudioInstructionsScreen.START],
    ["/tenant/t/event/e/election/x/vote", EAudioInstructionsScreen.BALLOT],
    ["/tenant/t/event/e/election/x/review", EAudioInstructionsScreen.REVIEW],
    ["/tenant/t/event/e/election/x/confirmation", EAudioInstructionsScreen.CONFIRMATION],
    ["/tenant/t/event/e/election/x/audit", EAudioInstructionsScreen.AUDIT],
    ["/tenant/t/event/e/election/x/ballot-locator", EAudioInstructionsScreen.BALLOT_LOCATOR],
    ["/tenant/t/event/e/election/x/ballot-locator/abc123", EAudioInstructionsScreen.BALLOT_LOCATOR],
    ["/tenant/t/event/e/login", undefined],
    ["/tenant/t/event/e/enroll", undefined],
    ["/", undefined],
])("maps %s to its screen", (path, expected) => {
    expect(audioInstructionsScreenAt(path)).toBe(expected)
})

it("gives the screen's own text in the voter's language", () => {
    at(BALLOT)
    const instructions = screen.getByTestId("audio-instructions")
    expect(instructions.dataset.text).toBe("text of audioInstructions.screens.ballot")
    expect(instructions.dataset.language).toBe("tl")
    expect(instructions.dataset.policy).toBe("recorded-or-synthesized")
    expect(instructions.dataset.recording).toBe("")
})

it("plays the event's recording for the screen and language", () => {
    mockMaterials = {
        sequent_backend_support_material: [
            recording("review-tl", "review", "tl"),
            recording("ballot-en", "ballot", "en"),
            recording("ballot-tl", "ballot", "tl"),
        ],
    }
    mockDocuments = {
        sequent_backend_document: [{id: "document-ballot-tl", name: "balota tl.mp3"}],
    }
    at(BALLOT)

    expect(screen.getByTestId("audio-instructions").dataset.recording).toBe(
        "https://files.invalid/tenant-north/document-document-ballot-tl/balota%20tl.mp3"
    )
    expect(mockQueries.find(({query}) => query === GET_DOCUMENT)?.variables).toEqual({
        ids: ["document-ballot-tl"],
        electionEventId: "mayor",
        tenantId: "north",
    })
})

it("falls back to the default language's recording", () => {
    mockMaterials = {sequent_backend_support_material: [recording("ballot-en", "ballot", "en")]}
    mockDocuments = {sequent_backend_document: [{id: "document-ballot-en", name: "ballot.mp3"}]}
    at(BALLOT)
    expect(screen.getByTestId("audio-instructions").dataset.recording).toContain(
        "document-document-ballot-en/ballot.mp3"
    )
})

it("prefers the published ballot style's presentation over the event's", () => {
    mockBallotStyle = {
        ballot_eml: {election_event_presentation: presentation(EAudioInstructionsPolicy.RECORDED)},
    }
    at(BALLOT)
    expect(screen.getByTestId("audio-instructions").dataset.policy).toBe("recorded")
})

it("asks for no recording without a session", () => {
    mockDisableAuth = true
    at(BALLOT)
    expect(screen.getByTestId("audio-instructions").dataset.recording).toBe("")
    expect(mockQueries).toEqual([])
})

it.each([
    ["the policy is disabled", presentation(EAudioInstructionsPolicy.DISABLED), BALLOT],
    ["the event has no policy", presentation(undefined), BALLOT],
    ["the event has no presentation", undefined, BALLOT],
    [
        "the screen has no instructions",
        presentation(EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED),
        "/tenant/north/event/mayor/login",
    ],
])("renders nothing and fetches nothing when %s", (_case, eventPresentation, path) => {
    mockElectionEvent = {presentation: eventPresentation}
    const {container} = at(path)
    expect(container).toBeEmptyDOMElement()
    expect(mockQueries).toEqual([])
})
