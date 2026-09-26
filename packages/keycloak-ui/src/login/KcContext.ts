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
    properties: Record<KcEnvName, string>
    sequent: {
        loginValidationPolicy: LoginValidationPolicy
        loginHintUsernamePolicy: LoginHintUsernamePolicy
    }
}

// context.ftl serializes the authenticator's Java enum as its wire value.
export type KcContextExtensionPerPage = {
    "message-otp.login.ftl": {
        address: string
        courier: MessageCourier
        isOtl: boolean
        codeJustSent?: boolean
        resendTimer?: string
        ttl?: string
        codeLength?: string
    }
}

export type KcContext = ExtendKcContext<KcContextExtension, KcContextExtensionPerPage>
