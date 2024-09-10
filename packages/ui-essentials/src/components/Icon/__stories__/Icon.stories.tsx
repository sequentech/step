// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {faExclamationTriangle} from "@fortawesome/free-solid-svg-icons"
import Icon from "../Icon"
import VerticalBox from "../../VerticalBox/VerticalBox"
import { IconProp } from "@fortawesome/fontawesome-svg-core"

const IconExample: React.FC = () => (
    <VerticalBox maxWidth="32px">
        <Icon icon={faExclamationTriangle as IconProp} size="lg" />
        <Icon icon={faExclamationTriangle as IconProp} variant="info" size="lg" />
        <Icon icon={faExclamationTriangle as IconProp} variant="warning" size="lg" />
        <Icon icon={faExclamationTriangle as IconProp} variant="error" size="lg" />
        <Icon icon={faExclamationTriangle as IconProp} variant="success" size="lg" />
        <Icon icon={faExclamationTriangle as IconProp} variant="form" size="lg" />
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
