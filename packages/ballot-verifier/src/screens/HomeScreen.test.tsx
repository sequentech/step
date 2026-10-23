// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, fireEvent, waitFor} from "@testing-library/react"
import {Provider} from "react-redux"
import {MemoryRouter} from "react-router-dom"
import {MockedProvider} from "@apollo/client/testing"
import {IAuditableBallot, IDecodedVoteContest} from "@sequentech/ui-core"
import {HomeScreen} from "./HomeScreen"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {store} from "../store/store"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {IBallotStyle, setBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {TenantEventContext} from ".."

jest.mock("@sequentech/ui-core", () => ({}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("..", () => {
    const React = require("react")
    return {TenantEventContext: React.createContext({tenantId: null, eventId: null})}
})
jest.mock("../services/BallotStyles", () => ({updateBallotStyleAndSelection: jest.fn()}))
jest.mock("@sequentech/ui-essentials", () => {
    const React = require("react")
    const passthrough = ({children}: {children?: React.ReactNode}) =>
        React.createElement(React.Fragment, null, children)
    return {
        PageLimit: passthrough,
        BreadCrumbSteps: () => null,
        Icon: () => null,
        IconButton: passthrough,
        Dialog: passthrough,
        theme: {palette: {customGrey: {main: "#000"}}},
        DropFile: ({handleFiles}: {handleFiles: (files: FileList) => void}) =>
            React.createElement("input", {
                "type": "file",
                "data-testid": "drop-input-file",
                "onChange": (event: React.ChangeEvent<HTMLInputElement>) =>
                    event.target.files && handleFiles(event.target.files),
            }),
    }
})

const auditableBallot = {
    version: 1,
    issue_date: "2026-09-02",
    config: {election_id: "election-1", contests: []},
    contests: ["contest-1"],
    ballot_hash: "hash",
} as unknown as IAuditableBallot

const decodedContest = {contest_id: "contest-1"} as IDecodedVoteContest

const ballotService = (ciphertextConsistent: boolean | Error): IBallotService =>
    ({
        decodeAuditableBallot: jest.fn(() => [decodedContest]),
        decodeAuditableMultiBallot: jest.fn(() => null),
        hashBallot512: jest.fn(() => "hash"),
        hashMultiBallot: jest.fn(() => "hash"),
        verifyAuditableBallotCiphertext: jest.fn(() => {
            if (ciphertextConsistent instanceof Error) {
                throw ciphertextConsistent
            }
            return ciphertextConsistent
        }),
        verifyAuditableMultiBallotCiphertext: jest.fn(() => false),
        verifyBallotSignature: jest.fn(() => true),
        verifyMultiBallotSignature: jest.fn(() => true),
    }) as unknown as IBallotService

const ballotStylesMock = {
    request: {query: GET_BALLOT_STYLES},
    result: {data: {sequent_backend_ballot_publication: [], sequent_backend_ballot_style: []}},
}

const uploadBallot = async (service: IBallotService) => {
    const setConfirmationBallot = jest.fn()
    render(
        <Provider store={store}>
            <MockedProvider mocks={[ballotStylesMock]}>
                <MemoryRouter>
                    <HomeScreen
                        confirmationBallot={null}
                        setConfirmationBallot={setConfirmationBallot}
                        ballotId=""
                        setBallotId={jest.fn()}
                        fileName=""
                        setFileName={jest.fn()}
                        ballotService={service}
                    />
                </MemoryRouter>
            </MockedProvider>
        </Provider>
    )
    const file = new File([JSON.stringify(auditableBallot)], "ballot.json", {
        type: "application/json",
    })
    file.text = () => Promise.resolve(JSON.stringify(auditableBallot))
    fireEvent.change(screen.getByTestId("drop-input-file"), {target: {files: [file]}})
    await waitFor(() => expect(service.verifyAuditableBallotCiphertext).toHaveBeenCalled())
    return setConfirmationBallot
}

describe("HomeScreen ballot verification", () => {
    it("accepts a ballot whose plaintext and randomness reproduce its ciphertext", async () => {
        const service = ballotService(true)

        const setConfirmationBallot = await uploadBallot(service)

        await waitFor(() =>
            expect(setConfirmationBallot).toHaveBeenLastCalledWith(
                expect.objectContaining<Partial<IConfirmationBallot>>({
                    ballot_hash: "hash",
                    decoded_questions: [decodedContest],
                })
            )
        )
        expect(screen.getByTestId("ciphertext-error")).not.toBeVisible()
        expect(screen.getByTestId("import-error")).not.toBeVisible()
    })

    it("rejects a ballot whose ciphertext is not reproduced", async () => {
        const service = ballotService(false)

        const setConfirmationBallot = await uploadBallot(service)

        await waitFor(() => expect(screen.getByTestId("ciphertext-error")).toBeVisible())
        expect(screen.getByTestId("import-error")).not.toBeVisible()
        expect(setConfirmationBallot).toHaveBeenLastCalledWith(null)
        expect(service.hashBallot512).not.toHaveBeenCalled()
    })

    it("rejects a ballot that cannot be checked without calling it a mismatch", async () => {
        const service = ballotService(new Error("Error checking the ballot"))

        const setConfirmationBallot = await uploadBallot(service)

        await waitFor(() => expect(screen.getByTestId("import-error")).toBeVisible())
        expect(screen.getByTestId("ciphertext-error")).not.toBeVisible()
        expect(setConfirmationBallot).toHaveBeenLastCalledWith(null)
        expect(service.hashBallot512).not.toHaveBeenCalled()
    })
})

describe("HomeScreen receipt link", () => {
    /** The home screen of an event whose presentation has this transparency
     * mode. Each mode has its own event, as the store outlives the test. */
    const renderEvent = (transparency?: string) => {
        const eventId = `event-${transparency ?? "none"}`
        const at = "2026-10-01T00:00:00Z"
        store.dispatch(
            setBallotStyle({
                id: `style-${eventId}`,
                ballot_publication_id: "publication-1",
                publication_published_at: at,
                election_id: `election-${eventId}`,
                election_event_id: eventId,
                tenant_id: "tenant-1",
                ballot_eml: {
                    election_event_presentation: transparency ? {transparency} : {},
                } as unknown as IBallotStyle["ballot_eml"],
                created_at: at,
                last_updated_at: at,
            })
        )
        render(
            <Provider store={store}>
                <MockedProvider mocks={[ballotStylesMock]}>
                    <TenantEventContext.Provider value={{tenantId: "tenant-1", eventId}}>
                        <MemoryRouter>
                            <HomeScreen
                                confirmationBallot={null}
                                setConfirmationBallot={jest.fn()}
                                ballotId=""
                                setBallotId={jest.fn()}
                                fileName=""
                                setFileName={jest.fn()}
                                ballotService={ballotService(true)}
                            />
                        </MemoryRouter>
                    </TenantEventContext.Provider>
                </MockedProvider>
            </Provider>
        )
    }

    it.each(["public", "private"])(
        "links to the receipt screen when transparency is %s",
        (mode) => {
            renderEvent(mode)
            expect(screen.getByTestId("receipt-link")).toHaveAttribute("href", "/receipt")
        }
    )

    it.each([undefined, "disabled", "unknown"])(
        "has no receipt link when transparency is %s",
        (mode) => {
            renderEvent(mode)
            expect(screen.queryByTestId("receipt-link-section")).toBeNull()
        }
    )
})
