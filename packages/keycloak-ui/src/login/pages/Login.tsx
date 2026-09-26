// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useState} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import Checkbox from "@mui/material/Checkbox"
import Alert from "@mui/material/Alert"
import FormControlLabel from "@mui/material/FormControlLabel"
import IconButton from "@mui/material/IconButton"
import InputAdornment from "@mui/material/InputAdornment"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import type {I18n} from "../i18n"
import {LoginHintUsernamePolicy, LoginValidationPolicy, type KcContext} from "../KcContext"
import {ArrowIcon, EyeIcon} from "../icons"

export default function Login(props: PageProps<Extract<KcContext, {pageId: "login.ftl"}>, I18n>) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {realm, url, usernameHidden, login, auth, registrationDisabled, messagesPerField} =
        kcContext
    const {msg, msgStr} = i18n
    const [isSubmitting, setIsSubmitting] = useState(false)
    const [passwordVisible, setPasswordVisible] = useState(false)
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
                    className="auth-form"
                >
                    {credentialError && (
                        <Alert id="input-error" severity="error" role="alert">
                            {errorText}
                        </Alert>
                    )}
                    {!usernameHidden && (
                        <Box>
                            <Typography
                                component="label"
                                htmlFor="username"
                                className="auth-field-label"
                            >
                                {usernameLabel}
                            </Typography>
                            <TextField
                                id="username"
                                name="username"
                                defaultValue={login.username ?? ""}
                                autoFocus
                                autoComplete="username"
                                slotProps={{
                                    htmlInput: {
                                        readOnly: usernameReadOnly,
                                        "aria-describedby": credentialError
                                            ? "input-error"
                                            : undefined,
                                    },
                                }}
                                error={credentialError}
                                fullWidth
                            />
                        </Box>
                    )}
                    <Box>
                        <Typography
                            component="label"
                            htmlFor="password"
                            className="auth-field-label"
                        >
                            {msgStr("password")}
                        </Typography>
                        <TextField
                            id="password"
                            name="password"
                            type={passwordVisible ? "text" : "password"}
                            autoFocus={usernameHidden}
                            autoComplete="current-password"
                            error={credentialError}
                            slotProps={{
                                htmlInput: {
                                    "aria-describedby": credentialError ? "input-error" : undefined,
                                },
                                input: {
                                    endAdornment: (
                                        <InputAdornment position="end">
                                            <IconButton
                                                type="button"
                                                aria-label={msgStr(
                                                    passwordVisible
                                                        ? "hidePassword"
                                                        : "showPassword"
                                                )}
                                                aria-controls="password"
                                                onClick={() => setPasswordVisible(!passwordVisible)}
                                                edge="end"
                                                sx={{minWidth: 44, minHeight: 44}}
                                            >
                                                <EyeIcon hidden={passwordVisible} />
                                            </IconButton>
                                        </InputAdornment>
                                    ),
                                },
                            }}
                            fullWidth
                        />
                    </Box>
                    <Box className="auth-options">
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
                    </Box>
                    <input type="hidden" name="credentialId" value={auth.selectedCredential} />
                    <Button
                        id="kc-login"
                        name="login"
                        type="submit"
                        variant="contained"
                        disabled={isSubmitting}
                        className="auth-submit"
                        endIcon={<ArrowIcon />}
                        fullWidth
                    >
                        {msgStr("doLogIn")}
                    </Button>
                </Box>
            )}
        </Template>
    )
}
