// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {createTheme} from "@mui/material/styles"

// Scoped to the two React authentication pages; inherited FreeMarker pages keep
// their own styles. Fonts are bundled from the existing portal assets.
export const authTheme = createTheme({
    palette: {
        primary: {main: "#0d796a", dark: "#086455", contrastText: "#ffffff"},
        text: {primary: "#173449", secondary: "#5b6e7b"},
        background: {default: "#ffffff", paper: "#ffffff"},
        error: {main: "#a12b2b"},
    },
    typography: {
        fontFamily: '"Avenir Next", "Sequent Sans", "Segoe UI", system-ui, sans-serif',
        button: {fontWeight: 700, textTransform: "none"},
    },
    shape: {borderRadius: 10},
    components: {
        MuiButton: {
            defaultProps: {disableElevation: true},
            styleOverrides: {
                root: {minHeight: 52, padding: "13px 20px", fontSize: "1rem"},
            },
        },
        MuiOutlinedInput: {
            styleOverrides: {
                root: {backgroundColor: "#fff", minHeight: 54},
                input: {padding: "14px 16px", fontSize: "1rem"},
                notchedOutline: {borderColor: "#7c8d98"},
            },
        },
        MuiFormHelperText: {
            styleOverrides: {root: {marginInline: 0, fontSize: ".875rem", lineHeight: 1.6}},
        },
        MuiCheckbox: {styleOverrides: {root: {padding: 10}}},
        MuiAlert: {styleOverrides: {root: {fontSize: ".9375rem", lineHeight: 1.6}}},
    },
})
