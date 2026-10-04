// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Suspense, lazy} from "react"
import type {ClassKey} from "keycloakify/login"
import DefaultPage from "keycloakify/login/DefaultPage"
import DefaultTemplate from "keycloakify/login/Template"
import type {KcContext} from "./KcContext"
import {useI18n} from "./i18n"
import Template from "./Template"

const Login = lazy(() => import("./pages/Login"))
const LoginUsername = lazy(() => import("./pages/LoginUsername"))
const MessageOtpLogin = lazy(() => import("./pages/MessageOtpLogin"))
const ScanovateCapture = lazy(() => import("./pages/ScanovateCapture"))
const ScanovateError = lazy(() => import("./pages/ScanovateError"))
const ScanovateConfirmation = lazy(() => import("./pages/ScanovateConfirmation"))
const Register = lazy(() => import("./pages/Register"))
const EnrollmentFinish = lazy(() => import("./pages/EnrollmentFinish"))
const UserProfileFormFields = lazy(() => import("keycloakify/login/UserProfileFormFields"))

const classes = {} satisfies {[key in ClassKey]?: string}

export default function KcPage(props: {kcContext: KcContext}) {
    const {kcContext} = props
    const {i18n} = useI18n({kcContext})

    return (
        <Suspense>
            {(() => {
                switch (kcContext.pageId) {
                    case "login.ftl":
                        return (
                            <Login
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "login-username.ftl":
                        return (
                            <LoginUsername
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "message-otp.login.ftl":
                        return (
                            <MessageOtpLogin
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "scanovate-capture.ftl":
                        return (
                            <ScanovateCapture
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "scanovate-error.ftl":
                        return (
                            <ScanovateError
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "scanovate-confirmation.ftl":
                        return (
                            <ScanovateConfirmation
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "register.ftl":
                        return (
                            <Register
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    case "registration-finish.ftl":
                    case "registration-manual-finish.ftl":
                    case "registration-rejected-finish.ftl":
                    case "message-finish.ftl":
                        return (
                            <EnrollmentFinish
                                kcContext={kcContext}
                                i18n={i18n}
                                Template={Template}
                                doUseDefaultCss={false}
                                classes={classes}
                            />
                        )
                    // Synthetic fallback; mounted themes inherit unported FreeMarker pages.
                    default:
                        return (
                            <DefaultPage
                                kcContext={kcContext}
                                i18n={i18n}
                                classes={classes}
                                Template={DefaultTemplate}
                                doUseDefaultCss={true}
                                UserProfileFormFields={UserProfileFormFields}
                                doMakeUserConfirmPassword={true}
                            />
                        )
                }
            })()}
        </Suspense>
    )
}
