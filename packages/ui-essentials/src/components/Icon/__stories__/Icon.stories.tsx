// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {faExclamationTriangle} from "@fortawesome/free-solid-svg-icons"
import Icon from "../Icon"
import VerticalBox from "../../VerticalBox/VerticalBox"

const IconExample: React.FC = () => (
    <VerticalBox maxWidth="32px">
        <Icon icon={faExclamationTriangle} size="lg" />
        <Icon icon={faExclamationTriangle} variant="info" size="lg" />
        <Icon icon={faExclamationTriangle} variant="warning" size="lg" />
        <Icon icon={faExclamationTriangle} variant="error" size="lg" />
        <Icon icon={faExclamationTriangle} variant="success" size="lg" />
        <Icon icon={faExclamationTriangle} variant="form" size="lg" />
    </VerticalBox>
)

const meta: Meta<typeof IconExample> = {
    title: "components/Icon",
    component: IconExample,
    parameters: {
        backgrounds: {
            default: "white",
        },
    },
}

export default meta

type Story = StoryObj<typeof IconExample>

export const Primary: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {},
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
