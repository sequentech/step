// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useState} from "react"
import Checkbox from "@mui/material/Checkbox"
import FormControlLabel from "@mui/material/FormControlLabel"
import ChannelIcon from "../../../../ui-essentials/src/components/ChannelIcon/ChannelIcon"
import {messageLanguage} from "../i18n"
import {MessageChannel, inChannelOrder, type KcContext} from "../KcContext"
import {FieldError, RequiredMark, optionLabel, type ProfileForm} from "./ProfileFields"

type Register = Extract<KcContext, {pageId: "register.ftl"}>

// The enrollment form's fields for the channel, as voter-enrollment reads them.
export const CHANNEL_FIELD = "sequent.otp-channel"
export const CONSENT_FIELD = "sequent.message-consent"
const NUMBER_FIELDS: Partial<Record<MessageChannel, string>> = {
    [MessageChannel.WhatsApp]: "sequent.whatsapp-number",
    [MessageChannel.Viber]: "sequent.viber-number",
}

/** Whether the flow asks the voter how to get codes: it then sends the channels it offers. */
export function asksForChannel(kcContext: Register): boolean {
    return (
        kcContext.messagingChannels !== undefined || kcContext.messagingChannelsByPost !== undefined
    )
}

/**
 * The channels the selected Post offers, in the ticket's order. Before a Post is chosen, or for one
 * the page has no list for, the channels Keycloak sent for the submitted Post.
 */
export function postChannels(kcContext: Register, post: string | undefined): MessageChannel[] {
    const byPost = kcContext.messagingChannelsByPost
    if (post !== undefined && post !== "" && byPost?.[post] !== undefined) {
        return inChannelOrder(byPost[post])
    }
    return inChannelOrder(kcContext.messagingChannels)
}

/** Every channel some Post offers: a Post that offers fewer says so. */
function eventChannels(kcContext: Register): MessageChannel[] {
    return inChannelOrder([
        ...Object.values(kcContext.messagingChannelsByPost ?? {}).flat(),
        ...(kcContext.messagingChannels ?? []),
    ])
}

// What the voter submitted, when Keycloak shows the form again with errors.
function submitted(kcContext: Register, field: string): string | undefined {
    return kcContext.messagingFormData?.[field]
}

/**
 * "Get codes and updates by": one channel per voter, from the channels the voter's Post offers.
 * The list follows the Post field. Only the chosen channel's number and consent are on the form,
 * so nothing typed for another channel is ever sent. There is no notification preference: codes
 * and notices go to the one channel.
 */
export default function ChannelChoice(props: {form: ProfileForm}) {
    const {form} = props
    const {kcContext, i18n} = form
    const {msg, msgStr} = i18n
    const {messagesPerField} = kcContext
    const postAttribute = kcContext.messagingPostAttribute
    const post = postAttribute === undefined ? undefined : form.values[postAttribute]?.[0]
    const channels = postChannels(kcContext, post)
    const [selected, setSelected] = useState<string | undefined>(() =>
        submitted(kcContext, CHANNEL_FIELD)
    )
    // A channel the new Post doesn't offer is no longer chosen.
    const current =
        channels.find((channel) => channel === selected) ??
        (channels.length === 1 ? channels[0] : undefined)
    const postDefinition =
        postAttribute === undefined
            ? undefined
            : form.attributes.find((attribute) => attribute.name === postAttribute)
    const postName =
        post === undefined || post === ""
            ? undefined
            : postDefinition === undefined
              ? post
              : optionLabel(postDefinition, post, i18n)
    const fewer = postName !== undefined && channels.length < eventChannels(kcContext).length
    const channelInvalid = messagesPerField.existsError(CHANNEL_FIELD)
    const organization = kcContext.messagingOrganization ?? kcContext.realm.displayName ?? ""
    const label = (channel: MessageChannel) => msgStr(`messageChannel.${channel}`)

    return (
        <div id="messaging-channel-choice">
            <span
                id="channel-choice-label"
                className="auth-field-label"
                lang={messageLanguage(kcContext, i18n, "messaging.channelChoice.label")}
            >
                {msg("messaging.channelChoice.label")}
                <RequiredMark />
            </span>
            {fewer && (
                <div
                    id="channel-choice-help"
                    className="auth-helper"
                    lang={messageLanguage(kcContext, i18n, "messaging.channelChoice.help")}
                >
                    {msg("messaging.channelChoice.help", postName ?? "")}
                </div>
            )}
            <div
                role="radiogroup"
                className="auth-channel-choice"
                aria-labelledby="channel-choice-label"
                aria-describedby={
                    [fewer && "channel-choice-help", channelInvalid && "input-error-otp-channel"]
                        .filter(Boolean)
                        .join(" ") || undefined
                }
                aria-invalid={channelInvalid || undefined}
                aria-required="true"
            >
                {channels.map((channel) => (
                    <label key={channel} className="auth-channel-option">
                        <input
                            type="radio"
                            id={`otp-channel-${channel}`}
                            name={CHANNEL_FIELD}
                            value={channel}
                            checked={current === channel}
                            required
                            onChange={() => setSelected(channel)}
                        />
                        <ChannelIcon channel={channel} />
                        <span>{label(channel)}</span>
                    </label>
                ))}
            </div>
            {channelInvalid && (
                <FieldError
                    id="input-error-otp-channel"
                    html={messagesPerField.get(CHANNEL_FIELD)}
                />
            )}
            {current !== undefined && (
                <ChannelDetails
                    key={current}
                    form={form}
                    channel={current}
                    organization={organization}
                />
            )}
        </div>
    )
}

/** The chosen channel's number, its Messenger help and the consent that names it. */
function ChannelDetails(props: {form: ProfileForm; channel: MessageChannel; organization: string}) {
    const {form, channel, organization} = props
    const {kcContext, i18n} = form
    const {msg, msgStr} = i18n
    const {messagesPerField} = kcContext
    const numberField = NUMBER_FIELDS[channel]
    const numberInvalid = numberField !== undefined && messagesPerField.existsError(numberField)
    const consentInvalid = messagesPerField.existsError(CONSENT_FIELD)
    const consentVersion = kcContext.messagingConsentVersion ?? ""
    const consentValue = `${consentVersion}:${channel}`
    const [consent, setConsent] = useState(
        () => submitted(kcContext, CONSENT_FIELD) === consentValue
    )
    const channelLabel = msgStr(`messageChannel.${channel}`)
    return (
        <div className="auth-channel-details" data-messaging-for={channel}>
            {numberField !== undefined && (
                <div>
                    <label htmlFor={numberField} className="auth-field-label">
                        {msg(
                            channel === MessageChannel.WhatsApp
                                ? "messaging.number.WHATSAPP"
                                : "messaging.number.VIBER"
                        )}
                        <RequiredMark />
                    </label>
                    <input
                        type="tel"
                        id={numberField}
                        name={numberField}
                        className="auth-input"
                        autoComplete="tel"
                        inputMode="tel"
                        placeholder="+"
                        pattern="\+[1-9][0-9]{6,14}"
                        required
                        defaultValue={submitted(kcContext, numberField) ?? ""}
                        aria-invalid={numberInvalid || undefined}
                        aria-describedby={[
                            `${numberField}-help`,
                            numberInvalid && `input-error-${numberField}`,
                        ]
                            .filter(Boolean)
                            .join(" ")}
                    />
                    {numberInvalid && (
                        <FieldError
                            id={`input-error-${numberField}`}
                            html={messagesPerField.get(numberField)}
                        />
                    )}
                    <div
                        id={`${numberField}-help`}
                        className="auth-helper after"
                        lang={messageLanguage(kcContext, i18n, "messaging.number.help")}
                    >
                        {msg("messaging.number.help", organization)}
                    </div>
                </div>
            )}
            {channel === MessageChannel.Messenger && kcContext.messagingPage !== undefined && (
                <div
                    id="messenger-connect-help"
                    className="auth-helper"
                    lang={messageLanguage(kcContext, i18n, "messageOtp.messenger.connectHelp")}
                >
                    <strong>{msg("messageOtp.messenger.connect")}</strong>{" "}
                    {msg("messageOtp.messenger.connectHelp", kcContext.messagingPage)}
                </div>
            )}
            <div>
                <FormControlLabel
                    lang={messageLanguage(kcContext, i18n, "messaging.consent")}
                    label={msgStr("messaging.consent", organization, channelLabel)}
                    control={
                        <Checkbox
                            id={`message-consent-${channel}`}
                            name={CONSENT_FIELD}
                            value={consentValue}
                            checked={consent}
                            onChange={(event) => setConsent(event.target.checked)}
                            slotProps={{
                                input: {
                                    required: true,
                                    "aria-invalid": consentInvalid || undefined,
                                    "aria-describedby": consentInvalid
                                        ? "input-error-message-consent"
                                        : undefined,
                                },
                            }}
                        />
                    }
                />
                {consentInvalid && (
                    <FieldError
                        id="input-error-message-consent"
                        html={messagesPerField.get(CONSENT_FIELD)}
                    />
                )}
            </div>
        </div>
    )
}
