// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {i18nBuilder} from "keycloakify/login"
import type {ThemeName} from "../kc.gen"
import type {KcContext} from "./KcContext"

const englishMessages = {
    loginAccountTitle: {
        "sequent-ui-admin": "Sign in to continue",
        "sequent-ui-voting": "Sign in to vote",
        "sequent-ui-architect": "Sign in to Election Architect",
    },
    doContinue: "Continue",
    "identity-provider-login-label": "Or sign in with",
    continueWithProvider: "Continue with {0}",
    doLogIn: "LOGIN",
    "system.version": "Version:",
    "system.hash": "Hash:",
    invalidCredentialsMessage: "The details you entered are incorrect.",
    "messageOtp.auth.address": "We sent a code to {0}.",
    "messageOtp.auth.title": "Enter your verification code",
    "messageOtp.auth.instructionBoth":
        "Enter the code we sent to your mobile device via sms or email.",
    "messageOtp.auth.instructionSms": "Enter the code we sent to your mobile device via sms.",
    "messageOtp.auth.instructionEmail": "Enter the code we sent to your email.",
    "messageOtp.auth.ttlTime": "Code valid for {0} minutes.",
    "messageOtp.auth.resend.button": "Resend code",
    "messageOtp.auth.resend.timer": "Resend code in {0} seconds",
    "messageOtp.otl.address": "We sent a sign-in link to {0}.",
    "messageOtp.otl.title": "Check your messages",
    "messageOtp.otl.instructionBoth":
        "We have sent you a verification link to your mobile device via email and/or SMS. Please open this link to continue.",
    "messageOtp.otl.instructionSms":
        "We have sent you a verification link to your mobile device via SMS. Please open this link to continue.",
    "messageOtp.otl.instructionEmail":
        "We have sent you a verification link to your mobile device via email. Please open this link to continue.",
    "messageOtp.otl.ttlTime": "Link valid for {0} minutes.",
    "messageOtp.otl.resend.button": "Resend link",
    "messageOtp.otl.resend.timer": "Resend link in {0} seconds",
    otpDigit: "Digit {0} of {1}",
    otpCodeLabel: "Verification code",
    "messageChannel.EMAIL": "Email",
    "messageChannel.SMS": "SMS",
    "messageChannel.WHATSAPP": "WhatsApp",
    "messageChannel.VIBER": "Viber",
    "messageChannel.MESSENGER": "Facebook Messenger",
    "messageOtp.auth.sentTo": "We sent a code to your {0}, {1}.",
    "messageOtp.auth.sentToChannel": "We sent a code to your {0}.",
    "messageOtp.auth.openApp": "Open {0} on your phone. The message is from {1}.",
    "messageOtp.otherWay.title": "Get the code another way",
    "messageOtp.otherWay.help":
        "Choose an available method. Requesting a new code replaces the previous code.",
    "messageOtp.otherWay.send": "Send code",
    "messageOtp.choose.title": "Get a code by",
    "messageOtp.choose.help": "Use one of the ways you set up.",
    "messageOtp.choose.option": "We send the code to your {0}, {1}.",
    "messageOtp.delivery.unknown": "Delivery is not confirmed yet.",
    "messageOtp.delivery.failed": "We could not send the code to your {0}.",
    "messageOtp.messenger.title": "Get your code in Messenger",
    "messageOtp.messenger.intro": "The Facebook Page {0} sends your code in a chat.",
    "messageOtp.messenger.connect": "Connect Messenger",
    "messageOtp.messenger.step1": "Open Messenger or Facebook",
    "messageOtp.messenger.step2": "Tap Get Started in the chat",
    "messageOtp.messenger.step3": "Come back and enter the code",
    "messageOtp.messenger.word": "No code after tapping Get Started? Send {0} to {1} in the chat.",
    "messageOtp.messenger.scan": "Scan with your phone to open the chat.",
    "messageOtp.messenger.check": "Check again",
    "messageOtp.messenger.pending": "Waiting for you to open the chat.",
    "messageOtp.messenger.codeSent": "We sent your code in the chat.",
    "messageOtp.messenger.expired": "This request has expired. Get the code another way.",
} as const

// Keycloakify resolves messages in the browser: keys that the server-side
// bundles define (sequent-theme, message-otp-authenticator's theme-resources)
// have to be repeated here.
const {useI18n, ofTypeI18n} = i18nBuilder
    .withThemeName<ThemeName>()
    .withCustomTranslations({
        en: englishMessages,
        es: {
            loginAccountTitle: {
                "sequent-ui-admin": "Iniciar sesión para continuar",
                "sequent-ui-voting": "Iniciar sesión para votar",
                "sequent-ui-architect": "Iniciar sesión en Election Architect",
            },
            doContinue: "Continuar",
            "identity-provider-login-label": "O inicie sesión con",
            continueWithProvider: "Continuar con {0}",
            doLogIn: "INICIAR SESIÓN",
            "system.version": "Versión:",
            "system.hash": "Hash:",
            invalidCredentialsMessage: "Los datos introducidos no son correctos.",
            "messageOtp.auth.address": "Enviamos un código a {0}.",
            "messageOtp.auth.title": "Ingrese su código de verificación",
            "messageOtp.auth.instructionBoth":
                "Ingrese el código que le enviamos a su dispositivo móvil por SMS o a su email.",
            "messageOtp.auth.instructionSms":
                "Ingrese el código que le enviamos a su dispositivo móvil por SMS.",
            "messageOtp.auth.instructionEmail": "Ingrese el código que le enviamos a su email.",
            "messageOtp.auth.ttlTime": "Código válido durante {0} minutos.",
            "messageOtp.auth.resend.button": "Reenviar código",
            "messageOtp.auth.resend.timer": "Reenviar código en {0} segundos",
            "messageOtp.otl.address": "Enviamos un enlace de acceso a {0}.",
            "messageOtp.otl.title": "Revise sus mensajes",
            "messageOtp.otl.instructionBoth":
                "Le hemos enviado un enlace de verificación a su dispositivo móvil por email y/o SMS. Abra este enlace para continuar.",
            "messageOtp.otl.instructionSms":
                "Le hemos enviado un enlace de verificación a su dispositivo móvil por SMS. Abra este enlace para continuar.",
            "messageOtp.otl.instructionEmail":
                "Le hemos enviado un enlace de verificación a su dispositivo móvil por email. Abra este enlace para continuar.",
            "messageOtp.otl.ttlTime": "Enlace válido durante {0} minutos.",
            "messageOtp.otl.resend.button": "Reenviar enlace",
            "messageOtp.otl.resend.timer": "Reenviar enlace en {0} segundos",
            otpDigit: "Dígito {0} de {1}",
            otpCodeLabel: "Código de verificación",
            "messageChannel.EMAIL": "Email",
            "messageChannel.SMS": "SMS",
            "messageChannel.WHATSAPP": "WhatsApp",
            "messageChannel.VIBER": "Viber",
            "messageChannel.MESSENGER": "Facebook Messenger",
            "messageOtp.auth.sentTo": "Enviamos un código a su {0}, {1}.",
            "messageOtp.auth.sentToChannel": "Enviamos un código a su {0}.",
            "messageOtp.auth.openApp": "Abra {0} en su teléfono. El mensaje es de {1}.",
            "messageOtp.otherWay.title": "Recibir el código de otra forma",
            "messageOtp.otherWay.help":
                "Elija un método disponible. Solicitar un código nuevo reemplaza el anterior.",
            "messageOtp.otherWay.send": "Enviar código",
            "messageOtp.choose.title": "Recibir un código por",
            "messageOtp.choose.help": "Use una de las formas que configuró.",
            "messageOtp.choose.option": "Enviamos el código a su {0}, {1}.",
            "messageOtp.delivery.unknown": "La entrega aún no está confirmada.",
            "messageOtp.delivery.failed": "No pudimos enviar el código a su {0}.",
            "messageOtp.messenger.title": "Reciba su código en Messenger",
            "messageOtp.messenger.intro":
                "La página de Facebook {0} le envía el código en un chat.",
            "messageOtp.messenger.connect": "Conectar Messenger",
            "messageOtp.messenger.step1": "Abra Messenger o Facebook",
            "messageOtp.messenger.step2": "Pulse Empezar en el chat",
            "messageOtp.messenger.step3": "Vuelva aquí e introduzca el código",
            "messageOtp.messenger.word":
                "¿No recibe el código tras pulsar Empezar? Envíe {0} a {1} en el chat.",
            "messageOtp.messenger.scan": "Escanee con su teléfono para abrir el chat.",
            "messageOtp.messenger.check": "Comprobar de nuevo",
            "messageOtp.messenger.pending": "Esperando a que abra el chat.",
            "messageOtp.messenger.codeSent": "Le enviamos el código en el chat.",
            "messageOtp.messenger.expired":
                "Esta solicitud ha caducado. Reciba el código de otra forma.",
        },
    })
    .build()

type I18n = typeof ofTypeI18n

export {useI18n, type I18n}

// Keycloakify falls back to our English custom messages outside en/es. Server
// translations take precedence; identify only exact English defaults, never
// guess the language of a realm's custom text.
export function messageLanguage(
    kcContext: KcContext,
    i18n: I18n,
    key: keyof typeof englishMessages
): string {
    const current = i18n.currentLanguage.languageTag
    const server = kcContext["x-keycloakify"].messages[key]
    if (server === undefined) {
        return current === "es" ? "es" : "en"
    }
    const value = englishMessages[key]
    const english = typeof value === "string" ? value : value[kcContext.themeName]
    // These two pre-existing provider titles differ from the browser defaults.
    const providerTitle = {
        "messageOtp.auth.title": "Your Authentication Code",
        "messageOtp.otl.title": "Your Authentication Link",
    }
    if (server === english || server === providerTitle[key as keyof typeof providerTitle]) {
        return "en"
    }
    return current
}
