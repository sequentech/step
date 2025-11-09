// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import WarnBox from "../WarnBox"
import VerticalBox from "../../VerticalBox/VerticalBox"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const WarnBoxExample: React.FC = () => (
    <VerticalBox>
        <WarnBox variant="error" onClose={() => undefined}>
            <b>Question / Contest Title 2:</b> You have chosen more than the allowed selecitons on
            this contest/office.
        </WarnBox>
        <WarnBox variant="success" onClose={() => undefined}>
            <b>Question / Contest Title 2:</b> You have chosen more than the allowed selecitons on
            this contest/office.
        </WarnBox>
        <WarnBox variant="warning" onClose={() => undefined}>
            <b>Question / Contest Title 2:</b> You have chosen more than the allowed selecitons on
            this contest/office.
        </WarnBox>
        <WarnBox variant="info" onClose={() => undefined}>
            <b>Question / Contest Title 2:</b> You have chosen more than the allowed selecitons on
            this contest/office.
        </WarnBox>
    </VerticalBox>
)

const meta: Meta<typeof WarnBoxExample> = {
    title: "components/WarnBox",
    component: WarnBoxExample,
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

type Story = StoryObj<typeof WarnBoxExample>

export const Primary: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {},
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const PrimaryMobile: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {},
    parameters: {
        viewport: {
            defaultViewport: "iphone6",
        },
    },
}
