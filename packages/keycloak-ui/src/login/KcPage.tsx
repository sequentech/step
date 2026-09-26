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
const MessageOtpLogin = lazy(() => import("./pages/MessageOtpLogin"))
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
