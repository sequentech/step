// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent} from "react"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import {messageLanguage, type I18n} from "../i18n"
import {MessageCourier, type KcContext} from "../KcContext"
import {ArrowIcon} from "../icons"

type OtpContext = Extract<KcContext, {pageId: "message-otp.login.ftl"}>

const DEFAULT_CODE_LENGTH = 6
const DEFAULT_RESEND_SECONDS = 60
// Shared with message-otp.login.ftl, so switching themes keeps the countdown.
const RESEND_END_KEY = "resendOtpEndTime"

const INSTRUCTIONS: Record<MessageCourier, "Sms" | "Email" | "Both"> = {
    [MessageCourier.Sms]: "Sms",
    [MessageCourier.Email]: "Email",
    [MessageCourier.Both]: "Both",
    [MessageCourier.None]: "Both",
}

function useResendCountdown(seconds: number, codeJustSent: boolean): number {
    const [remaining, setRemaining] = useState(() => {
        if (codeJustSent) {
            localStorage.setItem(RESEND_END_KEY, String(Date.now() + seconds * 1000))
        }
        const end = Number(localStorage.getItem(RESEND_END_KEY) ?? 0)
        return Math.max(Math.ceil((end - Date.now()) / 1000), 0)
    })
    useEffect(() => {
        if (remaining === 0) {
            return
        }
        const timer = setTimeout(() => setRemaining(remaining - 1), 1000)
        return () => clearTimeout(timer)
    }, [remaining])
    return remaining
}

export default function MessageOtpLogin(props: PageProps<OtpContext, I18n>) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {url, address, courier, isOtl, codeJustSent, resendTimer, ttl, codeLength} = kcContext
    const {codeRequest, codeRequests} = kcContext
    const {msg, msgStr} = i18n
    const flow = isOtl ? "otl" : "auth"
    const length = Number(codeLength ?? DEFAULT_CODE_LENGTH)
    const [digits, setDigits] = useState<string[]>(() => Array(length).fill(""))
    const inputs = useRef<(HTMLInputElement | null)[]>([])
    const submit = useRef<HTMLButtonElement | null>(null)
    const codeError = kcContext.message?.type === "error"
    const remaining = useResendCountdown(
        Number(resendTimer ?? DEFAULT_RESEND_SECONDS),
        codeJustSent === true
    )

    const focusInput = (index: number) => {
        if (index >= length) {
            submit.current?.focus()
        } else {
            inputs.current[index]?.focus()
            inputs.current[index]?.select()
        }
    }
    const enterCode = (value: string) => {
        const pasted = value.replace(/\D/g, "").slice(0, length)
        if (pasted === "") return
        setDigits(Array.from({length}, (_, index) => pasted[index] ?? ""))
        focusInput(pasted.length)
    }
    const setDigit = (index: number, value: string) => {
        // SMS autofill and password managers may populate the first field with
        // the whole code in one input event. Never discard all but its last digit.
        if (value.length > 1) {
            enterCode(value)
            return
        }
        const next = [...digits]
        next[index] = value.replace(/\D/g, "")
        setDigits(next)
        if (next[index] !== "") {
            focusInput(index + 1)
        }
    }
    const onKeyDown = (index: number, event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key === "Backspace") {
            event.preventDefault()
            setDigits((current) =>
                current.map((digit, position) => (position === index ? "" : digit))
            )
            focusInput(Math.max(0, index - 1))
        } else if (event.key === "ArrowLeft" && index > 0) {
            event.preventDefault()
            focusInput(index - 1)
        } else if (event.key === "ArrowRight") {
            event.preventDefault()
            focusInput(index + 1)
        }
    }
    const onPaste = (event: ClipboardEvent<HTMLInputElement>) => {
        event.preventDefault()
        enterCode(event.clipboardData.getData("text"))
    }
    const instruction = INSTRUCTIONS[courier ?? MessageCourier.Both]
    // Several codes in a row: says which one this is, so the next doesn't look like a retry.
    const counted = codeRequest !== undefined && codeRequests !== undefined && codeRequests > 1
    const codeLabel = {
        text: msgStr("messageOtp.auth.codeProgress", String(codeRequest), String(codeRequests)),
        lang: messageLanguage(kcContext, i18n, "messageOtp.auth.codeProgress"),
    }
    const codeNote =
        codeRequest === codeRequests
            ? {key: "messageOtp.auth.codeLast" as const, args: [] as string[]}
            : {key: "messageOtp.auth.codeNext" as const, args: [String(codeRequests)]}

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            displayInfo={ttl !== undefined}
            headerNode={msg(`messageOtp.${flow}.title`)}
            {...(counted
                ? {
                      eyebrow: codeLabel,
                      progress: {step: codeRequest, total: codeRequests, label: codeLabel},
                  }
                : {})}
            infoNode={
                <Typography
                    id="otp-validity"
                    variant="body2"
                    lang={messageLanguage(kcContext, i18n, `messageOtp.${flow}.ttlTime`)}
                >
                    {msg(`messageOtp.${flow}.ttlTime`, String(Math.round(Number(ttl) / 60)))}
                </Typography>
            }
        >
            <Typography
                className="auth-address"
                lang={messageLanguage(kcContext, i18n, `messageOtp.${flow}.address`)}
            >
                {msg(`messageOtp.${flow}.address`, address)}
            </Typography>
            {counted && (
                <Typography
                    id="otp-progress"
                    className="auth-lead"
                    lang={messageLanguage(kcContext, i18n, codeNote.key)}
                >
                    {msgStr(codeNote.key, ...codeNote.args)}
                </Typography>
            )}
            <Typography
                id="otp-instructions"
                className="kc-message-otl-instructions auth-instructions"
                lang={messageLanguage(
                    kcContext,
                    i18n,
                    `messageOtp.${flow}.instruction${instruction}`
                )}
            >
                {msg(`messageOtp.${flow}.instruction${instruction}`)}
            </Typography>
            <Box
                component="form"
                id="kc-message-code-login-form"
                action={url.loginAction}
                method="post"
                className="auth-form"
            >
                {!isOtl && (
                    <>
                        <Box
                            component="fieldset"
                            className="auth-code-group"
                            lang={messageLanguage(kcContext, i18n, "otpCodeLabel")}
                            aria-describedby={[
                                "otp-instructions",
                                ttl !== undefined && "otp-validity",
                                codeError && "kc-feedback",
                            ]
                                .filter(Boolean)
                                .join(" ")}
                        >
                            <Typography component="legend" className="auth-field-label">
                                {msgStr("otpCodeLabel")}
                            </Typography>
                            <Box
                                id="otp-inputs"
                                className="auth-code-inputs"
                                sx={{
                                    gridTemplateColumns: `repeat(${Math.min(length, 6)}, minmax(0, 1fr))`,
                                }}
                            >
                                {digits.map((digit, index) => (
                                    <TextField
                                        key={index}
                                        id={`otp-${index + 1}`}
                                        value={digit}
                                        autoFocus={index === 0}
                                        autoComplete={index === 0 ? "one-time-code" : "off"}
                                        error={codeError}
                                        inputRef={(element) => {
                                            inputs.current[index] = element
                                        }}
                                        onChange={(event) => setDigit(index, event.target.value)}
                                        onFocus={(event) => event.target.select()}
                                        onKeyDown={(event) =>
                                            onKeyDown(
                                                index,
                                                event as KeyboardEvent<HTMLInputElement>
                                            )
                                        }
                                        onPaste={(event) =>
                                            onPaste(event as ClipboardEvent<HTMLInputElement>)
                                        }
                                        slotProps={{
                                            htmlInput: {
                                                inputMode: "numeric",
                                                pattern: "\\d",
                                                maxLength: length,
                                                "aria-describedby": codeError
                                                    ? "kc-feedback"
                                                    : undefined,
                                                "aria-label": msgStr(
                                                    "otpDigit",
                                                    String(index + 1),
                                                    String(length)
                                                ),
                                                lang: messageLanguage(kcContext, i18n, "otpDigit"),
                                            },
                                        }}
                                    />
                                ))}
                            </Box>
                        </Box>
                        <input type="hidden" id="code" name="code" value={digits.join("")} />
                        <Box className="auth-actions">
                            <Button
                                id="kc-form-submit"
                                type="submit"
                                variant="contained"
                                fullWidth
                                ref={submit}
                                className="auth-submit"
                                endIcon={<ArrowIcon />}
                            >
                                {msgStr("doSubmit")}
                            </Button>
                        </Box>
                    </>
                )}
                <Button
                    id="resend-otp-btn"
                    type="submit"
                    name="resend"
                    value="true"
                    formNoValidate
                    variant="outlined"
                    disabled={remaining > 0}
                    className="auth-resend"
                    lang={messageLanguage(
                        kcContext,
                        i18n,
                        `messageOtp.${flow}.resend.${remaining > 0 ? "timer" : "button"}`
                    )}
                >
                    {remaining > 0
                        ? msgStr(`messageOtp.${flow}.resend.timer`, String(remaining))
                        : msgStr(`messageOtp.${flow}.resend.button`)}
                </Button>
            </Box>
        </Template>
    )
}
