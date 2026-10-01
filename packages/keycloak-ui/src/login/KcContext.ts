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
}

export type KcContext = ExtendKcContext<KcContextExtension, KcContextExtensionPerPage>
