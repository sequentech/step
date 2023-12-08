// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Header} from "@sequentech/ui-essentials"
import React, {useContext} from "react"
import {AppBar} from "react-admin"
import {AuthContext} from "../providers/AuthContextProvider"
import {adminTheme} from "@sequentech/ui-essentials"
import globalSettings from "@/GlobalSettings"

export const CustomAppBar: React.FC = () => {
    const authContext = useContext(AuthContext)

    return (
        <AppBar
            toolbar={<></>}
            position="static"
            sx={{
                "backgroundColor": adminTheme.palette.lightBackground,
                "color": "black",
                "& .MuiContainer-root.MuiContainer-maxWidthLg": {
                    maxWidth: "unset",
                },
                "boxShadow": "unset",
            }}
        >
            <Header
                appVersion={{main: globalSettings.APP_VERSION}}
                userProfile={{
                    username: authContext.username,
                    email: authContext.email,
                    openLink: authContext.openProfileLink,
                }}
                logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
            />
        </AppBar>
    )
}
