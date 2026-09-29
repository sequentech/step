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

export enum ScanovateSide {
    Front = "FRONT",
    Back = "BACK",
}

// The Liveness Plus iframe, when it checks the voter's face instead of our camera.
export type ScanovateLiveness = {
    url: string
    origin: string
    languages: string[]
}

export type ScanovateCaptureSettings = {
    documentType: string
    sides: ScanovateSide[]
    videoSeconds: number
    attemptsLeft: number
    maxAttempts: number
    liveness?: ScanovateLiveness
}

export type ScanovateStoredAttribute = {
    key: string
    value: string
    type: string
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
}

export type KcContext = ExtendKcContext<KcContextExtension, KcContextExtensionPerPage>
