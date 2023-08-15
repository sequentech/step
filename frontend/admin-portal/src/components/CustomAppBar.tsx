// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { Header } from "@sequentech/ui-essentials"
import React from "react"
import { AppBar } from "react-admin"

export const CustomAppBar: React.FC = () => (
    <AppBar
        position="static"
        sx={{
            "backgroundColor": "#F7F9FE",
            "color": "black",
            "& .MuiContainer-root.MuiContainer-maxWidthLg": {
                maxWidth: "unset",
            },
        }}
    >
        <Header />
    </AppBar>
)