// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, type ReactNode} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import type {TemplateProps} from "keycloakify/login/TemplateProps"
import Alert, {type AlertColor} from "@mui/material/Alert"
import Box from "@mui/material/Box"
import CssBaseline from "@mui/material/CssBaseline"
import Paper from "@mui/material/Paper"
import Typography from "@mui/material/Typography"
import {ThemeProvider} from "@mui/material/styles"
import logo from "./assets/sequent-white.svg"
import {GlobeIcon, MessageIcon, ShieldIcon} from "./icons"
import {authTheme} from "./theme"
import {getAuthCopy} from "./authCopy"
import {buildDetail} from "./buildDetail"
import {messageLanguage, type I18n} from "./i18n"
import {EVoterAccessibilitySettingsPolicy, type KcContext} from "./KcContext"
import {AccessibilitySettings} from "./accessibility/AccessibilitySettings"
import {AudioInstructions} from "./accessibility/AudioInstructions"
import {getAccessibilityCopy, isInstructionsPage} from "./accessibility/copy"
import "./auth.css"

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
    const {realm, message, isAppInitiatedAction, properties} = kcContext
    const systemVersion = buildDetail(properties.systemVersion)
    const systemHash = buildDetail(properties.systemHash)
    const voting = kcContext.themeName === "sequent-ui-voting"
    // A product name, the same in every language.
    const architect = kcContext.themeName === "sequent-ui-architect"
    // Keycloakify derives direction from the language when older contexts do
    // not include locale.rtl; preserve that resolved value.
    const direction =
        (kcContext.locale?.rtl ?? document.documentElement.dir === "rtl") ? "rtl" : "ltr"
    const copy = getAuthCopy(currentLanguage.languageTag)
    const accessibilityCopy = getAccessibilityCopy(currentLanguage.languageTag)

    useEffect(() => {
        document.title = documentTitle ?? msgStr("loginTitle", realm.displayName || realm.name)
    }, [documentTitle, msgStr, realm.displayName, realm.name])

    useEffect(() => {
        const previousLang = document.documentElement.lang
        const previousDir = document.documentElement.dir
        document.documentElement.lang = currentLanguage.languageTag
        document.documentElement.dir = direction
        return () => {
            document.documentElement.lang = previousLang
            document.documentElement.dir = previousDir
        }
    }, [currentLanguage.languageTag, direction])

    const showMessage =
        displayMessage &&
        message !== undefined &&
        (message.type !== "warning" || !isAppInitiatedAction)

    return (
        <ThemeProvider theme={authTheme}>
            <CssBaseline />
            <Box className="sequent-auth" lang={currentLanguage.languageTag} dir={direction}>
                <Box className="auth-layout">
                    <Box component="header" className="auth-header">
                        <Box className="auth-brand">
                            <img src={logo} alt="Sequent" width={170} height={32} />
                            {architect ? (
                                <span className="auth-brand-context" lang="en">
                                    Election Architect
                                </span>
                            ) : (
                                <span className="auth-brand-context" lang={copy.languageTag}>
                                    {voting ? copy.votingPortal : copy.adminPortal}
                                </span>
                            )}
                        </Box>
                        <Box className="auth-header-tools">
                            {(systemVersion || systemHash) && (
                                <dl className="auth-build">
                                    {systemVersion && (
                                        <div>
                                            <dt
                                                lang={messageLanguage(
                                                    kcContext,
                                                    i18n,
                                                    "system.version"
                                                )}
                                            >
                                                {msgStr("system.version")}
                                            </dt>
                                            <dd dir="ltr">{systemVersion}</dd>
                                        </div>
                                    )}
                                    {systemHash && (
                                        <div>
                                            <dt
                                                lang={messageLanguage(
                                                    kcContext,
                                                    i18n,
                                                    "system.hash"
                                                )}
                                            >
                                                {msgStr("system.hash")}
                                            </dt>
                                            <dd dir="ltr">{systemHash}</dd>
                                        </div>
                                    )}
                                </dl>
                            )}
                            {kcContext.sequent.voterAccessibilitySettingsPolicy ===
                                EVoterAccessibilitySettingsPolicy.ENABLED && (
                                <AccessibilitySettings copy={accessibilityCopy} />
                            )}
                            {enabledLanguages.length > 1 && (
                                <LanguageSelect
                                    label={msgStr("languages")}
                                    current={currentLanguage.languageTag}
                                    languages={enabledLanguages}
                                />
                            )}
                        </Box>
                    </Box>
                    <Box component="main" aria-labelledby="kc-page-title">
                        <Paper className="auth-card" elevation={0}>
                            <Box className="auth-symbol" aria-hidden="true">
                                {kcContext.pageId === "message-otp.login.ftl" ? (
                                    <MessageIcon />
                                ) : (
                                    <ShieldIcon />
                                )}
                            </Box>
                            <Typography className="auth-eyebrow" lang={copy.languageTag}>
                                {voting ? copy.votingEyebrow : copy.adminEyebrow}
                            </Typography>
                            <Typography
                                id="kc-page-title"
                                className="auth-title"
                                component="h1"
                                lang={messageLanguage(
                                    kcContext,
                                    i18n,
                                    kcContext.pageId === "message-otp.login.ftl"
                                        ? `messageOtp.${kcContext.isOtl ? "otl" : "auth"}.title`
                                        : "loginAccountTitle"
                                )}
                            >
                                {headerNode}
                            </Typography>
                            {showMessage && (
                                <Alert
                                    id="kc-feedback"
                                    className="auth-feedback"
                                    severity={MESSAGE_SEVERITY[message.type]}
                                    role={message.type === "error" ? "alert" : "status"}
                                >
                                    <span
                                        dangerouslySetInnerHTML={{
                                            __html: kcSanitize(message.summary),
                                        }}
                                    />
                                </Alert>
                            )}
                            {isInstructionsPage(kcContext.pageId) && (
                                <AudioInstructions
                                    policy={kcContext.sequent.audioInstructionsPolicy}
                                    text={accessibilityCopy.instructions[kcContext.pageId]}
                                    copy={accessibilityCopy}
                                />
                            )}
                            {children}
                            {displayInfo && <Box className="auth-info">{infoNode}</Box>}
                        </Paper>
                    </Box>
                    <Box component="footer" className="auth-footer">
                        <span lang={copy.languageTag}>{copy.poweredBy}</span>
                    </Box>
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
        <Box className="auth-language">
            <GlobeIcon />
            <select
                value={current}
                aria-label={label}
                onChange={(event) => {
                    const target = languages.find(
                        ({languageTag}) => languageTag === event.target.value
                    )
                    if (target !== undefined) {
                        window.location.href = target.href
                    }
                }}
            >
                {languages.map(({languageTag, label: name}) => (
                    <option key={languageTag} value={languageTag} lang={languageTag}>
                        {name}
                    </option>
                ))}
            </select>
        </Box>
    )
}
