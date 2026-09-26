// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {
    ScenarioId,
    scenarioSnapshot,
    type JsonObject,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {AuthContext} from "../providers/AuthContextProvider"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {useAppSelector} from "../store/hooks"
import {store} from "../store/store"
import {setBallotSelectionVoteChoice} from "../store/ballotSelections/ballotSelectionsSlice"
import {PreviewScreen} from "./screens"
import type {PreviewSession} from "./session"
import {VoterPreview} from "./VoterPreview"

jest.mock("@sequentech/ui-essentials", () => ({
    Loader: () => <div role="progressbar" />,
    theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
}))
// Keycloak is never created in a preview; only its context is provided.
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({}),
}))
// The WASM gate and the encryption hook need sequent-core, which stories exercise.
jest.mock("../providers/WasmWrapper", () => ({
    WasmWrapper: ({children}: React.PropsWithChildren) => <>{children}</>,
}))
const mockEncrypt = jest.fn()
jest.mock("../hooks/useEncryptBallotForReview", () => ({
    useEncryptBallotForReview: () => ({encryptAndStoreBallot: mockEncrypt}),
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({isPreferential: () => false}),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))

function Probe() {
    const {globalSettings} = useContext(SettingsContext)
    const auth = useContext(AuthContext)
    const elections = useAppSelector((state) => Object.keys(state.elections))
    const alice = useAppSelector(
        (state) => state.ballotSelections[IDS.election]?.[0].choices[0].selected
    )
    return (
        <>
            <output aria-label="probe">
                {JSON.stringify({
                    disableAuth: globalSettings.DISABLE_AUTH,
                    kiosk: auth.isKiosk(),
                    elections,
                    alice,
                })}
            </output>
            <button onClick={() => auth.logout("https://example.org/finish")}>Log out</button>
        </>
    )
}

const probe = async () => JSON.parse((await screen.findByLabelText("probe")).textContent ?? "")

const session = (id: ScenarioId, screen = PreviewScreen.START): PreviewSession => ({
    snapshot: scenarioSnapshot(id),
    screen,
})

beforeEach(() => mockEncrypt.mockReset().mockReturnValue(true))

test("screens render once the snapshot is in the production store", async () => {
    render(
        <VoterPreview session={session(ScenarioId.SIMPLE_PLURALITY)}>
            <Probe />
        </VoterPreview>
    )
    expect(await probe()).toEqual({
        disableAuth: true,
        kiosk: false,
        elections: [IDS.election],
        alice: -1,
    })
    expect(store.getState().ballotStyles[IDS.election]?.area_id).toBe(IDS.area)
    expect(mockEncrypt).not.toHaveBeenCalled()
})

test("the snapshot channel decides whether the voter uses a kiosk", async () => {
    render(
        <VoterPreview session={session(ScenarioId.KIOSK_VOTER)}>
            <Probe />
        </VoterPreview>
    )
    expect(await probe()).toMatchObject({kiosk: true})
})

test("a new session reloads the voter state and the same session keeps it", async () => {
    const first = session(ScenarioId.SIMPLE_PLURALITY)
    const {rerender} = render(
        <VoterPreview session={first}>
            <Probe />
        </VoterPreview>
    )
    await probe()
    act(() => {
        const ballotStyle = store.getState().ballotStyles[IDS.election]!
        store.dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle,
                contestId: IDS.contest,
                voteChoice: {id: IDS.alice, selected: 0},
            })
        )
    })
    rerender(
        <VoterPreview session={first}>
            <Probe />
        </VoterPreview>
    )
    expect(await probe()).toMatchObject({alice: 0})
    rerender(
        <VoterPreview session={{...first}}>
            <Probe />
        </VoterPreview>
    )
    expect(await probe()).toMatchObject({alice: -1})
})

test("a review session is prepared with the encrypted sample ballot", async () => {
    render(
        <VoterPreview session={session(ScenarioId.SIMPLE_PLURALITY, PreviewScreen.REVIEW)}>
            <Probe />
        </VoterPreview>
    )
    await probe()
    expect(mockEncrypt).toHaveBeenCalledTimes(1)
    expect(store.getState().extra.isVoted).toEqual({[IDS.election]: true})
})

test("a snapshot the portal rejects is reported instead of the screens", async () => {
    const snapshot: ScenarioSnapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    const [contest] = snapshot.preview.ballot_styles[0].contests
    for (const candidate of contest.candidates as JsonObject[])
        candidate.presentation = {is_explicit_invalid: true}
    render(
        <VoterPreview session={{snapshot, screen: PreviewScreen.VOTE}}>
            <Probe />
        </VoterPreview>
    )
    expect((await screen.findByRole("alert")).textContent).toBe(
        "The portal could not load this snapshot" +
            "errors.configuration.multipleExplicitInvalidCandidates"
    )
    expect(screen.queryByLabelText("probe")).toBeNull()
})

test("a screen that cannot be prepared is reported", async () => {
    mockEncrypt.mockReturnValue(false)
    render(
        <VoterPreview session={session(ScenarioId.SIMPLE_PLURALITY, PreviewScreen.CONFIRMATION)}>
            <Probe />
        </VoterPreview>
    )
    expect(await screen.findByRole("alert")).toHaveTextContent(
        "The sample ballot for the confirmation screen could not be encrypted"
    )
})

test("the portal's logout calls reach the host", async () => {
    const onLogout = jest.fn()
    render(
        <VoterPreview session={session(ScenarioId.SIMPLE_PLURALITY)} onLogout={onLogout}>
            <Probe />
        </VoterPreview>
    )
    await probe()
    fireEvent.click(screen.getByRole("button", {name: "Log out"}))
    expect(onLogout).toHaveBeenCalledWith("https://example.org/finish")
})
