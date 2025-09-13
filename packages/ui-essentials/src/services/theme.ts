// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {
    BreakpointsOptions,
    Components,
    SimplePaletteColorOptions,
    ThemeOptions,
    createTheme,
} from "@mui/material"
import {LinkProps} from "@mui/material/Link"
import LinkBehavior from "../components/LinkBehavior/LinkBehavior"
import {Theme as MUITheme} from "@mui/material"
import {TypographyOptions} from "@mui/material/styles/createTypography"

// Re-declare the emotion theme to have the properties of the MaterialUiTheme
// See: https://github.com/emotion-js/emotion/discussions/2291#discussioncomment-491185
declare module "@emotion/react" {
    export interface Theme extends MUITheme {}
}

declare module "@mui/material/Typography" {
    interface TypographyPropsVariantOverrides {
        black: true
    }
}

declare module "@mui/material/Paper" {
    interface PaperPropsVariantOverrides {
        dashed: true
        responsive: true
        fixed: true
        error: true
        success: true
        warning: true
        info: true
    }
}

declare module "@mui/material/Card" {
    interface PaperPropsVariantOverrides {
        responsive: true
    }
}

declare module "@mui/material/Button" {
    interface ButtonPropsVariantOverrides {
        secondary: true
        action: true
        warning: true
        cancel: true
        solidWarning: true
        softWarning: true
        actionbar: true
        listAction: true
    }
}
declare module "@mui/material/TextField" {
    interface TextFieldPropsVariantOverrides {
        writeIn: true
    }
}

declare module "@mui/material/styles" {
    export interface PaletteOptions {
        lightBackground: string
        brandColor: string
        brandSuccess: string
        errorColor: string
        red: SimplePaletteColorOptions
        green: SimplePaletteColorOptions
        customGreen: SimplePaletteColorOptions
        yellow: SimplePaletteColorOptions
        blue: SimplePaletteColorOptions
        customGrey: SimplePaletteColorOptions
        extraGrey: SimplePaletteColorOptions
        white: string
        black: string
    }
    export interface Palette extends PaletteOptions {}
}

const palette = {
    lightBackground: "#F7F9FE",
    brandColor: "#0F054C",
    secondary: {
        main: "#666666",
    },
    brandSuccess: "#43E3A1",
    errorColor: "#DC2626",
    red: {
        light: "#FECACA",
        main: "#991B1B",
        dark: "#991B1B",
    },
    green: {
        light: "#ECFDF5",
        main: "#064E3B",
        dark: "#047857",
    },
    customGreen: {
        light: "#CFF0DC",
        main: "#0EB048",
        dark: "#0EB048",
    },
    yellow: {
        light: "#FFF7D9",
        main: "#837032",
        dark: "#837032",
    },
    blue: {
        light: "#CCE5FF",
        main: "#292F99",
        dark: "#292F99",
    },
    customGrey: {
        light: "#E7EAEE",
        main: "#757575",
        dark: "#64748B",
        contrastText: "#191D23",
    },
    extraGrey: {
        main: "#B8C0CC",
    },
    white: "white",
    black: "black",
}

let breakpoints: BreakpointsOptions = {
    values: {
        xs: 0,
        sm: 750,
        md: 970,
        lg: 1170,
        xl: 1536,
    },
}

let MuiButton: Components["MuiButton"] = {
    styleOverrides: {
        root: {
            "padding": "6px 12px",
            "display": "flex",
            "flexDirection": "row",
            "gap": "4px",
            "fontSize": "16px",
            "textTransform": "unset",
            "backgroundColor": palette.brandColor,
            "border": `1px solid ${palette.brandColor}`,
            "color": palette.white,
            "minHeight": "44px",
            "&:hover": {
                backgroundColor: palette.brandColor,
                color: palette.white,
                boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
            },
            "&:active": {
                color: `${palette.brandColor} !important`,
                backgroundColor: `${palette.white} !important`,
            },
            "&:focus": {
                border: `2px solid ${palette.brandSuccess}`,
                color: palette.white,
                backgroundColor: palette.brandColor,
            },
            "&.Mui-disabled": {
                background: "rgba(15, 5, 76, 0.4)",
                border: "1px solid rgba(255, 255, 255, 0.4)",
                color: palette.white,
            },

            "borderRadius": "0",
            [`@media (min-width: ${breakpoints.values!.sm!}px)`]: {
                borderRadius: "4px",
            },
        },
    },
    variants: [
        {
            props: {variant: "secondary"},
            style: {
                "backgroundColor": palette.white,
                "border": `1px solid ${palette.brandColor}`,
                "color": palette.brandColor,
                "&:hover": {
                    backgroundColor: palette.white,
                    border: `1px solid ${palette.brandColor}`,
                    color: palette.brandColor,
                    boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
                },
                "&:active": {
                    backgroundColor: `${palette.brandColor} !important`,
                    border: `1px solid ${palette.brandColor}`,
                    color: `${palette.white} !important`,
                },
                "&:focus": {
                    border: `2px solid ${palette.brandSuccess}`,
                    backgroundColor: palette.white,
                    color: palette.brandColor,
                },
                "&.Mui-disabled": {
                    "background": "rgba(255, 255, 255, 0.4)",
                    "border": "1px solid rgba(15, 5, 76, 0.4)",
                    "color": palette.brandColor,
                    "*": {
                        opacity: 0.5,
                    },
                },
            },
        },
        {
            props: {variant: "listAction"},
            style: {
                "backgroundColor": palette.white,
                "border": `1px solid ${palette.brandColor}`,
                "color": palette.brandColor,
                "&:hover": {
                    color: palette.white,
                    border: `1px solid ${palette.brandColor}`,
                    backgroundColor: palette.brandColor,
                    boxShadow: "none",
                },
            },
        },
        {
            props: {variant: "action"},
            style: {
                "backgroundColor": palette.brandSuccess,
                "border": `1px solid ${palette.brandColor}`,
                "color": palette.brandColor,
                "&:hover": {
                    backgroundColor: palette.brandSuccess,
                    border: `1px solid ${palette.brandColor}`,
                    color: palette.brandColor,
                    boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
                },
                "&:active": {
                    backgroundColor: `${palette.brandColor} !important`,
                    border: `1px solid ${palette.brandSuccess}`,
                    color: `${palette.brandSuccess} !important`,
                },
                "&:focus": {
                    border: `2px solid ${palette.brandColor}`,
                    backgroundColor: palette.brandSuccess,
                    color: palette.brandColor,
                },
                "&.Mui-disabled": {
                    "background": "rgba(67, 227, 161, 0.4)",
                    "border": "1px solid rgba(15, 5, 76, 0.4)",
                    "color": palette.brandColor,
                    "*": {
                        opacity: 0.5,
                    },
                },
            },
        },
        {
            props: {variant: "warning"},
            style: {
                "backgroundColor": palette.white,
                "border": `1px solid ${palette.errorColor}`,
                "color": palette.errorColor,
                "&:hover": {
                    backgroundColor: palette.white,
                    border: `1px solid ${palette.errorColor}`,
                    color: palette.errorColor,
                    boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
                },
                "&:active": {
                    backgroundColor: `${palette.errorColor} !important`,
                    border: `1px solid ${palette.errorColor}`,
                    color: `${palette.white} !important`,
                },
                "&:focus": {
                    border: `2px solid ${palette.errorColor}`,
                    backgroundColor: palette.white,
                    color: palette.errorColor,
                },
                "&.Mui-disabled": {
                    "background": "rgba(255, 255, 255, 0.4)",
                    "border": "1px solid rgba(239, 68, 68, 0.4)",
                    "color": palette.errorColor,
                    "*": {
                        opacity: 0.5,
                    },
                },
            },
        },
        {
            props: {variant: "cancel"},
            style: {
                "backgroundColor": palette.customGrey.light,
                "border": `1px solid ${palette.customGrey.light}`,
                "color": palette.black,
                "&:hover": {
                    backgroundColor: palette.customGrey.light,
                    border: `1px solid ${palette.customGrey.light}`,
                    color: palette.black,
                    boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
                },
                "&:active": {
                    backgroundColor: `${palette.black} !important`,
                    border: `1px solid ${palette.black}`,
                    color: `${palette.customGrey.light} !important`,
                },
                "&:focus": {
                    border: `2px solid ${palette.black}`,
                    backgroundColor: palette.customGrey.light,
                    color: palette.black,
                },
                "&.Mui-disabled": {
                    "background": "rgba(231, 234, 238, 0.4)",
                    "border": "1px solid rgba(231, 234, 238, 0.4)",
                    "color": palette.black,
                    "*": {
                        opacity: 0.5,
                    },
                },
            },
        },
        {
            props: {variant: "solidWarning"},
            style: {
                "backgroundColor": palette.errorColor,
                "border": `1px solid ${palette.errorColor}`,
                "color": palette.white,
                "&:hover": {
                    backgroundColor: palette.errorColor,
                    border: `1px solid ${palette.errorColor}`,
                    color: palette.white,
                    boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
                },
                "&:active": {
                    backgroundColor: `${palette.white} !important`,
                    border: `1px solid ${palette.errorColor}`,
                    color: `${palette.errorColor} !important`,
                },
                "&:focus": {
                    border: `2px solid ${palette.brandColor}`,
                    backgroundColor: palette.errorColor,
                    color: palette.white,
                },
                "&.Mui-disabled": {
                    background: "rgba(239, 68, 68, 0.4)",
                    border: "1px solid rgba(239, 68, 68, 0.4)",
                    color: palette.white,
                },
            },
        },
        {
            props: {variant: "softWarning"},
            style: {
                "backgroundColor": palette.yellow.main,
                "border": `1px solid ${palette.yellow.main}`,
                "color": palette.white,
                "&:hover": {
                    backgroundColor: palette.yellow.dark,
                    border: `1px solid ${palette.yellow.dark}`,
                    color: palette.white,
                    boxShadow: "0px 4px 4px rgba(0, 0, 0, 0.25)",
                },
                "&:active": {
                    backgroundColor: `${palette.yellow.main} !important`,
                    border: `1px solid ${palette.yellow.main}`,
                    color: `${palette.white} !important`,
                },
                "&:focus": {
                    border: `2px solid ${palette.yellow.dark}`,
                    backgroundColor: palette.yellow.dark,
                    color: palette.white,
                },
                "&.Mui-disabled": {
                    background: "rgba(239, 68, 68, 0.4)",
                    border: "1px solid rgba(239, 68, 68, 0.4)",
                    color: palette.white,
                },
            },
        },
        {
            props: {variant: "actionbar"},
            style: {
                "backgroundColor": "transparent",
                "color": palette.brandColor,
                "border": `1px solid ${palette.brandColor}`,
                "borderRadius": "0",
                "fontWeight": "500",
                "fontSize": "14px",
                "fontStyle": "normal",
                "textTransform": "uppercase",
                "padding": "6px 12px",
                "height": "35px",
                "&:hover": {
                    "border": `1px solid ${palette.brandColor}`,
                    "color": palette.white,
                    "backgroundColor": palette.brandColor,
                    "*": {
                        filter: "none",
                    },
                    "boxShadow": "unset",
                },
                "&:active": {
                    backgroundColor: `${palette.brandColor} !important`,
                    border: `1px solid ${palette.brandColor}`,
                    color: `${palette.white} !important`,
                },
                "&:focus": {
                    border: `1px solid ${palette.brandColor}`,
                    backgroundColor: "transparent",
                    color: `${palette.brandColor}`,
                },
                "&.Mui-disabled": {
                    background: "rgba(255, 255, 255, 0.4)",
                    border: "none",
                    color: palette.brandColor,
                    opacity: 0.5,
                },
            },
        },
    ],
}

let AdminMuiButton: Components["MuiButton"] = {
    styleOverrides: {
        root: {
            ...((MuiButton.styleOverrides?.root as {}) || {}),
            minWidth: "unset",
        },
    },
    variants: MuiButton.variants,
}

let MuiLink: Components["MuiLink"] = {
    defaultProps: {
        component: LinkBehavior,
    } as LinkProps,
    variants: [
        {
            props: {variant: "black"},
            style: {
                color: "black",
                fontWeight: "bold",
                textDecorationColor: "black",
            },
        },
    ],
    styleOverrides: {
        root: {
            "fontSize": "0.875rem",
            "textDecoration": "none",
            "&:hover": {
                textDecoration: "underline",
            },
        },
    },
}

let MuiButtonBase: Components["MuiButtonBase"] = {
    defaultProps: {
        LinkComponent: LinkBehavior,
    },
}

let MuiMenu: Components["MuiMenu"] = {
    defaultProps: {
        PaperProps: {
            style: {
                minWidth: "120px",
            },
        },
    },
}

let MuiMenuItem: Components["MuiMenuItem"] = {
    defaultProps: {
        style: {
            marginBlockStart: 0,
            marginBlockEnd: 0,
        },
    },
}

let MuiPaper: Components["MuiPaper"] = {
    variants: [
        {
            props: {variant: "dashed"},
            style: {
                border: `2px dashed ${palette.brandSuccess}77`,
                padding: "0 10px",
            },
        },
        {
            props: {variant: "responsive"},
            style: {
                gap: "20px",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                borderRadius: "unset",
            },
        },
        {
            props: {variant: "fixed"},
            style: {
                height: "16rem",
                width: "28rem",
                maxWidth: "100%",
                position: "relative",
                borderRadius: "unset",
            },
        },
        {
            props: {variant: "error"},
            style: {
                backgroundColor: palette.red.light,
                color: palette.red.main,
            },
        },
        {
            props: {variant: "success"},
            style: {
                backgroundColor: palette.green.light,
                color: palette.green.main,
            },
        },
        {
            props: {variant: "warning"},
            style: {
                backgroundColor: palette.yellow.light,
                color: palette.yellow.main,
            },
        },
        {
            props: {variant: "info"},
            style: {
                backgroundColor: palette.blue.light,
                color: palette.blue.main,
            },
        },
    ],
}

let MuiSkeleton: Components["MuiSkeleton"] = {
    variants: [
        {
            props: {variant: "text"},
            style: {
                lineHeight: 1.43,
                fontSize: "0.875rem",
                margin: "14px 0",
            },
        },
    ],
}

let MuiDialogTitle: Components["MuiDialogTitle"] = {
    styleOverrides: {
        root: {
            padding: "5px 10px",
            display: "flex",
            flexDirection: "row",
            gap: "11px",
            alignItems: "center",
            backgroundColor: palette.lightBackground,
            fontSize: "16px",
            color: palette.customGrey.contrastText,
        },
    },
}

let MuiDialogContent: Components["MuiDialogContent"] = {
    styleOverrides: {
        root: {
            padding: "16px 48px 0 48px !important",
        },
    },
}

let MuiDialogActions: Components["MuiDialogActions"] = {
    styleOverrides: {
        root: {
            padding: "15px 48px",
        },
    },
}

let MuiDialog: Components["MuiDialog"] = {
    styleOverrides: {
        paper: ({ownerState}) => {
            return {
                ...{
                    border: `2px solid ${palette.black}`,
                },
                ...(ownerState.maxWidth === "xs"
                    ? {
                          maxWidth: "496px",
                      }
                    : {}),
            }
        },
    },
}

let MuiIconButton: Components["MuiIconButton"] = {
    styleOverrides: {
        root: {
            "padding": 0,
            "border": `2px solid transparent`,
            "color": palette.black,
            "&:hover": {
                padding: 0,
                filter: "drop-shadow(0px 4px 4px rgba(0, 0, 0, 0.25))",
            },
            "&:active": {
                color: palette.customGrey.main,
                border: `2px solid ${palette.black}`,
                backgroundColor: palette.white,
            },
        },
    },
}

let MuiTextField: Components["MuiTextField"] = {
    styleOverrides: {
        root: {
            "margin": 0,
            "width": "100%",
            "& .MuiInputBase-input": {
                fontSize: "14px",
                padding: "6px 12px",
            },
        },
    },
}

let MuiCheckbox: Components["MuiCheckbox"] = {
    styleOverrides: {
        root: {
            "color": palette.extraGrey.main,
            "&:hover": {
                backgroundColor: "unset",
            },
            "& .MuiSvgIcon-root": {fontSize: 31},
            "&.Mui-checked": {
                color: palette.brandColor,
            },
        },
    },
}

let typography: TypographyOptions = {
    body1: {
        textAlign: "left",
        marginBlockStart: "1em",
        marginBlockEnd: "1em",
        marginInlineStart: "0px",
        marginInlineEnd: "0px",
        wordBreak: "keep-all",
        wordWrap: "break-word",
    },
    body2: {
        textAlign: "left",
        marginBlockStart: "1em",
        marginBlockEnd: "1em",
        marginInlineStart: "0px",
        marginInlineEnd: "0px",
        wordBreak: "keep-all",
        wordWrap: "break-word",
    },
    h4: {
        paddingTop: "32px",
        paddingBottom: "16px",
        textAlign: "center",
    },
}

export const themeOptions: ThemeOptions = {
    breakpoints,
    components: {
        MuiLink,
        MuiButtonBase,
        MuiButton,
        MuiMenu,
        MuiMenuItem,
        MuiPaper,
        MuiSkeleton,
        MuiDialogTitle,
        MuiDialogContent,
        MuiDialogActions,
        MuiDialog,
        MuiIconButton,
        MuiTextField,
        MuiCheckbox,
    },
    typography,
    palette,
}

export const adminThemeOptions = {
    breakpoints,
    components: {
        MuiLink,
        MuiButtonBase,
        MuiButton: AdminMuiButton,
        MuiMenu,
        MuiMenuItem,
        MuiPaper,
        MuiSkeleton,
        MuiDialogTitle,
        MuiDialogContent,
        MuiDialogActions,
        MuiDialog,
        MuiIconButton,
        MuiTextField,
        MuiCheckbox,
    },
    typography,
    palette,
}

export const theme = createTheme(themeOptions)

export const adminTheme = createTheme(adminThemeOptions)

export default theme
