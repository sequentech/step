// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Meta, StoryObj} from "@storybook/react"
import BallotInput from "../BallotInput"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {MemoryRouter} from "react-router-dom"

const meta: Meta<typeof BallotInput> = {
    title: "components/BallotInput",
    component: BallotInput,
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}

export default meta

type Story = StoryObj<typeof BallotInput>

// Use a wrapper component to allow hooks and provide Router context
const BallotInputPrimaryStory = (args: any) => {
    const [value, setValue] = useState("")
    return (
        <MemoryRouter initialEntries={["/tenant/1/event/2/election-chooser"]}>
            <BallotInput
                {...args}
                title="Ballot ID"
                subTitle="Enter your ballot identifier"
                label="Ballot ID"
                error="Invalid ballot ID"
                placeholder="e.g. 1234abcd"
                value={value}
                doChange={(e) => setValue(e.target.value)}
                captureEnterAction={(e) => {
                    if (e.key === "Enter") alert("Enter pressed!")
                }}
                labelProps={{shrink: true}}
                helpText="Your ballot ID is provided on your voting card."
                dialogTitle="What is a Ballot ID?"
                dialogOk="OK"
                backButtonText="Back"
                ballotStyle={undefined}
            />
        </MemoryRouter>
    )
}

export const Primary: Story = {
    render: (args) => <BallotInputPrimaryStory {...args} />,
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
