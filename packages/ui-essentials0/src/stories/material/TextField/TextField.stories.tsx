// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import TextField from "@mui/material/TextField"

const TextFieldType: typeof TextField = ({...props}) => <TextField className="fff" {...props} />

const meta: Meta<typeof TextField> = {
    title: "material/TextField",
    component: TextFieldType,
    parameters: {
        backgrounds: {
            default: "light",
        },
    },
    argTypes: {},
}

export default meta

type Story = StoryObj<typeof TextField>

export const Fixed: Story = {
    args: {
        label: "Ballot ID",
        placeholder: "Type in your Ballot ID",
        InputLabelProps: {shrink: true},
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
