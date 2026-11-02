// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {ExtendKcContext} from "keycloakify/login"
import type {KcEnvName, ThemeName} from "../kc.gen"
import {
    EAudioInstructionsPolicy,
    EVoterAccessibilitySettingsPolicy,
} from "../../../ui-core/src/types/ElectionEventPresentation"

export {EAudioInstructionsPolicy, EVoterAccessibilitySettingsPolicy}

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

// The ticket's order for every list of channels: WhatsApp, Viber, Messenger, SMS, Email.
export const CHANNEL_ORDER: readonly MessageChannel[] = [
    MessageChannel.WhatsApp,
    MessageChannel.Viber,
    MessageChannel.Messenger,
    MessageChannel.Sms,
    MessageChannel.Email,
]

/** The known channels of a list, once each, in the ticket's order. */
export function inChannelOrder(channels: readonly string[] | undefined): MessageChannel[] {
    const given = new Set(channels ?? [])
    return CHANNEL_ORDER.filter((channel) => given.has(channel))
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

// What the code page may say about where the code went. A password reset is NEUTRAL: it never
// tells whether the account exists, so it names no channel or contact.
export enum CodeDisclosure {
    Named = "NAMED",
    Neutral = "NEUTRAL",
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
        voterAccessibilitySettingsPolicy: EVoterAccessibilitySettingsPolicy
        audioInstructionsPolicy: EAudioInstructionsPolicy
        // The realm's loginCustomCss localization text, which the FreeMarker login themes inject too.
        loginCustomCss?: string
    }
}

export enum ScanovateSide {
    Front = "FRONT",
    Back = "BACK",
}

// The Liveness Plus API that checks the voter's face frames, with Keycloak's
// one-time token for it.
export type ScanovateLiveness = {
    url: string
    token: string
    caseId: string
}

// Where the page uploads its captures, with Keycloak's one-time token for it:
// Keycloak can't take files on its login actions URL.
export type ScanovateUpload = {
    url: string
    token: string
}

// Physical format of the document, which the capture guide takes the shape of.
export enum DocumentFormat {
    // ISO/IEC 7810 ID-1 cards: national IDs, driver's licenses.
    Id1 = "ID_1",
    // ICAO 9303 TD3 passport data pages.
    Td3 = "TD3",
}

export type ScanovateCaptureSettings = {
    documentType: string
    // Absent on pages rendered by earlier authenticators: ID-1.
    format?: DocumentFormat
    sides: ScanovateSide[]
    videoSeconds: number
    attemptsLeft: number
    maxAttempts: number
    upload: ScanovateUpload
    liveness: ScanovateLiveness
}

export type ScanovateStoredAttribute = {
    key: string
    value: string
    type: string
}

// How the deferred registration form is used. Keycloak's own registration
// form sends none.
export enum RegistrationFormMode {
    Registration = "REGISTRATION",
    Login = "LOGIN",
}

// The realm's credential-field-position attribute.
export enum CredentialFieldPosition {
    First = "FIRST",
    Last = "LAST",
}

export type SequentRegistration = {
    formMode?: RegistrationFormMode
    credentialFieldPosition: CredentialFieldPosition
    // Profile attributes the authenticator leaves out of the form.
    hiddenAttributes: string[]
    // Attributes prefilled from a read-only login hint: sent, but not editable.
    lockedAttributes: string[]
}

export type EnrollmentMismatch = {
    name: string
    // The voter left it empty.
    value: string | null
}

// Why an enrollment wasn't approved automatically.
export type EnrollmentOutcome = {
    reason?: string
    mismatchedFields: EnrollmentMismatch[]
}

type EnrollmentFinish = {
    enrollmentOutcome?: EnrollmentOutcome
    // Where the result of the enrollment goes: the verified channel and its masked contact.
    noticeChannel?: MessageChannel
    noticeAddress?: string
    // The application's reference for support: 8 capital letters and digits, "XXXX-XXXX".
    supportReference?: string
}

// The channel choice of the enrollment form, when the flow asks for one.
export type MessagingChoice = {
    // The channels of the submitted Post (every restricted election's common channels before one
    // is chosen).
    messagingChannels?: MessageChannel[]
    // Each Post's channels, by the option value of the Post attribute, so the list follows the
    // Post the voter picks.
    messagingChannelsByPost?: Record<string, MessageChannel[]>
    // The profile attribute that names the voter's Post.
    messagingPostAttribute?: string
    messagingOrganization?: string
    messagingConsentVersion?: string
    // The Facebook Page's name, when the Post offers Messenger.
    messagingPage?: string
    // What the voter submitted in these fields, when the form comes back with errors
    // (context.ftl's bridge of register.formData).
    messagingFormData?: Record<string, string | undefined>
}

// context.ftl serializes the authenticator's Java enum as its wire value.
export type KcContextExtensionPerPage = {
    "message-otp.login.ftl": {
        // Absent on a NEUTRAL page.
        address?: string
        courier?: MessageCourier
        // Absent means NAMED.
        codeDisclosure?: CodeDisclosure
        isOtl: boolean
        codeJustSent?: boolean
        // A send was attempted now, delivered or not: the resend countdown starts.
        resendTimerStarted?: boolean
        resendTimer?: string
        ttl?: string
        codeLength?: string
        // Only with the CHOSEN courier: the voter's one channel. There is no other way: the
        // voter changes the channel through the contact-change required action.
        channel?: MessageChannel
        deliveryState?: DeliveryState
        senderLabel?: string
        // The realm's limit of wrong codes was reached: the authentication session is finished,
        // only starting again helps, and a resend never resets the count.
        attemptsExhausted?: boolean
        // A NEUTRAL page whose event offers Messenger for codes: how to connect it, shown for any
        // account (its link binds to nothing for one that doesn't use Messenger).
        messengerConnect?: boolean
        // Who sends the codes, for the Messenger explanation.
        messagingOrganization?: string
        messengerPage?: string
        // The m.me link with the one-time reference, and the same link as a QR image (a data URI)
        // drawn by Keycloak.
        messengerLink?: string
        messengerQr?: string
        messengerState?: MessengerLinkState
        // The event's Messenger Page changed since the voter connected: this sign-in connects
        // Messenger again, and the page says why.
        messengerReconnect?: boolean
        // With the code progress policy, which of the flow's codes this is, from 1.
        codeRequest?: number
        codeRequests?: number
    }
    "scanovate-capture.ftl": {
        scanovate: ScanovateCaptureSettings
    }
    "scanovate-error.ftl": {
        error: string
        canRetry: boolean
        code_id: string
        attemptsLeft?: number
    }
    "scanovate-confirmation.ftl": {
        storedAttributes: ScanovateStoredAttribute[]
        documentType?: string
    }
    "register.ftl": {
        // Absent on pages rendered without context.ftl's bridge.
        sequentRegistration?: SequentRegistration
    } & MessagingChoice
    "registration-finish.ftl": EnrollmentFinish
    "registration-manual-finish.ftl": EnrollmentFinish
    "registration-rejected-finish.ftl": EnrollmentFinish
    "message-finish.ftl": EnrollmentFinish
}

export type KcContext = ExtendKcContext<KcContextExtension, KcContextExtensionPerPage>
