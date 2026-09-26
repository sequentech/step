// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {i18nBuilder} from "keycloakify/login"
import type {ThemeName} from "../kc.gen"

// Keycloakify resolves messages in the browser: keys that the server-side
// bundles define (sequent-theme, message-otp-authenticator's theme-resources)
// have to be repeated here.
const {useI18n, ofTypeI18n} = i18nBuilder
    .withThemeName<ThemeName>()
    .withCustomTranslations({
        en: {
            loginAccountTitle: "Login to the Admin Portal",
            doLogIn: "LOGIN",
            invalidCredentialsMessage: "The details you entered are incorrect.",
            "messageOtp.auth.address": "OTP (One Time Password) was sent to {0}",
            "messageOtp.auth.instructionBoth":
                "Enter the code we sent to to your mobile device via sms or email.",
            "messageOtp.auth.instructionSms":
                "Enter the code we sent to your mobile device via sms.",
            "messageOtp.auth.instructionEmail": "Enter the code we sent to your email.",
            "messageOtp.auth.ttlTime": "The Authenticator Code is valid for {0} minutes.",
            "messageOtp.auth.resend.button": "Didn't receive the Code yet? Click here to resend",
            "messageOtp.auth.resend.timer":
                "Didn't receive the Code yet? Wait {0} seconds to resend",
            "messageOtp.otl.address": "Authentication Link was sent to {0}",
            "messageOtp.otl.instructionBoth":
                "We have sent you a verification link to your mobile device via email and/or SMS. Please open this link to continue.",
            "messageOtp.otl.instructionSms":
                "We have sent you a verification link to your mobile device via SMS. Please open this link to continue.",
            "messageOtp.otl.instructionEmail":
                "We have sent you a verification link to your mobile device via email. Please open this link to continue.",
            "messageOtp.otl.ttlTime": "Authentication link is valid for {0} minutes.",
            "messageOtp.otl.resend.button": "Didn't receive the link yet? Click here to resend",
            "messageOtp.otl.resend.timer":
                "Didn't receive the link yet? Wait {0} seconds to resend",
            otpDigit: "Digit {0} of {1}",
        },
        es: {
            loginAccountTitle: "Iniciar sesión en el Portal de Administración",
            doLogIn: "INICIAR SESIÓN",
            invalidCredentialsMessage: "Los datos introducidos no son correctos.",
            "messageOtp.auth.address": "Su OTP (Código de Autenticación) fue enviado a {0}",
            "messageOtp.auth.instructionBoth":
                "Ingrese el código que le enviamos a su dispositivo móvil por SMS o a su email.",
            "messageOtp.auth.instructionSms":
                "Ingrese el código que le enviamos a su dispositivo móvil por SMS.",
            "messageOtp.auth.instructionEmail": "Ingrese el código que le enviamos a su email.",
            "messageOtp.auth.ttlTime": "El código de autenticación es válido por {0} minutos.",
            "messageOtp.auth.resend.button":
                "¿Aún no recibió el código? Haga clic aquí para reenviar",
            "messageOtp.auth.resend.timer":
                "¿Aún no recibió el código? Espere {0} segundos para reenviar",
            "messageOtp.otl.address": "El enlace de autenticación fue enviado a {0}",
            "messageOtp.otl.instructionBoth":
                "Le hemos enviado un enlace de verificación a su dispositivo móvil por email y/o SMS. Abra este enlace para continuar.",
            "messageOtp.otl.instructionSms":
                "Le hemos enviado un enlace de verificación a su dispositivo móvil por SMS. Abra este enlace para continuar.",
            "messageOtp.otl.instructionEmail":
                "Le hemos enviado un enlace de verificación a su dispositivo móvil por email. Abra este enlace para continuar.",
            "messageOtp.otl.ttlTime": "El enlace de autenticación es válido por {0} minutos.",
            "messageOtp.otl.resend.button":
                "¿Aún no recibió el enlace? Haga clic aquí para reenviar",
            "messageOtp.otl.resend.timer":
                "¿Aún no recibió el enlace? Espere {0} segundos para reenviar",
            otpDigit: "Dígito {0} de {1}",
        },
    })
    .build()

type I18n = typeof ofTypeI18n

export {useI18n, type I18n}
