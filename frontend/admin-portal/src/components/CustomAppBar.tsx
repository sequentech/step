// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Header} from "@sequentech/ui-essentials"
import React, {useContext} from "react"
import {AppBar} from "react-admin"
import {AuthContext} from "../providers/AuthContextProvider"

export const CustomAppBar: React.FC = () => {
    const authContext = useContext(AuthContext)

    return (
        <AppBar
            position="static"
            sx={{
                "backgroundColor": "#F7F9FE",
                "color": "black",
                "& .MuiContainer-root.MuiContainer-maxWidthLg": {
                    maxWidth: "unset",
                },
                "marginBottom": "8px",
            }}
        >
            <Header logoutFn={authContext.isAuthenticated ? authContext.logout : undefined} />
        </AppBar>
    )
}
