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
        voterAccessibilitySettingsPolicy: EVoterAccessibilitySettingsPolicy
        audioInstructionsPolicy: EAudioInstructionsPolicy
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
    }
    "registration-finish.ftl": EnrollmentFinish
    "registration-manual-finish.ftl": EnrollmentFinish
    "registration-rejected-finish.ftl": EnrollmentFinish
    "message-finish.ftl": EnrollmentFinish
}

export type KcContext = ExtendKcContext<KcContextExtension, KcContextExtensionPerPage>
