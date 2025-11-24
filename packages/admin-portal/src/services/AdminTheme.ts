// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {defaultTheme} from "react-admin"
import {adminTheme} from "@sequentech/ui-essentials"

export const mixedAdminTheme = {
    ...defaultTheme,
    ...adminTheme,
}

let AdminMuiButton = {
    styleOverrides: adminTheme.components?.MuiButton?.styleOverrides,
    variants: [...(adminTheme.components?.MuiButton?.variants || [])],
}

export const fullAdminTheme = {
    ...mixedAdminTheme,
    sidebar: {
        width: 300,
        closedWidth: 50,
    },
    components: {
        ...mixedAdminTheme.components,
        MuiButton: {
            ...AdminMuiButton,
        },
        MuiToolbar: {
            styleOverride: {
                root: {
                    "&:last-child": {
                        borderRight: "1px solid #0F054C",
                    },
                },
            },
        },
        MuiTextField: {
            styleOverrides: {
                root: {
                    //"margin": 0,
                    "width": "100%",
                    "& .MuiInputBase-input": {
                        //fontSize: "16px",
                        //padding: "4px 12px",
                    },
                },
            },
        },
        MuiInputBase: {
            styleOverrides: {
                root: {
                    //marginTop: "4px",
                    //marginBottom: 0,
                    "& #tenant-oelect": {
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
                    "height": "100%",

                    "& .RaMenu-open": {
                        flexGrow: 2,
                        paddingBotton: 0,
                        marginBottom: "4px",
                    },
                    "& .RaMenu-closed": {
                        flexGrow: 2,
                        paddingBotton: 0,
                        marginBottom: "4px",
                    },
                    "& .RaSidebar-fixed": {
                        dioplay: "flex",
                        flexDirection: "column",
                        height: "100%",
                    },
                },
            },
        },
        RaAppBar: {
            styleOverrides: {
                root: {
                    "& .RaAppBar-menuButton": {
                        display: "none",
                    },
                },
            },
        },
        MuiDrawer: {
            styleOverrides: {
                root: {
                    "& .MuiDrawer-paper": {
                        width: "50%",
                    },
                    // doesn't work but it's a good try..
                    [mixedAdminTheme.breakpoints.down("xs") + "& .MuiDrawer-paper"]: {
                        width: "50%",
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
        MuiSwitch: {
            styleOverrides: {
                thumb: {},
                track: {},
                switchBase: {
                    "& + .MuiSwitch-track": {
                        backgroundColor: "rgba(0, 0, 0, 0.12)",
                    },
                    ".MuiSwitch-thumb": {
                        color: "#fff",
                    },
                    "&.Mui-checked": {
                        "& + .MuiSwitch-track": {
                            backgroundColor: "#0F054C",
                            opacity: 0.5,
                        },
                        ".MuiSwitch-thumb": {
                            color: "#0F054C",
                        },
                    },
                    "&.Mui-disabled + .MuiSwitch-track": {
                        opacity: 0.5,
                    },
                },
            },
        },
        MuiTabs: {
            styleOverrides: {
                indicator: {
                    backgroundColor: "#43E3A1",
                },
            },
        },
        MuiTab: {
            styleOverrides: {
                root: {
                    "textTransform": "uppercase",
                    "fontWeight": "500",
                    "fontSize": "14px",
                    "fontFamily": "Roboto",
                    "lineHeight": "24px",
                    "color": "#000",
                    "opacity": 0.4,
                    "letter": "0.4",
                    "cursor": "pointer",
                    "&:hover": {
                        opacity: 0.6,
                    },
                    "&.Mui-selected": {
                        color: "#0F054C",
                        opacity: 1,
                    },
                },
            },
        },
        MuiAccordion: {
            styleOverrides: {
                root: {
                    "&.MuiPaper-root": {
                        position: "relative !important",
                    },
                },
            },
        },
    },
}
