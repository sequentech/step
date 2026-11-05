// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useMemo, useState} from "react"
import {KEYCLOAK_SYNTHETIC_USER} from "@sequentech/ui-test-kit/fixtures/keycloak"
import KcPage from "./login/KcPage"
import {getKcContextMock} from "./login/KcPageStory"
import {LoginHintUsernamePolicy, LoginValidationPolicy, MessageCourier} from "./login/KcContext"

enum Scenario {
    Login = "login",
    ReadOnly = "login-hint",
    ServerOnly = "server-validation",
    EmailOtp = "email-otp",
    SmsOtp = "sms-otp",
    Link = "one-time-link",
}

const titles: Record<Scenario, string> = {
    [Scenario.Login]: "Login",
    [Scenario.ReadOnly]: "Read-only login hint",
    [Scenario.ServerOnly]: "Server validation",
    [Scenario.EmailOtp]: "Email OTP",
    [Scenario.SmsOtp]: "SMS OTP",
    [Scenario.Link]: "One-time link",
}

export default function Preview() {
    const query = new URLSearchParams(window.location.search)
    const [scenario, setScenario] = useState(
        Object.values(Scenario).find((value) => value === query.get("scenario")) ?? Scenario.Login
    )
    const [locale, setLocale] = useState(query.get("locale") === "es" ? "es" : "en")
    const context = useMemo(() => {
        const login = [Scenario.Login, Scenario.ReadOnly, Scenario.ServerOnly].includes(scenario)
        const result = login
            ? getKcContextMock({
                  pageId: "login.ftl",
                  overrides: {
                      themeName: "sequent-ui-voting",
                      login: {
                          username:
                              scenario === Scenario.ReadOnly
                                  ? KEYCLOAK_SYNTHETIC_USER.username
                                  : "",
                      },
                      sequent: {
                          loginHintUsernamePolicy:
                              scenario === Scenario.ReadOnly
                                  ? LoginHintUsernamePolicy.ReadOnly
                                  : LoginHintUsernamePolicy.Editable,
                          loginValidationPolicy:
                              scenario === Scenario.ServerOnly
                                  ? LoginValidationPolicy.ServerOnly
                                  : LoginValidationPolicy.Browser,
                      },
                  },
              })
            : getKcContextMock({
                  pageId: "message-otp.login.ftl",
                  overrides: {
                      courier:
                          scenario === Scenario.SmsOtp ? MessageCourier.Sms : MessageCourier.Email,
                      isOtl: scenario === Scenario.Link,
                  },
              })
        if (result.locale) result.locale.currentLanguageTag = locale
        return result
    }, [scenario, locale])

    return (
        <div onSubmit={(event) => event.preventDefault()}>
            <aside aria-label="Preview controls" style={{padding: 16}}>
                <label>
                    Scenario{" "}
                    <select
                        value={scenario}
                        onChange={(event) => setScenario(event.target.value as Scenario)}
                    >
                        {Object.values(Scenario).map((value) => (
                            <option key={value} value={value}>
                                {titles[value]}
                            </option>
                        ))}
                    </select>
                </label>{" "}
                <label>
                    Language{" "}
                    <select value={locale} onChange={(event) => setLocale(event.target.value)}>
                        <option value="en">English</option>
                        <option value="es">Spanish</option>
                    </select>
                </label>
                <p>Synthetic preview. Use the proxied realm URL to test real authentication.</p>
            </aside>
            <KcPage key={`${scenario}:${locale}`} kcContext={context} />
        </div>
    )
}
