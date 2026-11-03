// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {i18nBuilder} from "keycloakify/login"
import type {ThemeName} from "../kc.gen"
import type {KcContext} from "./KcContext"
import {enrollmentEnglish, enrollmentSpanish} from "./enrollment/messages"
import {scanovateEnglish, scanovateSpanish} from "./scanovate/messages"

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
    "messageOtp.auth.codeProgress": "Code {0} of {1}",
    "messageOtp.auth.codeNext":
        "You’ll get {0} codes, one after the other. After this one, we’ll send you the next.",
    "messageOtp.auth.codeLast": "Your previous code was accepted. This is the last one.",
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
    "messageOtp.auth.tooManyAttempts": "Too many attempts. Start again.",
    "messageOtp.auth.restart": "Start again",
    "messageOtp.auth.neutralSent": "If an account exists, we sent a code to its contact.",
    "messageOtp.delivery.unknown": "Delivery is not confirmed yet.",
    "messageOtp.delivery.failed": "We could not send the code to your {0}.",
    "messageOtp.messenger.title": "Get your code in Messenger",
    "messageOtp.messenger.ifMessenger": "If you get your codes in Messenger",
    "messageOtp.messenger.intro": "The Facebook Page {0} sends your code in a chat.",
    "messageOtp.messenger.introPage": "{0}'s Facebook Page, {1}, sends your code in a chat.",
    "messageOtp.messenger.connect": "Connect Messenger",
    "messageOtp.messenger.connectHelp": "Open {0} and tap Get Started to get your code.",
    "messageOtp.messenger.open": "Open Messenger",
    "messageOtp.messenger.step1": "Open Messenger or Facebook",
    "messageOtp.messenger.step2": "Tap Get Started in the chat",
    "messageOtp.messenger.step3": "Come back and enter the code",
    "messageOtp.messenger.scanStep1": "Scan the code with your phone",
    "messageOtp.messenger.scanStep2": "Tap Get Started in the chat",
    "messageOtp.messenger.scanStep3": "Enter the code below",
    "messageOtp.messenger.getStarted":
        "Messenger lets the Page send you the code only after you tap Get Started.",
    "messageOtp.messenger.scan": "Scan with your phone to open the chat.",
    "messageOtp.messenger.check": "Check again",
    "messageOtp.messenger.pending": "Waiting for you to open the chat.",
    "messageOtp.messenger.codeSent": "We sent your code in the chat.",
    "messageOtp.messenger.expired":
        "This request has expired. Resend the code, then tap Get Started again.",
    "messageOtp.messenger.reconnect":
        "Codes now come from a different Facebook Page. Tap Get Started in its chat to connect Messenger again. The new connection is saved when you enter the code.",
    "messageOtp.pending.channel": "We will send the result to your {0}.",
    "messageOtp.pending.channelAddress": "We will send the result to your {0}, {1}.",
    "messageOtp.pending.reference":
        "Need help? Contact support and give them this reference code: {0}",
    "messageOtp.pending.copyReference": "Copy",
    "messageOtp.pending.referenceCopied": "Copied",
    "messaging.channelChoice.label": "Get codes and updates by",
    "messaging.channelChoice.help": "The ways {0} can reach you.",
    "messaging.number.WHATSAPP": "WhatsApp number",
    "messaging.number.VIBER": "Viber number",
    "messaging.number.help": "{0} will send codes and election notices to this number.",
    "messaging.consent": "I agree that {0} sends me codes and election notices by {1}.",
    "forgotPassword.success.contact.message":
        "If an account exists, we sent the link to its contact.",
    "resetAppOtp.auth.enterContactTitle": "Change how we contact you",
    "resetAppOtp.auth.sendCodeButton": "Send verification code",
    "resetAppOtp.auth.enterOtpTitle": "Enter the code we sent you",
    "resetAppOtp.auth.sentToContact": "Verification code sent to <strong>{0}</strong>",
    "resetAppOtp.auth.changeContact": "Change the contact",
    "resetAppOtp.auth.verifyButton": "Verify",
    "resetAppOtp.auth.resendTextPrefix.question": "Didn't receive the code yet?",
    "resetAppOtp.auth.ttlTime": "The code is valid for {0} minutes.",
    "resetAppOtp.auth.resend.timer": "Wait {0} seconds to resend",
    "resetAppOtp.auth.resend.button.link": "Click here to resend",
    "resetAppOtp.auth.error.invalidInput": "Please enter a valid number.",
    "resetAppOtp.auth.error.sendError": "Could not send verification code. Please try again.",
    "resetAppOtp.auth.error.resendTimer": "Please wait before asking for another code.",
    "resetAppOtp.auth.error.codeExpired": "The code has expired.",
    "resetAppOtp.auth.error.codeInvalid": "Invalid code entered, please try again.",
    "resetAppOtp.auth.error.maxReceiverReuse":
        "Another voter already uses this contact. Contact administrator.",
    "resetAppOtp.auth.error.invalidCountry":
        "Invalid country code for phone number. Contact administrator.",
    ...scanovateEnglish,
    ...enrollmentEnglish,
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
            "messageOtp.auth.codeProgress": "Código {0} de {1}",
            "messageOtp.auth.codeNext":
                "Recibirá {0} códigos, uno tras otro. Después de este, le enviaremos el siguiente.",
            "messageOtp.auth.codeLast": "Su código anterior fue aceptado. Este es el último.",
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
            "messageOtp.auth.tooManyAttempts": "Demasiados intentos. Empiece de nuevo.",
            "messageOtp.auth.restart": "Empezar de nuevo",
            "messageOtp.auth.neutralSent":
                "Si existe una cuenta, le enviamos un código a su contacto.",
            "messageOtp.delivery.unknown": "La entrega aún no está confirmada.",
            "messageOtp.delivery.failed": "No pudimos enviar el código a su {0}.",
            "messageOtp.messenger.title": "Reciba su código en Messenger",
            "messageOtp.messenger.ifMessenger": "Si recibe sus códigos por Messenger",
            "messageOtp.messenger.intro":
                "La página de Facebook {0} le envía el código en un chat.",
            "messageOtp.messenger.introPage":
                "La página de Facebook de {0}, {1}, le envía el código en un chat.",
            "messageOtp.messenger.connect": "Conectar Messenger",
            "messageOtp.messenger.connectHelp": "Abra {0} y pulse Empezar para recibir su código.",
            "messageOtp.messenger.open": "Abrir Messenger",
            "messageOtp.messenger.step1": "Abra Messenger o Facebook",
            "messageOtp.messenger.step2": "Pulse Empezar en el chat",
            "messageOtp.messenger.step3": "Vuelva aquí e introduzca el código",
            "messageOtp.messenger.scanStep1": "Escanee el código con su teléfono",
            "messageOtp.messenger.scanStep2": "Pulse Empezar en el chat",
            "messageOtp.messenger.scanStep3": "Introduzca el código abajo",
            "messageOtp.messenger.getStarted":
                "Messenger solo permite que la página le envíe el código después de que pulse Empezar.",
            "messageOtp.messenger.scan": "Escanee con su teléfono para abrir el chat.",
            "messageOtp.messenger.check": "Comprobar de nuevo",
            "messageOtp.messenger.pending": "Esperando a que abra el chat.",
            "messageOtp.messenger.codeSent": "Le enviamos el código en el chat.",
            "messageOtp.messenger.expired":
                "Esta solicitud ha caducado. Reenvíe el código y vuelva a pulsar Empezar.",
            "messageOtp.messenger.reconnect":
                "Los códigos ahora llegan desde otra página de Facebook. Pulse Empezar en su chat para volver a conectar Messenger. La nueva conexión se guarda cuando introduzca el código.",
            "messageOtp.pending.channel": "Le enviaremos el resultado a su {0}.",
            "messageOtp.pending.channelAddress": "Le enviaremos el resultado a su {0}, {1}.",
            "messageOtp.pending.reference":
                "¿Necesita ayuda? Contacte con soporte y deles este código de referencia: {0}",
            "messageOtp.pending.copyReference": "Copiar",
            "messageOtp.pending.referenceCopied": "Copiado",
            "messaging.channelChoice.label": "Recibir códigos y avisos por",
            "messaging.channelChoice.help": "Las formas en que {0} puede contactarle.",
            "messaging.number.WHATSAPP": "Número de WhatsApp",
            "messaging.number.VIBER": "Número de Viber",
            "messaging.number.help": "{0} enviará códigos y avisos electorales a este número.",
            "messaging.consent": "Acepto que {0} me envíe códigos y avisos electorales por {1}.",
            "forgotPassword.success.contact.message":
                "Si existe una cuenta, le enviamos el enlace a su contacto.",
            "resetAppOtp.auth.enterContactTitle": "Cambiar cómo le contactamos",
            "resetAppOtp.auth.sendCodeButton": "Enviar código de verificación",
            "resetAppOtp.auth.enterOtpTitle": "Introduzca el código que le hemos enviado",
            "resetAppOtp.auth.sentToContact":
                "Código de verificación enviado a <strong>{0}</strong>",
            "resetAppOtp.auth.changeContact": "Cambiar el contacto",
            "resetAppOtp.auth.verifyButton": "Verificar",
            "resetAppOtp.auth.resendTextPrefix.question": "¿Aún no recibió el código?",
            "resetAppOtp.auth.ttlTime": "El código es válido durante {0} minutos.",
            "resetAppOtp.auth.resend.timer": "Espere {0} segundos para reenviar",
            "resetAppOtp.auth.resend.button.link": "Haga clic aquí para reenviar",
            "resetAppOtp.auth.error.invalidInput": "Por favor, introduzca un número válido.",
            "resetAppOtp.auth.error.sendError":
                "No se pudo enviar el código de verificación. Por favor, intente de nuevo.",
            "resetAppOtp.auth.error.resendTimer": "Por favor, espere antes de pedir otro código.",
            "resetAppOtp.auth.error.codeExpired": "El código ha expirado.",
            "resetAppOtp.auth.error.codeInvalid": "Código inválido, por favor intente de nuevo.",
            "resetAppOtp.auth.error.maxReceiverReuse":
                "Otro votante ya usa este contacto. Contacte al administrador.",
            "resetAppOtp.auth.error.invalidCountry":
                "Código de país no válido para el número de teléfono. Contacte al administrador.",
            ...scanovateSpanish,
            ...enrollmentSpanish,
        },
    })
    .build()

type I18n = typeof ofTypeI18n
type MessageKey = keyof typeof englishMessages

export {useI18n, type I18n, type MessageKey}

// Keycloakify falls back to our English custom messages outside en/es. Server
// translations take precedence; identify only exact English defaults, never
// guess the language of a realm's custom text.
export function messageLanguage(kcContext: KcContext, i18n: I18n, key: MessageKey): string {
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

export function isMessageKey(key: string): key is MessageKey {
    return Object.prototype.hasOwnProperty.call(englishMessages, key)
}

// For keys chosen at run time: a document type, a stored attribute or a server error.
export function dynamicMessageLanguage(kcContext: KcContext, i18n: I18n, key: string): string {
    if (isMessageKey(key)) {
        return messageLanguage(kcContext, i18n, key)
    }
    return kcContext["x-keycloakify"].messages[key] === undefined
        ? "en"
        : i18n.currentLanguage.languageTag
}
