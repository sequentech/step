// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {defaultTheme} from "react-admin"
import {adminTheme} from "@sequentech/ui-essentials"

export const mixedAdminTheme = {
    ...defaultTheme,
    ...adminTheme,
}

export const fullAdminTheme = {
    ...mixedAdminTheme,
    sidebar: {
        width: 300,
        closedWidth: 50,
    },
    components: {
        ...(mixedAdminTheme.components),
        RaSidebar: {
            styleOverrides: {
                root: {
                    paddingRight: "4px",
                }
            }
        },
        RaLayout: {
            styleOverrides: {
                root: {
                    "& .RaLayout-appFrame": {
                        marginTop: 0,
                        position: "absolute",
                        top: 0,
                        right: 0,
                        left: 0,
                        bottom: 0,
                        overflow: "auto",
                        backgroundColor: adminTheme.palette.lightBackground,
                    },
                }
            }
        }
    }
}

