// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import DropFile from "../DropFile"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const meta: Meta<typeof DropFile> = {
    title: "components/DropFile",
    component: DropFile,
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

type Story = StoryObj<typeof DropFile>

const handleFiles = (files: FileList) => {
    alert("Number of files: " + files.length)
}

export const Primary: Story = {
    args: {
        handleFiles,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
