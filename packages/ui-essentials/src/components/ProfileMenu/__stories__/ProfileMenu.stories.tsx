// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryFn, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {ProfileMenu} from "../ProfileMenu"
import {StyledButtonTooltipText} from "../../../components/Header/Header"
import theme from "../../../services/theme"
import {Box} from "@mui/material"

const meta: Meta<typeof ProfileMenu> = {
    title: "components/ProfileMenu",
    component: ProfileMenu,
    decorators: [
        (Story: StoryFn) => (
            <Box
                sx={{
                    width: "100%",
                    flexDirection: "row",
                    justifyContent: "flex-end",
                    display: "flex",
                    alignItems: "flex-end",
                }}
            >
                <Story />
            </Box>
        ),
    ],
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

type Story = StoryObj<typeof ProfileMenu>

export const Primary: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
        timeContent: function timeContent() {
            return (
                <>
                    <StyledButtonTooltipText
                        sx={{
                            fontWeight: 500,
                            color: theme.palette.brandColor,
                        }}
                    >
                        Sameple title
                    </StyledButtonTooltipText>
                    <StyledButtonTooltipText>Sample time left definition</StyledButtonTooltipText>
                </>
            )
        },
        userProfile: {
            email: "johnhasaverysupersuperduperverysuperduperlongname@sequentech.io",
            username: "John has a very super super duper very super duper long name",
            openLink() {
                alert("rouge")
            },
        },
        logoutFn() {
            alert("logging out")
        },
        setOpenModal: () => alert("open log out modal"),
        handleOpenTimeModal: () => alert("open time modal"),
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
