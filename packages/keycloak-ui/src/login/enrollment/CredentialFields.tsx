// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useState} from "react"
import type {Attribute} from "keycloakify/login/KcContext"
import IconButton from "@mui/material/IconButton"
import InputAdornment from "@mui/material/InputAdornment"
import TextField from "@mui/material/TextField"
import {EyeIcon} from "../icons"
import type {I18n} from "../i18n"
import type {KcContext} from "../KcContext"
import {textFor} from "../scanovate/text"
import {loadScript} from "./assets"
import {FieldError, HelperPlacement, HelperText, RequiredMark, helperId} from "./ProfileFields"
import {annotation, strengthOf} from "./profile"

declare global {
    interface Window {
        // From sequent-theme's resources.
        zxcvbn?: (password: string) => {score: number}
    }
}

type Register = Extract<KcContext, {pageId: "register.ftl"}>

function PasswordInput(props: {
    id: string
    i18n: I18n
    invalid: boolean
    describedBy?: string
    autoFocus?: boolean
    value: string
    onChange: (value: string) => void
}) {
    const {id, i18n, invalid, describedBy, autoFocus, value, onChange} = props
    const [visible, setVisible] = useState(false)
    return (
        <TextField
            id={id}
            name={id}
            type={visible ? "text" : "password"}
            value={value}
            onChange={(event) => onChange(event.target.value)}
            autoComplete="new-password"
            autoFocus={autoFocus}
            required
            error={invalid}
            fullWidth
            slotProps={{
                htmlInput: {"aria-describedby": describedBy},
                input: {
                    endAdornment: (
                        <InputAdornment position="end">
                            <IconButton
                                type="button"
                                aria-label={i18n.msgStr(visible ? "hidePassword" : "showPassword")}
                                aria-controls={id}
                                onClick={() => setVisible(!visible)}
                                edge="end"
                                sx={{minWidth: 44, minHeight: 44}}
                            >
                                <EyeIcon hidden={visible} />
                            </IconButton>
                        </InputAdornment>
                    ),
                },
            }}
        />
    )
}

// zxcvbn, once the theme's copy is loaded.
function useStrengthScore(resources: string, enabled: boolean) {
    const [ready, setReady] = useState(false)
    useEffect(() => {
        if (!enabled) return
        let current = true
        const check = () => {
            if (current) setReady(typeof window.zxcvbn === "function")
        }
        if (typeof window.zxcvbn === "function") check()
        else void loadScript(`${resources}/js/zxcvbn.js`).then(check, () => undefined)
        return () => {
            current = false
        }
    }, [resources, enabled])
    return ready ? (password: string) => window.zxcvbn?.(password).score ?? 0 : undefined
}

function joined(...ids: (string | false)[]): string | undefined {
    return ids.filter((id) => id !== false).join(" ") || undefined
}

// The password a voter chooses, with the helper texts and strength bar that
// the annotations of the attribute next to it ask for.
export default function CredentialFields(props: {
    kcContext: Register
    i18n: I18n
    source: Attribute
    // Asked for twice, with a strength bar: the voter is choosing it.
    creating: boolean
    autoFocus: boolean
}) {
    const {kcContext, i18n, source, creating, autoFocus} = props
    const {messagesPerField, url} = kcContext
    const [password, setPassword] = useState("")
    const [confirmation, setConfirmation] = useState("")
    const text = textFor(kcContext, i18n)
    const before = annotation(source, "passwordHelperTextBefore") ?? ""
    const after = annotation(source, "passwordHelperTextAfter") ?? ""
    const score = useStrengthScore(
        url.resourcesPath,
        creating && annotation(source, "passwordStrengthBar") !== undefined
    )
    const strength = score === undefined ? undefined : strengthOf(password, score(password))
    const strengthLabel = text("enrollmentPasswordStrength")
    const invalid = messagesPerField.existsError("password")
    const confirmationInvalid = messagesPerField.existsError("password-confirm")
    const beforeId = helperId("password", HelperPlacement.Before)
    const afterId = helperId("password", HelperPlacement.After)
    return (
        <>
            <div>
                <label htmlFor="password" className="auth-field-label">
                    {i18n.msgStr("password")}
                    <RequiredMark />
                </label>
                <HelperText
                    name="password"
                    placement={HelperPlacement.Before}
                    text={before}
                    i18n={i18n}
                />
                <PasswordInput
                    id="password"
                    i18n={i18n}
                    invalid={invalid || confirmationInvalid}
                    describedBy={joined(
                        before !== "" && beforeId,
                        invalid && "input-error-password",
                        after !== "" && afterId
                    )}
                    autoFocus={autoFocus}
                    value={password}
                    onChange={setPassword}
                />
                {strength !== undefined && (
                    <div
                        id="password-progress"
                        className="auth-strength"
                        role="progressbar"
                        aria-label={strengthLabel.text}
                        lang={strengthLabel.lang}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={strength.percent}
                        data-level={strength.level}
                    >
                        <span style={{width: `${strength.percent}%`}} />
                    </div>
                )}
                {invalid && (
                    <FieldError id="input-error-password" html={messagesPerField.get("password")} />
                )}
                <HelperText
                    name="password"
                    placement={HelperPlacement.After}
                    text={after}
                    i18n={i18n}
                />
            </div>
            {creating && (
                <div>
                    <label htmlFor="password-confirm" className="auth-field-label">
                        {i18n.msgStr("passwordConfirm")}
                        <RequiredMark />
                    </label>
                    <PasswordInput
                        id="password-confirm"
                        i18n={i18n}
                        invalid={confirmationInvalid}
                        describedBy={joined(confirmationInvalid && "input-error-password-confirm")}
                        value={confirmation}
                        onChange={setConfirmation}
                    />
                    {confirmationInvalid && (
                        <FieldError
                            id="input-error-password-confirm"
                            html={messagesPerField.get("password-confirm")}
                        />
                    )}
                </div>
            )}
        </>
    )
}
