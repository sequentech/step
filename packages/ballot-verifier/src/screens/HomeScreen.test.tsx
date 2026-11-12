// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, fireEvent, waitFor, act} from "@testing-library/react"
import {Provider} from "react-redux"
import {MemoryRouter} from "react-router-dom"
import {MockedProvider} from "@apollo/client/testing"
import {IAuditableBallot, IDecodedVoteContest} from "@sequentech/ui-core"
import {HomeScreen} from "./HomeScreen"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {store} from "../store/store"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"

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

const publishedStyle = {
    id: "style-1",
    election_id: "election-1",
    area_id: "area-1",
    public_key: {public_key: "key-1", is_demo: false},
    contests: [{id: "contest-1", candidates: [{id: "cand-1"}, {id: "cand-2"}]}],
}

const auditableBallotWith = (
    version: number,
    config: Record<string, unknown> = publishedStyle
): IAuditableBallot =>
    ({
        version,
        issue_date: "2026-09-02",
        config,
        contests: ["contest-1"],
        ballot_hash: "hash",
    }) as unknown as IAuditableBallot

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
    result: {
        data: {
            sequent_backend_ballot_publication: [
                {
                    __typename: "sequent_backend_ballot_publication",
                    id: "publication-1",
                    published_at: "2026-09-01T00:00:00+00:00",
                },
            ],
            sequent_backend_ballot_style: [
                {
                    __typename: "sequent_backend_ballot_style",
                    id: "style-1",
                    ballot_publication_id: "publication-1",
                    election_id: "election-1",
                    election_event_id: "event-1",
                    status: null,
                    tenant_id: "tenant-1",
                    ballot_eml: JSON.stringify(publishedStyle),
                    ballot_signature: null,
                    created_at: "2026-09-01T00:00:00+00:00",
                    area_id: "area-1",
                    annotations: null,
                    labels: null,
                    last_updated_at: "2026-09-01T00:00:00+00:00",
                    deleted_at: null,
                },
            ],
        },
    },
}

const uploadBallot = async (
    service: IBallotService,
    auditableBallot: IAuditableBallot = auditableBallotWith(3)
) => {
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
    await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0))
    })
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

    it("rejects a ballot whose ballot style has another public key", async () => {
        const service = ballotService(true)
        const forgedStyle = {...publishedStyle, public_key: {public_key: "forged", is_demo: false}}

        const setConfirmationBallot = await uploadBallot(
            service,
            auditableBallotWith(3, forgedStyle)
        )

        await waitFor(() => expect(screen.getByTestId("style-error")).toBeVisible())
        expect(screen.getByTestId("ciphertext-error")).not.toBeVisible()
        expect(setConfirmationBallot).toHaveBeenLastCalledWith(null)
        expect(service.hashBallot512).not.toHaveBeenCalled()
    })

    it("rejects a ballot whose ballot style is not published", async () => {
        const service = ballotService(true)

        const setConfirmationBallot = await uploadBallot(
            service,
            auditableBallotWith(3, {...publishedStyle, id: "style-2"})
        )

        await waitFor(() => expect(screen.getByTestId("style-error")).toBeVisible())
        expect(setConfirmationBallot).toHaveBeenLastCalledWith(null)
    })

    it("accepts a legacy ballot with a warning", async () => {
        const service = ballotService(true)

        const setConfirmationBallot = await uploadBallot(service, auditableBallotWith(2))

        await waitFor(() => expect(screen.getByTestId("legacy-ballot-warning")).toBeVisible())
        expect(screen.getByTestId("style-error")).not.toBeVisible()
        expect(setConfirmationBallot).toHaveBeenLastCalledWith(
            expect.objectContaining<Partial<IConfirmationBallot>>({ballot_hash: "hash"})
        )
    })
})
