// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import Paper from "@mui/material/Paper"
import {theme} from "../../../services/theme"

const PaperType: typeof Paper = ({...props}) => <Paper className="fff" {...props} />

const meta: Meta<typeof Paper> = {
    title: "material/Paper",
    component: PaperType,
    parameters: {
        backgrounds: {
            default: "light",
        },
    },
    argTypes: {},
}

export default meta

type Story = StoryObj<typeof Paper>

export const Fixed: Story = {
    args: {
        variant: "fixed",
        sx: {
            justifyContent: "center",
            alignItems: "center",
            display: "flex",
        },
        children: (
            <>
                <Paper
                    sx={{
                        width: "10rem",
                        height: "10rem",
                        backgroundColor: `${theme.palette.customGrey.light} !important`,
                    }}
                />
            </>
        ),
    },
}

export const Responsive: Story = {
    args: {
        variant: "responsive",
        sx: {
            justifyContent: "center",
            alignItems: "center",
            display: "flex",
        },
        children: (
            <>
                <Paper
                    sx={{
                        width: "10rem",
                        height: "10rem",
                        backgroundColor: `${theme.palette.customGrey.light} !important`,
                    }}
                />
            </>
        ),
    },
}

export const Dashed: Story = {
    args: {
        variant: "dashed",
        sx: {
            justifyContent: "center",
            alignItems: "center",
            display: "flex",
        },
        children: (
            <>
                <Paper
                    sx={{
                        width: "10rem",
                        height: "10rem",
                        backgroundColor: `${theme.palette.customGrey.light} !important`,
                    }}
                />
            </>
        ),
    },
}
