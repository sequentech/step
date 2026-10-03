// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent} from "react"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import FormControlLabel from "@mui/material/FormControlLabel"
import Radio from "@mui/material/Radio"
import RadioGroup from "@mui/material/RadioGroup"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import {QRCodeSVG} from "qrcode.react"
import {messageLanguage, type I18n} from "../i18n"
import {
    DeliveryState,
    MessageChannel,
    MessageCourier,
    MessengerLinkState,
    OtpView,
    type KcContext,
} from "../KcContext"
import {ArrowIcon} from "../icons"

type OtpContext = Extract<KcContext, {pageId: "message-otp.login.ftl"}>

const DEFAULT_CODE_LENGTH = 6
const DEFAULT_RESEND_SECONDS = 60
const DEFAULT_CODE_LIFETIME_SECONDS = 300
// Shared with message-otp.login.ftl, so switching themes keeps the countdown.
const RESEND_END_KEY = "resendOtpEndTime"

type Instruction = "Sms" | "Email" | "Both"

function instructionFor(
    courier: MessageCourier,
    channel: MessageChannel | undefined
): Instruction | undefined {
    switch (courier) {
        case MessageCourier.Sms:
            return "Sms"
        case MessageCourier.Email:
            return "Email"
        case MessageCourier.Both:
        case MessageCourier.None:
            return "Both"
        case MessageCourier.Chosen:
            if (channel === MessageChannel.Email) return "Email"
            if (channel === MessageChannel.Sms) return "Sms"
            return undefined
    }
}

const channelLabelKey = (channel: MessageChannel) => `messageChannel.${channel}` as const

// How often the page asks by itself whether the Messenger chat was opened.
const MESSENGER_POLL_SECONDS = 5
// Shared with messenger-status-poll.ftl: when the link of the current code was first shown.
const MESSENGER_POLL_START_KEY = "messengerPollStart:"

/** A code that was not sent starts no countdown, and ends the one of the code before it. */
function useResendCountdown(seconds: number, codeJustSent: boolean, sendFailed: boolean): number {
    const [remaining, setRemaining] = useState(() => {
        if (sendFailed) {
            localStorage.removeItem(RESEND_END_KEY)
        } else if (codeJustSent) {
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

/** Asks for a code on one of the voter's channels; a form of its own, so Enter submits codes. */
function ChannelForm(props: {
    id: string
    labelledBy: string
    action: string
    channels: MessageChannel[]
    addresses: Partial<Record<MessageChannel, string>>
    disabled: boolean
    i18n: I18n
    kcContext: OtpContext
}) {
    const {id, labelledBy, action, channels, addresses, disabled, i18n, kcContext} = props
    const {msgStr} = i18n
    const [selected, setSelected] = useState<MessageChannel | undefined>(channels[0])
    const optionLabel = (channel: MessageChannel) => {
        const address = addresses[channel]
        return address
            ? msgStr("messageOtp.choose.option", msgStr(channelLabelKey(channel)), address)
            : msgStr(channelLabelKey(channel))
    }
    return (
        <Box component="form" id={id} action={action} method="post" className="auth-form">
            <RadioGroup
                name="channel"
                value={selected ?? ""}
                onChange={(event) => setSelected(event.target.value as MessageChannel)}
                aria-labelledby={labelledBy}
            >
                {channels.map((channel) => (
                    <FormControlLabel
                        key={channel}
                        value={channel}
                        control={<Radio />}
                        label={optionLabel(channel)}
                        lang={messageLanguage(kcContext, i18n, "messageOtp.choose.option")}
                    />
                ))}
            </RadioGroup>
            <Button
                type="submit"
                variant="contained"
                fullWidth
                disabled={disabled || selected === undefined}
                className="auth-submit"
                endIcon={<ArrowIcon />}
                lang={messageLanguage(kcContext, i18n, "messageOtp.otherWay.send")}
            >
                {msgStr("messageOtp.otherWay.send")}
            </Button>
        </Box>
    )
}

/** When checking by itself ends: the code's lifetime after its link was first shown. */
function messengerPollDeadline(link: string, lifetimeSeconds: number): number {
    const key = MESSENGER_POLL_START_KEY + link
    let started = Number(sessionStorage.getItem(key) ?? 0)
    if (started === 0) {
        started = Date.now()
        sessionStorage.setItem(key, String(started))
    }
    return started + lifetimeSeconds * 1000
}

/**
 * Asks every few seconds whether the voter opened the Messenger chat, so the page moves on once
 * the code was sent. It waits while the tab is hidden or the voter is typing a code, and stops
 * when the code's lifetime is over; "Check again" stays for the voter.
 */
function MessengerStatusPoll(props: {
    action: string
    link: string
    lifetimeSeconds: number
    typing: boolean
}) {
    const {action, link, lifetimeSeconds, typing} = props
    const form = useRef<HTMLFormElement | null>(null)
    const paused = useRef(typing)
    const [deadline] = useState(() => messengerPollDeadline(link, lifetimeSeconds))
    const [active, setActive] = useState(() => Date.now() < deadline)
    useEffect(() => {
        paused.current = typing
    }, [typing])
    useEffect(() => {
        if (!active) {
            return
        }
        const interval = MESSENGER_POLL_SECONDS * 1000
        const shown = Date.now()
        const check = () => {
            if (Date.now() >= deadline) {
                setActive(false)
            } else if (!document.hidden && !paused.current) {
                form.current?.submit()
            }
        }
        const onVisible = () => {
            if (Date.now() - shown >= interval) {
                check()
            }
        }
        const timer = setInterval(check, interval)
        document.addEventListener("visibilitychange", onVisible)
        return () => {
            clearInterval(timer)
            document.removeEventListener("visibilitychange", onVisible)
        }
    }, [active, deadline])
    if (!active) {
        return null
    }
    return (
        <form ref={form} id="messenger-poll" action={action} method="post" hidden>
            <input type="hidden" name="messengerStatus" value="true" />
        </form>
    )
}

function MessengerConnect(props: {kcContext: OtpContext; i18n: I18n}) {
    const {kcContext, i18n} = props
    const {messengerPage, messengerLink, messengerWord, messengerState} = kcContext
    const {msg, msgStr} = i18n
    const expired =
        messengerState === MessengerLinkState.Expired ||
        messengerState === MessengerLinkState.Replaced
    return (
        <Box
            component="section"
            className="auth-messenger"
            aria-labelledby="messenger-title"
            lang={messageLanguage(kcContext, i18n, "messageOtp.messenger.title")}
        >
            <Typography id="messenger-title" component="h2" className="auth-field-label">
                {msg("messageOtp.messenger.title")}
            </Typography>
            {messengerPage && (
                <Typography>{msg("messageOtp.messenger.intro", messengerPage)}</Typography>
            )}
            <Box component="ol" className="auth-messenger-steps">
                <li>{msg("messageOtp.messenger.step1")}</li>
                <li>{msg("messageOtp.messenger.step2")}</li>
                <li>{msg("messageOtp.messenger.step3")}</li>
            </Box>
            {messengerLink && !expired && (
                <>
                    <Button
                        id="messenger-connect"
                        href={messengerLink}
                        target="_blank"
                        rel="noopener noreferrer"
                        variant="contained"
                        fullWidth
                    >
                        {msgStr("messageOtp.messenger.connect")}
                    </Button>
                    <Box
                        className="auth-messenger-qr"
                        sx={{display: {xs: "none", md: "flex"}}}
                        role="img"
                        aria-label={msgStr("messageOtp.messenger.scan")}
                    >
                        <QRCodeSVG value={messengerLink} size={160} />
                        <Typography variant="body2" aria-hidden="true">
                            {msg("messageOtp.messenger.scan")}
                        </Typography>
                    </Box>
                </>
            )}
            {messengerWord && messengerPage && !expired && (
                <Typography variant="body2">
                    {msg("messageOtp.messenger.word", messengerWord, messengerPage)}
                </Typography>
            )}
            {expired ? (
                <Typography role="alert">{msg("messageOtp.messenger.expired")}</Typography>
            ) : messengerState === MessengerLinkState.CodeSent ? (
                <Typography role="status">{msg("messageOtp.messenger.codeSent")}</Typography>
            ) : (
                messengerState === MessengerLinkState.Pending && (
                    <Typography role="status">{msg("messageOtp.messenger.pending")}</Typography>
                )
            )}
        </Box>
    )
}

export default function MessageOtpLogin(props: PageProps<OtpContext, I18n>) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {
        url,
        address,
        courier,
        isOtl,
        codeJustSent,
        resendTimer,
        ttl,
        codeLength,
        otpView,
        channel,
        otherWayChannels,
        channelAddresses,
        deliveryState,
        senderLabel,
    } = kcContext
    const {msg, msgStr} = i18n
    const flow = isOtl ? "otl" : "auth"
    const length = Number(codeLength ?? DEFAULT_CODE_LENGTH)
    const [digits, setDigits] = useState<string[]>(() => Array(length).fill(""))
    const inputs = useRef<(HTMLInputElement | null)[]>([])
    const submit = useRef<HTMLButtonElement | null>(null)
    const codeError = kcContext.message?.type === "error"
    const unconfirmed = deliveryState === DeliveryState.Unknown
    const failed = deliveryState === DeliveryState.Failed
    const [showOtherWay, setShowOtherWay] = useState(unconfirmed || failed)
    const remaining = useResendCountdown(
        Number(resendTimer ?? DEFAULT_RESEND_SECONDS),
        codeJustSent === true,
        failed
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
    const instruction = instructionFor(courier ?? MessageCourier.Both, channel)
    const addresses = channelAddresses ?? {}

    if (otpView === OtpView.Choose) {
        return (
            <Template
                kcContext={kcContext}
                i18n={i18n}
                doUseDefaultCss={doUseDefaultCss}
                classes={classes}
                headerNode={msg("messageOtp.choose.title")}
            >
                <Typography
                    id="otp-choose-title"
                    className="auth-instructions"
                    lang={messageLanguage(kcContext, i18n, "messageOtp.choose.help")}
                >
                    {msg("messageOtp.choose.help")}
                </Typography>
                <ChannelForm
                    id="otp-choose"
                    labelledBy="otp-choose-title"
                    action={url.loginAction}
                    channels={otherWayChannels ?? []}
                    addresses={addresses}
                    disabled={false}
                    i18n={i18n}
                    kcContext={kcContext}
                />
            </Template>
        )
    }

    const channelLabel = channel ? msgStr(channelLabelKey(channel)) : ""
    const deliveryLine = (() => {
        if (unconfirmed) {
            return (
                <Typography id="otp-delivery" role="status" className="auth-address">
                    {msg("messageOtp.delivery.unknown")}
                </Typography>
            )
        }
        if (failed && channel) {
            return (
                <Typography id="otp-delivery" role="alert" className="auth-address">
                    {msg("messageOtp.delivery.failed", channelLabel)}
                </Typography>
            )
        }
        if (channel === MessageChannel.Messenger) {
            return null
        }
        if (channel) {
            return (
                <Typography
                    className="auth-address"
                    lang={messageLanguage(kcContext, i18n, "messageOtp.auth.sentTo")}
                >
                    {address
                        ? msg("messageOtp.auth.sentTo", channelLabel, address)
                        : msg("messageOtp.auth.sentToChannel", channelLabel)}
                </Typography>
            )
        }
        return (
            <Typography
                className="auth-address"
                lang={messageLanguage(kcContext, i18n, `messageOtp.${flow}.address`)}
            >
                {msg(`messageOtp.${flow}.address`, address)}
            </Typography>
        )
    })()
    const appInstruction =
        senderLabel !== undefined &&
        (channel === MessageChannel.WhatsApp || channel === MessageChannel.Viber) &&
        !unconfirmed &&
        !failed

    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            displayInfo={ttl !== undefined}
            headerNode={msg(`messageOtp.${flow}.title`)}
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
            {deliveryLine}
            {appInstruction ? (
                <Typography
                    id="otp-instructions"
                    className="auth-instructions"
                    lang={messageLanguage(kcContext, i18n, "messageOtp.auth.openApp")}
                >
                    {msg("messageOtp.auth.openApp", channelLabel, senderLabel)}
                </Typography>
            ) : (
                instruction !== undefined && (
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
                )
            )}
            {channel === MessageChannel.Messenger && (
                <MessengerConnect kcContext={kcContext} i18n={i18n} />
            )}
            {channel === MessageChannel.Messenger &&
                kcContext.messengerState === MessengerLinkState.Pending &&
                kcContext.messengerLink !== undefined && (
                    <MessengerStatusPoll
                        action={url.loginAction}
                        link={kcContext.messengerLink}
                        lifetimeSeconds={Number(ttl ?? DEFAULT_CODE_LIFETIME_SECONDS)}
                        typing={digits.some((digit) => digit !== "")}
                    />
                )}
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
                                (appInstruction || instruction !== undefined) && "otp-instructions",
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
                {channel === MessageChannel.Messenger && (
                    <Button
                        id="messenger-check"
                        type="submit"
                        name="messengerStatus"
                        value="true"
                        formNoValidate
                        variant="text"
                        lang={messageLanguage(kcContext, i18n, "messageOtp.messenger.check")}
                    >
                        {msgStr("messageOtp.messenger.check")}
                    </Button>
                )}
            </Box>
            {otherWayChannels !== undefined && otherWayChannels.length > 0 && (
                <Box component="section" className="auth-other-way">
                    <Button
                        id="otp-other-way-title"
                        variant="text"
                        aria-expanded={showOtherWay}
                        aria-controls="otp-other-way"
                        onClick={() => setShowOtherWay(!showOtherWay)}
                        lang={messageLanguage(kcContext, i18n, "messageOtp.otherWay.title")}
                    >
                        {msgStr("messageOtp.otherWay.title")}
                    </Button>
                    {showOtherWay && (
                        <Box id="otp-other-way">
                            <Typography
                                className="auth-instructions"
                                lang={messageLanguage(kcContext, i18n, "messageOtp.otherWay.help")}
                            >
                                {msg("messageOtp.otherWay.help")}
                            </Typography>
                            <ChannelForm
                                id="otp-other-way-form"
                                labelledBy="otp-other-way-title"
                                action={url.loginAction}
                                channels={otherWayChannels}
                                addresses={addresses}
                                disabled={remaining > 0}
                                i18n={i18n}
                                kcContext={kcContext}
                            />
                        </Box>
                    )}
                </Box>
            )}
        </Template>
    )
}
