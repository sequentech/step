// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {faTimesCircle} from "@fortawesome/free-solid-svg-icons"
import IconButton from "../IconButton"
import VerticalBox from "../../VerticalBox/VerticalBox"
import {IconProp} from "@fortawesome/fontawesome-svg-core"

const IconButtonExample: React.FC = () => (
    <VerticalBox maxWidth="32px">
        <IconButton icon={faTimesCircle as IconProp} />
        <IconButton icon={faTimesCircle as IconProp} variant="info" />
        <IconButton icon={faTimesCircle as IconProp} variant="warning" />
        <IconButton icon={faTimesCircle as IconProp} variant="error" />
        <IconButton icon={faTimesCircle as IconProp} variant="success" />
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
