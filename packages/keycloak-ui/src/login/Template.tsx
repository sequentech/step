// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, type ReactNode} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import type {TemplateProps} from "keycloakify/login/TemplateProps"
import Alert, {type AlertColor} from "@mui/material/Alert"
import Box from "@mui/material/Box"
import CssBaseline from "@mui/material/CssBaseline"
import MenuItem from "@mui/material/MenuItem"
import Paper from "@mui/material/Paper"
import Select from "@mui/material/Select"
import Typography from "@mui/material/Typography"
import {ThemeProvider} from "@mui/material/styles"
import theme from "@sequentech/ui-essentials/theme"
import logo from "../../../ui-essentials/public/Sequent_logo.svg"
import type {I18n} from "./i18n"
import type {KcContext} from "./KcContext"

const MESSAGE_SEVERITY: Record<string, AlertColor> = {
    success: "success",
    warning: "warning",
    error: "error",
    info: "info",
}

export default function Template(props: TemplateProps<KcContext, I18n>) {
    const {
        displayInfo = false,
        displayMessage = true,
        headerNode,
        infoNode = null,
        documentTitle,
        kcContext,
        i18n,
        children,
    } = props
    const {msgStr, currentLanguage, enabledLanguages} = i18n
    const {realm, message, isAppInitiatedAction} = kcContext

    useEffect(() => {
        document.title = documentTitle ?? msgStr("loginTitle", realm.displayName || realm.name)
    }, [documentTitle, msgStr, realm.displayName, realm.name])

    const showMessage =
        displayMessage &&
        message !== undefined &&
        (message.type !== "warning" || !isAppInitiatedAction)

    return (
        <ThemeProvider theme={theme}>
            <CssBaseline />
            <Box sx={{minHeight: "100vh", display: "flex", flexDirection: "column"}}>
                <Box
                    component="header"
                    sx={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        px: 4,
                        py: 2,
                        bgcolor: "background.default",
                    }}
                >
                    <img src={logo} alt="Sequent" height={40} />
                    {enabledLanguages.length > 1 && (
                        <LanguageSelect
                            label={msgStr("languages")}
                            current={currentLanguage.languageTag}
                            languages={enabledLanguages}
                        />
                    )}
                </Box>
                <Box component="main" sx={{flex: 1, display: "flex", justifyContent: "center"}}>
                    <Paper variant="outlined" sx={{mt: 8, mb: "auto", p: 5, width: 500}}>
                        <Typography id="kc-page-title" variant="h4" component="h1" gutterBottom>
                            {headerNode}
                        </Typography>
                        {showMessage && (
                            <Alert severity={MESSAGE_SEVERITY[message.type]} sx={{mb: 2}}>
                                <span
                                    dangerouslySetInnerHTML={{
                                        __html: kcSanitize(message.summary),
                                    }}
                                />
                            </Alert>
                        )}
                        {children}
                        {displayInfo && <Box sx={{mt: 3}}>{infoNode}</Box>}
                    </Paper>
                </Box>
                <Box component="footer" sx={{py: 1.5, textAlign: "center"}}>
                    <Typography variant="body2">Powered by Sequent Tech Inc</Typography>
                </Box>
            </Box>
        </ThemeProvider>
    )
}

function LanguageSelect(props: {
    label: string
    current: string
    languages: {languageTag: string; label: string; href: string}[]
}): ReactNode {
    const {label, current, languages} = props
    return (
        <Select
            size="small"
            value={current}
            inputProps={{"aria-label": label}}
            onChange={(event) => {
                const target = languages.find(({languageTag}) => languageTag === event.target.value)
                if (target !== undefined) {
                    window.location.href = target.href
                }
            }}
        >
            {languages.map(({languageTag, label: name}) => (
                <MenuItem key={languageTag} value={languageTag} lang={languageTag}>
                    {name}
                </MenuItem>
            ))}
        </Select>
    )
}
