// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {MockedProvider} from "@apollo/client/testing"
import {configureStore} from "@reduxjs/toolkit"
import {Provider} from "react-redux"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import "../services/i18n"
import {HomeScreen} from "./HomeScreen"
import {IConfirmationBallot, provideBallotService} from "../services/BallotService"
import {TenantEventProvider} from "../providers/TenantEventContext"
import ballotStyles from "../store/ballotStyles/ballotStylesSlice"
import {GET_BALLOT_STYLES} from "../queries/GetBallotStyles"
import {resetSequentCore, sequentCore} from "../__mocks__/sequentCore"
import {
    IDS,
    multiContestBallot,
    recordSequentCore,
    RecordedBallot,
    sampleBallot,
    singleContestBallot,
    withModifiedSignature,
} from "../__mocks__/auditableBallots"

const eventPath = `/tenant/${IDS.tenant}/event/${IDS.event}`
const noBallotStyles = {
    request: {query: GET_BALLOT_STYLES},
    result: {data: {sequent_backend_ballot_publication: [], sequent_backend_ballot_style: []}},
}

/** Holds the import state the way App does, and reports every verified ballot. */
function renderImportStep() {
    const reported = jest.fn<void, [IConfirmationBallot | null]>()
    const ImportStep = () => {
        const [confirmationBallot, setConfirmationBallot] = useState<IConfirmationBallot | null>(
            null
        )
        const [ballotId, setBallotId] = useState("")
        const [fileName, setFileName] = useState("")
        return (
            <HomeScreen
                confirmationBallot={confirmationBallot}
                setConfirmationBallot={(ballot) => {
                    reported(ballot)
                    setConfirmationBallot(ballot)
                }}
                ballotId={ballotId}
                setBallotId={setBallotId}
                fileName={fileName}
                setFileName={setFileName}
                ballotService={provideBallotService()}
            />
        )
    }
    render(
        <ThemeProvider theme={theme}>
            <Provider store={configureStore({reducer: {ballotStyles}})}>
                <MockedProvider mocks={[noBallotStyles]} addTypename={false}>
                    <MemoryRouter initialEntries={[`${eventPath}/start`]}>
                        <TenantEventProvider tenantId={IDS.tenant} eventId={IDS.event}>
                            <Routes>
                                <Route path={`${eventPath}/start`} element={<ImportStep />} />
                                <Route
                                    path={`${eventPath}/confirmation`}
                                    element={<p>Confirmation step</p>}
                                />
                            </Routes>
                        </TenantEventProvider>
                    </MemoryRouter>
                </MockedProvider>
            </Provider>
        </ThemeProvider>
    )
    return {reported, lastReported: () => reported.mock.lastCall?.[0]}
}

const ballotIdField = () => screen.getByRole("textbox", {name: "Ballot ID"})
const nextButton = () => screen.getByRole("button", {name: "Next"})
const importError = () => screen.queryByRole("alert")

async function importFile(contents: string, name = "audited-ballot.json") {
    const dropZone = screen.getByTestId("drop-label-file")
    userEvent.upload(
        screen.getByTestId("drop-input-file"),
        new File([contents], name, {type: "application/json"})
    )
    // The drop zone stays busy until the verifier has finished with the file.
    await waitFor(() => expect(dropZone).toHaveAttribute("aria-busy", "false"))
}

const expectAccepted = async (fileName: string) => {
    expect(await screen.findByText("Uploaded", {exact: true})).toBeVisible()
    expect(screen.getAllByText(fileName)).not.toHaveLength(0)
    expect(importError()).not.toBeInTheDocument()
}

const expectRejected = async () => {
    const alert = await screen.findByRole("alert")
    expect(alert).toHaveTextContent("Error")
    expect(alert).toHaveTextContent(
        "There was a problem importing the auditable ballot. Did you choose the right file?"
    )
    expect(screen.queryByText("Uploaded", {exact: true})).not.toBeInTheDocument()
}

const confirmationOf = ({ballot, ballotId, decoded}: RecordedBallot) => ({
    ballot_hash: ballotId,
    election_config: ballot.config,
    decoded_questions: decoded,
})

beforeEach(() => {
    resetSequentCore()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "info").mockImplementation(() => undefined)
})
afterEach(() => jest.restoreAllMocks())

describe("importing an audited ballot", () => {
    it("verifies a signed single-contest ballot and continues once its ballot ID is entered", async () => {
        const single = singleContestBallot()
        recordSequentCore(single)
        const {lastReported} = renderImportStep()
        expect(nextButton()).toBeDisabled()

        await importFile(JSON.stringify(single.ballot))

        await expectAccepted("audited-ballot.json")
        expect(lastReported()).toEqual(confirmationOf(single))
        // A verified file alone is not enough: the voter must supply the ballot ID.
        expect(nextButton()).toBeDisabled()
        userEvent.type(ballotIdField(), single.ballotId)
        expect(nextButton()).toBeEnabled()
        userEvent.click(nextButton())
        expect(await screen.findByText("Confirmation step")).toBeVisible()
    })

    it("falls back to the multi-contest format and hashes the ballot in that format", async () => {
        const multi = multiContestBallot()
        recordSequentCore(multi)
        const {lastReported} = renderImportStep()

        await importFile(JSON.stringify(multi.ballot))

        await expectAccepted("audited-ballot.json")
        expect(lastReported()).toEqual(confirmationOf(multi))
        userEvent.type(ballotIdField(), multi.ballotId)
        expect(nextButton()).toBeEnabled()
    })

    // sequent-core accepts a ballot with neither signature field as unsigned (ballot.rs).
    it.each([
        ["absent", undefined],
        ["null", null],
    ])("accepts a ballot whose voter signature fields are %s", async (_form, missing) => {
        const single = singleContestBallot()
        const unsigned = {
            ...single.ballot,
            voter_signing_pk: missing,
            voter_ballot_signature: missing,
        }
        recordSequentCore(single)
        const {lastReported} = renderImportStep()

        await importFile(JSON.stringify(unsigned))

        await expectAccepted("audited-ballot.json")
        expect(lastReported()).toEqual(confirmationOf(single))
    })

    it.each([
        ["single-contest", singleContestBallot],
        ["multi-contest", multiContestBallot],
    ])(
        "rejects a %s ballot whose signature was modified, then accepts the original again",
        async (_format, recorded) => {
            const valid = recorded()
            recordSequentCore(valid)
            const {lastReported} = renderImportStep()
            await importFile(JSON.stringify(valid.ballot))
            await expectAccepted("audited-ballot.json")
            userEvent.type(ballotIdField(), valid.ballotId)

            await importFile(JSON.stringify(withModifiedSignature(valid.ballot)), "broken.json")

            await expectRejected()
            expect(lastReported()).toBeNull()
            expect(nextButton()).toBeDisabled()

            await importFile(JSON.stringify(valid.ballot))
            await expectAccepted("audited-ballot.json")
            expect(nextButton()).toBeEnabled()
        }
    )

    it.each([
        ["malformed JSON", "{invalid JSON"],
        ["a JSON document that is not an auditable ballot", JSON.stringify({ballot: "none"})],
        [
            "a ballot without its ballot style",
            JSON.stringify({...singleContestBallot().ballot, config: null}),
        ],
    ])("rejects %s after a verified import", async (_case, contents) => {
        const single = singleContestBallot()
        recordSequentCore(single)
        const {lastReported} = renderImportStep()
        await importFile(JSON.stringify(single.ballot))
        await expectAccepted("audited-ballot.json")
        userEvent.type(ballotIdField(), single.ballotId)

        await importFile(contents, "broken.json")

        await expectRejected()
        expect(lastReported()).toBeNull()
        expect(nextButton()).toBeDisabled()
    })

    // sequent-core rejects a signature without its public key, and the reverse
    // (INCOMPLETE_BALLOT_SIGNATURE_ERROR), in either ballot format.
    it.each([
        ["single-contest", "voter_signing_pk", singleContestBallot],
        ["single-contest", "voter_ballot_signature", singleContestBallot],
        ["multi-contest", "voter_signing_pk", multiContestBallot],
        ["multi-contest", "voter_ballot_signature", multiContestBallot],
    ] as const)(
        "rejects a signed %s ballot whose %s was removed",
        async (_format, field, recorded) => {
            const valid = recorded()
            const {[field]: _removed, ...incomplete} = valid.ballot
            recordSequentCore(valid)
            const {lastReported} = renderImportStep()

            await importFile(JSON.stringify(incomplete), "incomplete.json")

            await expectRejected()
            expect(lastReported()).toBeNull()
            expect(nextButton()).toBeDisabled()
        }
    )
})

describe("the sample ballot", () => {
    it("imports the generated sample and fills in its ballot ID", async () => {
        const sample = sampleBallot()
        recordSequentCore(sample)
        sequentCore.generate_sample_auditable_ballot_js.mockReturnValue(sample.ballot)
        const {lastReported} = renderImportStep()

        userEvent.click(screen.getByRole("button", {name: "Use a sample ballot"}))

        await expectAccepted("sample.json")
        expect(ballotIdField()).toHaveValue(sample.ballotId)
        expect(lastReported()).toEqual(confirmationOf(sample))
        expect(nextButton()).toBeEnabled()
        userEvent.clear(ballotIdField())
        expect(nextButton()).toBeDisabled()
    })

    it("leaves the form empty when no sample can be generated", async () => {
        sequentCore.generate_sample_auditable_ballot_js.mockImplementation(() => {
            throw "Error converting auditable ballot to json"
        })
        const {reported} = renderImportStep()

        userEvent.click(screen.getByRole("button", {name: "Use a sample ballot"}))

        expect(sequentCore.generate_sample_auditable_ballot_js).toHaveBeenCalled()
        expect(ballotIdField()).toHaveValue("")
        expect(screen.queryByText("Uploaded", {exact: true})).not.toBeInTheDocument()
        expect(reported).not.toHaveBeenCalled()
        expect(nextButton()).toBeDisabled()
    })
})

it("explains each step in a help dialog", async () => {
    renderImportStep()
    for (const [step, title] of [
        ["Step 1: Import your ballot", "Information: Import your ballot"],
        ["Step 2: Insert your ballot ID", "Information: Your ballot ID"],
    ]) {
        userEvent.click(
            within(screen.getByRole("heading", {name: new RegExp(step)})).getByRole("button")
        )
        const dialog = await screen.findByRole("dialog", {name: title})
        userEvent.click(within(dialog).getByRole("button", {name: "OK"}))
        await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    }
})
