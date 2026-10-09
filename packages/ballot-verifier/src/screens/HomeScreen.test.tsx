// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, fireEvent, waitFor} from "@testing-library/react"
import {Provider} from "react-redux"
import {MemoryRouter} from "react-router-dom"
import {MockedProvider} from "@apollo/client/testing"
import {IAuditableBallot, IBallotStyle, IDecodedVoteContest} from "@sequentech/ui-core"
import {HomeScreen} from "./HomeScreen"
import {IBallotService, IConfirmationBallot} from "../services/BallotService"
import {store} from "../store/store"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"

jest.mock("@sequentech/ui-core", () => ({
    isString: (value: unknown) => typeof value === "string",
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("..", () => {
    const React = require("react")
    return {TenantEventContext: React.createContext({tenantId: null, eventId: null})}
})
jest.mock("../services/BallotStyles", () => ({
    ...jest.requireActual("../services/BallotStyles"),
    updateBallotStyleAndSelection: jest.fn(),
}))
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

const publishedBallotStyle = {
    id: "ballot-style-1",
    election_id: "election-1",
    public_key: {public_key: "published-key", is_demo: false},
    contests: [],
    election_event_presentation: {css: ".published {}"},
}

const auditableBallot = {
    version: 1,
    issue_date: "2026-09-02",
    config: {
        ...publishedBallotStyle,
        public_key: {public_key: "other-key", is_demo: false},
        election_event_presentation: {css: ".from-file {}"},
    },
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
    result: {
        data: {
            sequent_backend_ballot_publication: [
                {id: "publication-1", published_at: "2026-09-01T00:00:00Z"},
            ],
            sequent_backend_ballot_style: [
                {
                    id: publishedBallotStyle.id,
                    ballot_publication_id: "publication-1",
                    election_id: publishedBallotStyle.election_id,
                    election_event_id: "event-1",
                    status: null,
                    tenant_id: "tenant-1",
                    ballot_eml: JSON.stringify(publishedBallotStyle),
                    ballot_signature: null,
                    created_at: "2026-09-01T00:00:00Z",
                    area_id: null,
                    annotations: null,
                    labels: null,
                    last_updated_at: "2026-09-01T00:00:00Z",
                    deleted_at: null,
                },
            ],
        },
    },
}

const uploadBallot = async (
    service: IBallotService,
    ballot: IAuditableBallot = auditableBallot
) => {
    const setConfirmationBallot = jest.fn()
    render(
        <Provider store={store}>
            <MockedProvider mocks={[ballotStylesMock]} addTypename={false}>
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
    await waitFor(() => expect(updateBallotStyleAndSelection).toHaveBeenCalled())
    const file = new File([JSON.stringify(ballot)], "ballot.json", {
        type: "application/json",
    })
    file.text = () => Promise.resolve(JSON.stringify(ballot))
    fireEvent.change(screen.getByTestId("drop-input-file"), {target: {files: [file]}})
    return setConfirmationBallot
}

describe("HomeScreen ballot verification", () => {
    afterEach(() => jest.clearAllMocks())

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

    it("decodes, checks and shows the ballot with the published ballot style it names", async () => {
        const service = ballotService(true)

        const setConfirmationBallot = await uploadBallot(service)

        await waitFor(() =>
            expect(setConfirmationBallot).toHaveBeenLastCalledWith(
                expect.objectContaining<Partial<IConfirmationBallot>>({
                    election_config: publishedBallotStyle as unknown as IBallotStyle,
                })
            )
        )
        expect(service.decodeAuditableBallot).toHaveBeenCalledWith(
            expect.objectContaining({config: publishedBallotStyle})
        )
        expect(service.verifyAuditableBallotCiphertext).toHaveBeenCalledWith(
            expect.objectContaining({config: publishedBallotStyle})
        )
    })

    it("rejects a ballot whose ballot style is not published", async () => {
        const service = ballotService(true)
        const unpublished = {
            ...auditableBallot,
            config: {...auditableBallot.config, id: "unpublished-ballot-style"},
        }

        const setConfirmationBallot = await uploadBallot(service, unpublished)

        await waitFor(() => expect(screen.getByTestId("unpublished-style-error")).toBeVisible())
        expect(screen.getByTestId("import-error")).not.toBeVisible()
        expect(screen.getByTestId("ciphertext-error")).not.toBeVisible()
        expect(setConfirmationBallot).toHaveBeenLastCalledWith(null)
        expect(service.decodeAuditableBallot).not.toHaveBeenCalled()
        expect(service.verifyAuditableBallotCiphertext).not.toHaveBeenCalled()
    })
})
