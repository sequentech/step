// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Layout, LayoutProps} from "react-admin"
import {CustomAppBar} from "./CustomAppBar"
import {CustomMenu} from "./CustomMenu"

export const CustomLayout: React.FC<LayoutProps> = (props) => (
    <Layout
        {...props}
        sx={{
            "& .RaLayout-appFrame": {
                marginTop: 0,
            },
            "& .MuiPaper-root": {
                //boxShadow: "unset",
            },
        }}
        appBar={CustomAppBar}
        menu={CustomMenu}
    />
)
