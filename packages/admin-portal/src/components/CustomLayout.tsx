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
                position: "absolute",
                top: 0,
                right: 0,
                left: 0,
                bottom: 0,
                overflow: "auto"
            },
            "& .MuiPaper-root": {
                //boxShadow: "unset",
                top: "0px",
                position: "sticky",
                zIndex: 100,
            },
            "& .MuiToolbar-root": {
                minHeight: "unset",
            },
        }}
        appBar={CustomAppBar}
        menu={CustomMenu}
    />
)
