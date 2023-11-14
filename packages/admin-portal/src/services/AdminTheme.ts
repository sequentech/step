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
        ...mixedAdminTheme.components,
        MuiTextField: {
            styleOverrides: {
                root: {
                    "margin": 0,
                    "width": "100%",
                    "& .MuiInputBase-input": {
                        fontSize: "16px",
                        padding: "4px 12px",
                    },
                },
            },
        },
        MuiInputBase: {
            styleOverrides: {
                root: {
                    //marginTop: "4px",
                    //marginBottom: 0,
                    "& #tenant-select": {
                        fontSize: "16px",
                        padding: "4px 12px",
                    },
                },
            },
        },
        RaSidebar: {
            styleOverrides: {
                root: {
                    //"boxShadow":
                    //    "0px 2px 1px -1px rgba(0, 0, 0, 0.20), 0px 1px 1px 0px rgba(0, 0, 0, 0.14), 0px 1px 3px 0px rgba(0, 0, 0, 0.12)",
                    "paddingRight": "4px",
                    "paddingLeft": "4px",
                    "height": "calc(100vh - 79px)",
                    "& .RaMenu-open": {
                        overflow: "clip",
                        flexGrow: 2,
                        paddingBotton: 0,
                        marginBottom: "4px",
                    },
                    "& .RaMenu-closed": {
                        overflow: "clip",
                        flexGrow: 2,
                        paddingBotton: 0,
                        marginBottom: "4px",
                    },
                    "& .RaSidebar-fixed": {
                        display: "flex",
                        flexDirection: "column",
                        height: "calc(100vh - 79px)",
                    },
                },
            },
        },
        RaLayout: {
            styleOverrides: {
                root: {
                    "& .RaLayout-content > .list-page > .RaList-main": {
                        overflow: "auto",
                        marginRight: "4px",
                    },
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
                },
            },
        },
    },
}
