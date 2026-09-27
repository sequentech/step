// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useState} from "react"
import {kcSanitize} from "keycloakify/lib/kcSanitize"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import Alert from "@mui/material/Alert"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import type {I18n} from "../i18n"
import type {KcContext} from "../KcContext"
import {ArrowIcon} from "../icons"

// The first step of a username-then-credential flow, e.g. an email address
// followed by an emailed code. Identity providers are offered alongside it.
export default function LoginUsername(
    props: PageProps<Extract<KcContext, {pageId: "login-username.ftl"}>, I18n>
) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {realm, url, usernameHidden, login, social, messagesPerField} = kcContext
    const {msg, msgStr} = i18n
    const [isSubmitting, setIsSubmitting] = useState(false)
    const usernameError = messagesPerField.existsError("username")
    const providers = social?.providers ?? []
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
            displayMessage={!usernameError}
            headerNode={msg("loginAccountTitle")}
        >
            {!usernameHidden && (
                <Box
                    component="form"
                    id="kc-form-login"
                    action={url.loginAction}
                    method="post"
                    onSubmit={() => {
                        setIsSubmitting(true)
                        return true
                    }}
                    className="auth-form"
                >
                    {usernameError && (
                        <Alert id="input-error" severity="error" role="alert">
                            <span
                                dangerouslySetInnerHTML={{
                                    __html: kcSanitize(messagesPerField.getFirstError("username")),
                                }}
                            />
                        </Alert>
                    )}
                    <Box>
                        <Typography component="label" htmlFor="username" className="auth-field-label">
                            {usernameLabel}
                        </Typography>
                        <TextField
                            id="username"
                            name="username"
                            type={realm.registrationEmailAsUsername ? "email" : "text"}
                            defaultValue={login.username ?? ""}
                            autoFocus
                            autoComplete="username"
                            required
                            error={usernameError}
                            slotProps={{
                                htmlInput: {
                                    "aria-describedby": usernameError ? "input-error" : undefined,
                                },
                            }}
                            fullWidth
                        />
                    </Box>
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
                        {msgStr("doContinue")}
                    </Button>
                </Box>
            )}
            {providers.length > 0 && (
                <Box component="nav" id="kc-social-providers" className="auth-providers" aria-labelledby="kc-social-providers-title">
                    <Typography id="kc-social-providers-title" className="auth-providers-title">
                        {msg("identity-provider-login-label")}
                    </Typography>
                    {providers.map((provider) => (
                        <Button
                            key={provider.alias}
                            id={`social-${provider.alias}`}
                            href={provider.loginUrl}
                            variant="outlined"
                            className="auth-provider"
                            fullWidth
                        >
                            {msgStr("continueWithProvider", provider.displayName)}
                        </Button>
                    ))}
                </Box>
            )}
        </Template>
    )
}
