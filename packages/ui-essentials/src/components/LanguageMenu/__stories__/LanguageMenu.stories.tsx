// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {ComponentStory, ComponentMeta} from "@storybook/react"
import LanguageMenu from "../LanguageMenu"
import Box from "@mui/material/Box"
import {within, userEvent} from "@storybook/testing-library"

export default {
    title: "components/LanguageMenu",
    component: LanguageMenu,
    parameters: {
        backgrounds: {
            default: "light",
        },
    },
    argTypes: {},
} as ComponentMeta<typeof LanguageMenu>

const Template: ComponentStory<typeof LanguageMenu> = (args) => (
    <Box style={{display: "inline-flex", backgroundColor: "white"}}>
        <LanguageMenu {...args} />
    </Box>
)

export const CollapsedMenu = Template.bind({})
// More on args: https://storybook.js.org/docs/react/writing-stories/args
CollapsedMenu.args = {
    label: "LanguageMenu",
}

export const ExpandedMenu = Template.bind({})
// More on args: https://storybook.js.org/docs/react/writing-stories/args
ExpandedMenu.args = {
    label: "LanguageMenu",
}

ExpandedMenu.play = async ({canvasElement}) => {
    // Starts querying the component from its root element
    const canvas = within(canvasElement)

    // See https://storybook.js.org/docs/react/essentials/actions#automatically-matching-args to learn how to setup logging in the Actions panel
    await userEvent.click(canvas.getByTestId("lang-button-test"))
}
