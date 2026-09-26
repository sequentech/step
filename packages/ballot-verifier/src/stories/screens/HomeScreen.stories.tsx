// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {MockedProvider} from "@apollo/client/testing"
import {Provider} from "react-redux"
import {configureStore} from "@reduxjs/toolkit"
import {expect, userEvent, within} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {HomeScreen} from "../../screens/HomeScreen"
import {IConfirmationBallot, provideBallotService} from "../../services/BallotService"
import ballotStyles from "../../store/ballotStyles/ballotStylesSlice"
import {GET_BALLOT_STYLES} from "../../queries/GetBallotStyles"

const HomeFixture = () => {
    const [store] = useState(() => configureStore({reducer: {ballotStyles}}))
    const [confirmationBallot, setConfirmationBallot] = useState<IConfirmationBallot | null>(null)
    const [ballotId, setBallotId] = useState("")
    const [fileName, setFileName] = useState("")
    return (
        <MockedProvider
            mocks={[
                {
                    request: {query: GET_BALLOT_STYLES},
                    result: {data: {sequent_backend_ballot_style: []}},
                },
            ]}
        >
            <Provider store={store}>
                <HomeScreen
                    confirmationBallot={confirmationBallot}
                    setConfirmationBallot={setConfirmationBallot}
                    ballotId={ballotId}
                    setBallotId={setBallotId}
                    fileName={fileName}
                    setFileName={setFileName}
                    ballotService={provideBallotService()}
                />
            </Provider>
        </MockedProvider>
    )
}
const meta = {
    title: "Screens/Verifier/Home screen",
    render: () => <HomeFixture />,
    loaders: [
        async () => {
            await initCore()
            return {}
        },
    ],
} satisfies Meta
export default meta
type Story = StoryObj<typeof meta>

export const Primary: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("button", {name: /Next/})).toBeDisabled()
    },
}
export const InvalidJson: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const file = new File(["{invalid JSON"], "ballot.json", {type: "application/json"})
        await userEvent.upload(canvas.getByTestId<HTMLInputElement>("drop-input-file"), file)
        await expect(await canvas.findByRole("alert")).toBeVisible()
        await expect(canvas.getByRole("button", {name: /Next/})).toBeDisabled()
    },
}

export const SampleBallot: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Use a sample ballot"}))
        await expect(
            canvas.getByRole<HTMLInputElement>("textbox", {name: "Ballot ID"}).value
        ).toMatch(/^[0-9a-f]{64}$/)
        await expect(canvas.getByText("Uploaded", {exact: true})).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled()
        await userEvent.clear(canvas.getByRole("textbox", {name: "Ballot ID"}))
        await expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
    },
}
