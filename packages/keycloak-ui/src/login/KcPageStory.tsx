// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {DeepPartial} from "keycloakify/tools/DeepPartial"
import {createGetKcContextMock} from "keycloakify/login/KcContext"
import {kcEnvDefaults, themeNames} from "../kc.gen"
import {KEYCLOAK_MESSAGE_OTP} from "@sequentech/ui-test-kit/fixtures/keycloak"
import {
    LoginHintUsernamePolicy,
    LoginValidationPolicy,
    MessageCourier,
    ScanovateSide,
    type KcContext,
    type KcContextExtension,
    type KcContextExtensionPerPage,
} from "./KcContext"
import KcPage from "./KcPage"

// Synthetic values only: no real addresses, codes or credentials.
const kcContextExtension: KcContextExtension = {
    themeName: themeNames[0],
    properties: {...kcEnvDefaults, systemVersion: "0.0.0-preview", systemHash: "synthetic"},
    sequent: {
        loginValidationPolicy: LoginValidationPolicy.Browser,
        loginHintUsernamePolicy: LoginHintUsernamePolicy.Editable,
    },
}
const kcContextExtensionPerPage: KcContextExtensionPerPage = {
    "message-otp.login.ftl": {
        ...KEYCLOAK_MESSAGE_OTP,
        courier: MessageCourier.Email,
    },
    "scanovate-capture.ftl": {
        scanovate: {
            documentType: "driversLicense",
            sides: [ScanovateSide.Front, ScanovateSide.Back],
            videoSeconds: 5,
            attemptsLeft: 3,
            maxAttempts: 3,
            upload: {
                url: "/realms/storybook/identity-verification/capture",
                token: "capture-token",
            },
        },
    },
    "scanovate-error.ftl": {
        error: "scanovateDocumentAuthenticationError",
        canRetry: true,
        code_id: "Q5KWeXSFuWKwRTRuA3R1FkF7",
        attemptsLeft: 2,
    },
    "scanovate-confirmation.ftl": {
        documentType: "driversLicense",
        storedAttributes: [
            {key: "firstName", value: "JUAN SANTOS", type: "text"},
            {key: "lastName", value: "DELA CRUZ", type: "text"},
            {key: "dateOfBirth", value: "1990-01-01", type: "date"},
            {key: "sequent.read-only.id-card-number", value: "N01-23-456789", type: "text"},
        ],
    },
}

export const {getKcContextMock} = createGetKcContextMock({
    kcContextExtension,
    kcContextExtensionPerPage,
    overrides: {},
    overridesPerPage: {},
})

export function createKcPageStory<PageId extends KcContext["pageId"]>(params: {pageId: PageId}) {
    const {pageId} = params

    function KcPageStory(props: {
        locale?: "en" | "es"
        kcContext?: DeepPartial<Extract<KcContext, {pageId: PageId}>>
    }) {
        const kcContextMock = getKcContextMock({pageId, overrides: props.kcContext})
        if (props.locale && kcContextMock.locale) {
            kcContextMock.locale.currentLanguageTag = props.locale
        }
        return <KcPage kcContext={kcContextMock} />
    }

    return {KcPageStory}
}
