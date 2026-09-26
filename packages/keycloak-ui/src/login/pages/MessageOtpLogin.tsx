// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent} from "react"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import type {I18n} from "../i18n"
import {MessageCourier, type KcContext} from "../KcContext"

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
    const {msg, msgStr} = i18n
    const flow = isOtl ? "otl" : "auth"
    const length = Number(codeLength ?? DEFAULT_CODE_LENGTH)
    const [digits, setDigits] = useState<string[]>(() => Array(length).fill(""))
    const inputs = useRef<(HTMLInputElement | null)[]>([])
    const remaining = useResendCountdown(
        Number(resendTimer ?? DEFAULT_RESEND_SECONDS),
        codeJustSent === true
    )

    const setDigit = (index: number, value: string) => {
        const next = [...digits]
        next[index] = value.slice(-1)
        setDigits(next)
        if (value !== "" && index < length - 1) {
            inputs.current[index + 1]?.focus()
        }
    }
    const onKeyDown = (index: number, event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key === "Backspace" && digits[index] === "" && index > 0) {
            inputs.current[index - 1]?.focus()
        }
    }
    const onPaste = (event: ClipboardEvent<HTMLInputElement>) => {
        const pasted = event.clipboardData.getData("text").trim().slice(0, length)
        event.preventDefault()
        setDigits(Array.from({length}, (_, index) => pasted[index] ?? ""))
        inputs.current[Math.min(pasted.length, length - 1)]?.focus()
    }
    const instruction = INSTRUCTIONS[courier]

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            displayInfo
            headerNode={msg(`messageOtp.${flow}.address`, address)}
            infoNode={
                <>
                    <Typography className="kc-message-otl-instructions">
                        {msg(`messageOtp.${flow}.instruction${instruction}`)}
                    </Typography>
                    {ttl !== undefined && (
                        <Typography variant="body2">
                            {msg(
                                `messageOtp.${flow}.ttlTime`,
                                String(Math.round(Number(ttl) / 60))
                            )}
                        </Typography>
                    )}
                </>
            }
        >
            <Box
                component="form"
                id="kc-message-code-login-form"
                action={url.loginAction}
                method="post"
                sx={{display: "flex", flexDirection: "column", gap: 2}}
            >
                {!isOtl && (
                    <>
                        <Box
                            id="otp-inputs"
                            sx={{display: "flex", justifyContent: "center", gap: 1}}
                        >
                            {digits.map((digit, index) => (
                                <TextField
                                    key={index}
                                    id={`otp-${index + 1}`}
                                    value={digit}
                                    autoFocus={index === 0}
                                    autoComplete={index === 0 ? "one-time-code" : "off"}
                                    inputRef={(element) => {
                                        inputs.current[index] = element
                                    }}
                                    onChange={(event) => setDigit(index, event.target.value)}
                                    onKeyDown={(event) =>
                                        onKeyDown(index, event as KeyboardEvent<HTMLInputElement>)
                                    }
                                    onPaste={(event) =>
                                        onPaste(event as ClipboardEvent<HTMLInputElement>)
                                    }
                                    slotProps={{
                                        htmlInput: {
                                            inputMode: "numeric",
                                            pattern: "\\d",
                                            maxLength: 1,
                                            "aria-label": msgStr(
                                                "otpDigit",
                                                String(index + 1),
                                                String(length)
                                            ),
                                            style: {textAlign: "center", width: 24},
                                        },
                                    }}
                                />
                            ))}
                        </Box>
                        <input type="hidden" id="code" name="code" value={digits.join("")} />
                        <Button id="kc-form-submit" type="submit" variant="contained" fullWidth>
                            {msgStr("doSubmit")}
                        </Button>
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
                >
                    {remaining > 0
                        ? msgStr(`messageOtp.${flow}.resend.timer`, String(remaining))
                        : msgStr(`messageOtp.${flow}.resend.button`)}
                </Button>
            </Box>
        </Template>
    )
}
