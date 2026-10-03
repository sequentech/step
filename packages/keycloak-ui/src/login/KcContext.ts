// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {ExtendKcContext} from "keycloakify/login"
import type {KcEnvName, ThemeName} from "../kc.gen"

export enum MessageCourier {
    Sms = "SMS",
    Email = "EMAIL",
    Both = "BOTH",
    None = "NONE",
    Chosen = "CHOSEN",
}

export enum MessageChannel {
    Email = "EMAIL",
    Sms = "SMS",
    WhatsApp = "WHATSAPP",
    Viber = "VIBER",
    Messenger = "MESSENGER",
}

export enum OtpView {
    Code = "CODE",
    Choose = "CHOOSE",
}

export enum DeliveryState {
    Queued = "QUEUED",
    Accepted = "ACCEPTED",
    Delivered = "DELIVERED",
    Failed = "FAILED",
    Unknown = "UNKNOWN",
}

export enum MessengerLinkState {
    Pending = "PENDING",
    CodeSent = "CODE_SENT",
    Confirmed = "CONFIRMED",
    Expired = "EXPIRED",
    Replaced = "REPLACED",
}

export enum LoginValidationPolicy {
    Browser = "BROWSER",
    ServerOnly = "SERVER_ONLY",
}

export enum LoginHintUsernamePolicy {
    Editable = "EDITABLE",
    ReadOnly = "READ_ONLY",
}

export type KcContextExtension = {
    themeName: ThemeName
    properties: Record<KcEnvName, string> & {
        systemVersion?: string
        systemHash?: string
    }
    sequent: {
        loginValidationPolicy: LoginValidationPolicy
        loginHintUsernamePolicy: LoginHintUsernamePolicy
    }
}

// context.ftl serializes the authenticator's Java enum as its wire value.
export type KcContextExtensionPerPage = {
    "message-otp.login.ftl": {
        address: string
        courier?: MessageCourier
        isOtl: boolean
        codeJustSent?: boolean
        resendTimer?: string
        ttl?: string
        codeLength?: string
        // Only with the CHOSEN courier: the channel in use and the voter's other channels.
        otpView?: OtpView
        channel?: MessageChannel
        otherWayChannels?: MessageChannel[]
        channelAddresses?: Partial<Record<MessageChannel, string>>
        deliveryState?: DeliveryState
        senderLabel?: string
        messengerPage?: string
        messengerLink?: string
        messengerWord?: string
        messengerState?: MessengerLinkState
    }
}

export type KcContext = ExtendKcContext<KcContextExtension, KcContextExtensionPerPage>
