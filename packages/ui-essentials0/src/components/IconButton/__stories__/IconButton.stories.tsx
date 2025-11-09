// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {faTimesCircle} from "@fortawesome/free-solid-svg-icons"
import IconButton from "../IconButton"
import VerticalBox from "../../VerticalBox/VerticalBox"

const IconButtonExample: React.FC = () => (
    <VerticalBox maxWidth="32px">
        <IconButton icon={faTimesCircle} />
        <IconButton icon={faTimesCircle} variant="info" />
        <IconButton icon={faTimesCircle} variant="warning" />
        <IconButton icon={faTimesCircle} variant="error" />
        <IconButton icon={faTimesCircle} variant="success" />
    </VerticalBox>
)

const meta: Meta<typeof IconButtonExample> = {
    title: "components/IconButton",
    component: IconButtonExample,
    parameters: {
        backgrounds: {
            default: "white",
        },
    },
}

export default meta

type Story = StoryObj<typeof IconButtonExample>

export const Primary: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {},
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
