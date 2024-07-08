// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Header, {HeaderErrorVariant} from "../Header"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const meta: Meta<typeof Header> = {
    title: "components/Header",
    component: Header,
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

type Story = StoryObj<typeof Header>

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
    args: {
        logoutFn: () => {},
    },
    parameters: {
        viewport: {
            defaultViewport: "iphone6",
        },
    },
}

export const WithUserProfile: Story = {
    args: {
        userProfile: {
            email: "john@sequentech.io",
            username: "John Doe",
            openLink() {
                alert("rouge")
            },
        },
        logoutFn() {
            alert("logging out")
        },
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const WithUserProfileLong: Story = {
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
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const HiddenUserProfile: Story = {
    args: {
        userProfile: {
            email: "john@sequentech.io",
            username: "John Doe",
            openLink() {
                alert("rouge")
            },
        },
        logoutFn() {
            alert("logging out")
        },
        errorVariant: HeaderErrorVariant.HIDE_PROFILE,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
