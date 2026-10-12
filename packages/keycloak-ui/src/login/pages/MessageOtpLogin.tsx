// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef, useState, type ClipboardEvent, type KeyboardEvent} from "react"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import {QRCodeSVG} from "qrcode.react"
import ChannelIcon from "../../../../ui-essentials/src/components/ChannelIcon/ChannelIcon"
import {messageLanguage, type I18n} from "../i18n"
import {
    CodeDisclosure,
    DeliveryState,
    MessageChannel,
    MessageCourier,
    MessengerLinkState,
    type KcContext,
} from "../KcContext"
import {ArrowIcon} from "../icons"
import type {EnrollmentPageProps} from "../enrollment/pageProps"

type OtpContext = Extract<KcContext, {pageId: "message-otp.login.ftl"}>
type OtpPageProps = EnrollmentPageProps<"message-otp.login.ftl">

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

/** Starts the resend countdown: every send starts it, also one that failed. */
function startResendCountdown(seconds: number) {
    try {
        localStorage.setItem(RESEND_END_KEY, String(Date.now() + seconds * 1000))
    } catch {
        // Without storage the server's own timer still refuses an early resend.
    }
}

function resendSecondsLeft(): number {
    try {
        const end = Number(localStorage.getItem(RESEND_END_KEY) ?? 0)
        return Math.max(Math.ceil((end - Date.now()) / 1000), 0)
    } catch {
        return 0
    }
}

/**
 * The seconds until the voter can ask for the code again. A send that just happened, delivered or
 * not, starts the countdown; a failed send never lifts it, as Keycloak's own timer doesn't.
 */
function useResendCountdown(seconds: number, sendAttempted: boolean): number {
    const [remaining, setRemaining] = useState(() => {
        if (sendAttempted) {
            startResendCountdown(seconds)
        }
        return resendSecondsLeft()
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

// Only an image Keycloak drew is shown; anything else is ignored.
function qrImage(source: string | undefined): string | undefined {
    return source !== undefined && source.startsWith("data:image/") ? source : undefined
}

// Only an absolute https link becomes a link (and a QR code); anything else gives none.
export function httpsLink(link: string | undefined): string | undefined {
    if (link === undefined) {
        return undefined
    }
    try {
        return new URL(link).protocol === "https:" ? link : undefined
    } catch {
        return undefined
    }
}

/**
 * Messenger: the Page may only write to a voter who tapped Get Started, so the page explains that
 * and offers the chat: a QR code to scan on a computer, a button to open it on a phone.
 *
 * On a NEUTRAL page (`generic`) the box is the same for every account: it is headed "If you get
 * your codes in Messenger" and says nothing about the link's state.
 */
function MessengerConnect(props: {kcContext: OtpContext; i18n: I18n; generic?: boolean}) {
    const {kcContext, i18n, generic = false} = props
    const {messengerPage, messengerLink, messagingOrganization} = kcContext
    const messengerState = generic ? undefined : kcContext.messengerState
    const {msg, msgStr} = i18n
    const expired =
        messengerState === MessengerLinkState.Expired ||
        messengerState === MessengerLinkState.Replaced
    const link = expired ? undefined : httpsLink(messengerLink)
    const qr = qrImage(kcContext.messengerQr)
    // The ticket's sentence names who sends the code; an older Keycloak names only the Page.
    const intro = messagingOrganization
        ? {
              key: "messageOtp.messenger.introPage" as const,
              args: [messagingOrganization, messengerPage ?? ""],
          }
        : {key: "messageOtp.messenger.intro" as const, args: [messengerPage ?? ""]}
    return (
        <Box
            component="section"
            className="auth-messenger"
            {...(generic
                ? {
                      "aria-labelledby": "messenger-if",
                      lang: messageLanguage(kcContext, i18n, "messageOtp.messenger.ifMessenger"),
                  }
                : {
                      "aria-label": msgStr("messageOtp.messenger.title"),
                      lang: messageLanguage(kcContext, i18n, "messageOtp.messenger.title"),
                  })}
        >
            {generic && (
                <Typography id="messenger-if" component="h2" className="auth-panel-title">
                    {msg("messageOtp.messenger.ifMessenger")}
                </Typography>
            )}
            {messengerPage && (
                <Typography
                    id="messenger-intro"
                    className="auth-instructions"
                    lang={messageLanguage(kcContext, i18n, intro.key)}
                >
                    {msg(intro.key, ...intro.args)}
                </Typography>
            )}
            {!generic && kcContext.messengerReconnect === true && (
                <Typography
                    id="messenger-reconnect"
                    className="auth-instructions"
                    lang={messageLanguage(kcContext, i18n, "messageOtp.messenger.reconnect")}
                >
                    {msg("messageOtp.messenger.reconnect")}
                </Typography>
            )}
            {link !== undefined && (
                <Box
                    id="messenger-scan"
                    className="auth-messenger-scan"
                    sx={{display: {xs: "none", md: "flex"}}}
                >
                    <Box className="auth-messenger-qr">
                        {qr !== undefined ? (
                            <img
                                src={qr}
                                width={160}
                                height={160}
                                alt={msgStr("messageOtp.messenger.scan")}
                            />
                        ) : (
                            <QRCodeSVG
                                value={link}
                                size={160}
                                role="img"
                                aria-label={msgStr("messageOtp.messenger.scan")}
                            />
                        )}
                    </Box>
                    <Box component="ol" className="auth-messenger-steps">
                        <li>{msg("messageOtp.messenger.scanStep1")}</li>
                        <li>{msg("messageOtp.messenger.scanStep2")}</li>
                        <li>{msg("messageOtp.messenger.scanStep3")}</li>
                    </Box>
                </Box>
            )}
            <Box
                id="messenger-steps"
                component="ol"
                className="auth-messenger-steps"
                sx={link !== undefined ? {display: {xs: "block", md: "none"}} : undefined}
            >
                <li>{msg("messageOtp.messenger.step1")}</li>
                <li>{msg("messageOtp.messenger.step2")}</li>
                <li>{msg("messageOtp.messenger.step3")}</li>
            </Box>
            <Typography variant="body2" className="auth-instructions">
                {msg("messageOtp.messenger.getStarted")}
            </Typography>
            {link !== undefined && (
                <Button
                    id="messenger-connect"
                    href={link}
                    target="_blank"
                    rel="noopener noreferrer"
                    variant="outlined"
                    fullWidth
                    endIcon={<ArrowIcon />}
                >
                    {msgStr("messageOtp.messenger.open")}
                </Button>
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

/** After too many wrong codes the authentication session is over: the voter starts again. */
function TooManyAttempts(props: OtpPageProps & {flow: "otl" | "auth"}) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes, flow} = props
    const {msg, msgStr} = i18n
    const channel = isNeutral(kcContext) ? undefined : kcContext.channel
    return (
        <Template
            kcContext={kcContext}
            i18n={i18n}
            doUseDefaultCss={doUseDefaultCss}
            classes={classes}
            displayMessage={false}
            symbol={channel ? <ChannelIcon channel={channel} /> : undefined}
            headerNode={msg(`messageOtp.${flow}.title`)}
        >
            <Typography
                id="otp-exhausted"
                role="alert"
                className="auth-instructions"
                lang={messageLanguage(kcContext, i18n, "messageOtp.auth.tooManyAttempts")}
            >
                {msg("messageOtp.auth.tooManyAttempts")}
            </Typography>
            <Box className="auth-actions">
                <Button
                    id="otp-restart"
                    href={kcContext.url.loginRestartFlowUrl}
                    variant="contained"
                    fullWidth
                    className="auth-submit"
                    endIcon={<ArrowIcon />}
                    lang={messageLanguage(kcContext, i18n, "messageOtp.auth.restart")}
                >
                    {msgStr("messageOtp.auth.restart")}
                </Button>
            </Box>
        </Template>
    )
}

// A NEUTRAL page (a password reset) never tells whether the account exists: it names no channel
// or contact, whatever else the page data holds.
function isNeutral(kcContext: OtpContext): boolean {
    return kcContext.codeDisclosure === CodeDisclosure.Neutral
}

export default function MessageOtpLogin(props: OtpPageProps) {
    const {kcContext, i18n, Template, doUseDefaultCss, classes} = props
    const {url, courier, isOtl, codeJustSent, resendTimer, ttl, codeLength} = kcContext
    const neutral = isNeutral(kcContext)
    const {address, channel, deliveryState, senderLabel} = neutral
        ? {address: undefined, channel: undefined, deliveryState: undefined, senderLabel: undefined}
        : kcContext
    const {codeRequest, codeRequests} = kcContext
    const {msg, msgStr} = i18n
    const flow = isOtl ? "otl" : "auth"
    const length = Number(codeLength ?? DEFAULT_CODE_LENGTH)
    const [digits, setDigits] = useState<string[]>(() => Array(length).fill(""))
    const inputs = useRef<(HTMLInputElement | null)[]>([])
    const submit = useRef<HTMLButtonElement | null>(null)
    const codeError = kcContext.message?.type === "error"
    const unconfirmed = deliveryState === DeliveryState.Unknown
    const failed = deliveryState === DeliveryState.Failed
    const resendSeconds = Number(resendTimer ?? DEFAULT_RESEND_SECONDS)
    const remaining = useResendCountdown(
        resendSeconds,
        kcContext.resendTimerStarted === true || codeJustSent === true
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
    // The courier's instruction names SMS or email: a neutral page has its own line instead.
    const instruction = neutral
        ? undefined
        : instructionFor(courier ?? MessageCourier.Both, channel)

    if (kcContext.attemptsExhausted === true) {
        return <TooManyAttempts {...props} flow={flow} />
    }

    const channelLabel = channel ? msgStr(channelLabelKey(channel)) : ""
    const deliveryLine = (() => {
        if (neutral) {
            return (
                <Typography
                    id="otp-neutral"
                    className="auth-address"
                    lang={messageLanguage(kcContext, i18n, "messageOtp.auth.neutralSent")}
                >
                    {msg("messageOtp.auth.neutralSent")}
                </Typography>
            )
        }
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
                {msg(`messageOtp.${flow}.address`, address ?? "")}
            </Typography>
        )
    })()
    const appInstruction =
        senderLabel !== undefined &&
        (channel === MessageChannel.WhatsApp || channel === MessageChannel.Viber) &&
        !unconfirmed &&
        !failed
    // Several codes in a row: says which one this is, so the next doesn't look like a retry.
    const counted = codeRequest !== undefined && codeRequests !== undefined && codeRequests > 1
    const codeLabel = {
        text: msgStr("messageOtp.auth.codeProgress", String(codeRequest), String(codeRequests)),
        lang: messageLanguage(kcContext, i18n, "messageOtp.auth.codeProgress"),
    }
    const messenger = channel === MessageChannel.Messenger
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
            symbol={channel ? <ChannelIcon channel={channel} /> : undefined}
            {...(messenger
                ? {
                      headerNode: msg("messageOtp.messenger.title"),
                      titleLang: messageLanguage(kcContext, i18n, "messageOtp.messenger.title"),
                  }
                : {headerNode: msg(`messageOtp.${flow}.title`)})}
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
            {deliveryLine}
            {counted && (
                <Typography
                    id="otp-progress"
                    className="auth-lead"
                    lang={messageLanguage(kcContext, i18n, codeNote.key)}
                >
                    {msgStr(codeNote.key, ...codeNote.args)}
                </Typography>
            )}
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
            {neutral && kcContext.messengerConnect === true && (
                <MessengerConnect kcContext={kcContext} i18n={i18n} generic />
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
                    onClick={() => startResendCountdown(resendSeconds)}
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
        </Template>
    )
}
