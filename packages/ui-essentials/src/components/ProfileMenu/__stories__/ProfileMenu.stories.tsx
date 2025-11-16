// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryFn, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import {ProfileMenu, StyledButtonTooltipText} from "../ProfileMenu"
import {StyledButtonTooltip} from "../../../components/Header/Header"
import theme from "../../../services/theme"
import {Box} from "@mui/material"
import {EVotingPortalCountdownPolicy} from "@sequentech/ui-core"

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
                <>{Story({} as any, {} as any)}</>
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

export const CountdownWithAlert: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
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
        expiry: {
            endTime: new Date(Date.now() + 60000), //current time plus 2 minutes
            countdown: EVotingPortalCountdownPolicy.COUNTDOWN_WITH_ALERT,
            countdownAt: 60,
            alertAt: 30,
        },
        setTimeLeftDialogText: (v: string) => console.log({v}),
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const CountdownOnly: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
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
        expiry: {
            endTime: new Date(Date.now() + 30000), //current time plus 2 minutes
            countdown: EVotingPortalCountdownPolicy.COUNTDOWN,
            countdownAt: 30,
            alertAt: 10,
            duration: 60,
        },
        setTimeLeftDialogText: (v: string) => console.log({v}),
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const NoCountdown: Story = {
    // More on args: https://storybook.js.org/docs/react/writing-stories/args
    args: {
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
        setTimeLeftDialogText: (v: string) => console.log({v}),
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
