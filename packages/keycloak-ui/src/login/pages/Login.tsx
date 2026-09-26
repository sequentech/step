// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useState} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import Checkbox from "@mui/material/Checkbox"
import FormControlLabel from "@mui/material/FormControlLabel"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import type {I18n} from "../i18n"
import {LoginHintUsernamePolicy, LoginValidationPolicy, type KcContext} from "../KcContext"

export default function Login(props: PageProps<Extract<KcContext, {pageId: "login.ftl"}>, I18n>) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {realm, url, usernameHidden, login, auth, registrationDisabled, messagesPerField} =
        kcContext
    const {msg, msgStr} = i18n
    const [isSubmitting, setIsSubmitting] = useState(false)
    const credentialError = messagesPerField.existsError("username", "password")
    const usernameReadOnly =
        kcContext.themeName === "sequent-ui-voting" &&
        Boolean(login.username) &&
        login.rememberMe === undefined &&
        kcContext.sequent.loginHintUsernamePolicy === LoginHintUsernamePolicy.ReadOnly
    const errorText = credentialError ? (
        <span
            dangerouslySetInnerHTML={{
                __html: kcSanitize(messagesPerField.getFirstError("username", "password")),
            }}
        />
    ) : undefined
    const usernameLabel = !realm.loginWithEmailAllowed
        ? msgStr("username")
        : !realm.registrationEmailAsUsername
          ? msgStr("usernameOrEmail")
          : msgStr("email")

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            displayMessage={!credentialError}
            displayInfo={realm.password && realm.registrationAllowed && !registrationDisabled}
            headerNode={msg("loginAccountTitle")}
            infoNode={
                <Typography variant="body2">
                    {msg("noAccount")} <a href={url.registrationUrl}>{msg("doRegister")}</a>
                </Typography>
            }
        >
            {realm.password && (
                <Box
                    component="form"
                    id="kc-form-login"
                    action={url.loginAction}
                    method="post"
                    noValidate={
                        kcContext.sequent.loginValidationPolicy === LoginValidationPolicy.ServerOnly
                    }
                    onSubmit={() => {
                        setIsSubmitting(true)
                        return true
                    }}
                    sx={{display: "flex", flexDirection: "column", gap: 2}}
                >
                    {!usernameHidden && (
                        <TextField
                            id="username"
                            name="username"
                            label={usernameLabel}
                            defaultValue={login.username ?? ""}
                            autoFocus
                            autoComplete="username"
                            slotProps={{htmlInput: {readOnly: usernameReadOnly}}}
                            error={credentialError}
                            helperText={errorText}
                            fullWidth
                        />
                    )}
                    <TextField
                        id="password"
                        name="password"
                        type="password"
                        label={msgStr("password")}
                        autoComplete="current-password"
                        error={credentialError}
                        helperText={usernameHidden ? errorText : undefined}
                        fullWidth
                    />
                    {realm.rememberMe && !usernameHidden && (
                        <FormControlLabel
                            control={
                                <Checkbox
                                    id="rememberMe"
                                    name="rememberMe"
                                    defaultChecked={login.rememberMe === "on"}
                                />
                            }
                            label={msgStr("rememberMe")}
                        />
                    )}
                    {realm.resetPasswordAllowed && (
                        <a href={url.loginResetCredentialsUrl}>{msg("doForgotPassword")}</a>
                    )}
                    <input type="hidden" name="credentialId" value={auth.selectedCredential} />
                    <Button
                        id="kc-login"
                        name="login"
                        type="submit"
                        variant="contained"
                        disabled={isSubmitting}
                        fullWidth
                    >
                        {msgStr("doLogIn")}
                    </Button>
                </Box>
            )}
        </Template>
    )
}
