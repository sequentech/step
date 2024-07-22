// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Layout, LayoutProps, SidebarClasses} from "react-admin"
import {CustomAppBar} from "./CustomAppBar"
import {CustomMenu} from "./CustomMenu"
import {CustomSidebar} from "./menu/CustomSidebar"

const SequentSidebar = (props: any) => {
    return (
        <CustomSidebar {...props}>
            <CustomMenu {...props} classes={SidebarClasses} />
        </CustomSidebar>
    )
}

export const CustomLayout: React.FC<LayoutProps> = (props) => (
    <Layout
        {...props}
        sx={{
            "& .MuiPaper-root.RaSidebar-paper, & .MuiPaper-root.MuiAppBar-root": {
                top: "0",
                position: "sticky",
                zIndex: 100,
            },
            "& .MuiToolbar-root": {
                minHeight: "unset",
            },
            "& .RaList-main": {
                width: "50%",
            },
        }}
        appBar={CustomAppBar}
        sidebar={SequentSidebar}
    />
)
