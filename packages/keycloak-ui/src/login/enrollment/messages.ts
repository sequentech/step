// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The registration form and the pages that end an enrollment. The outcome and
// reason keys repeat the voter-enrollment bundle so the browser fallback
// matches the server.
export const enrollmentEnglish = {
    enrollmentStepDetails: "Your details",
    enrollmentPasswordStrength: "Password strength",
    enrollmentOutcomeEnrolled: "Enrollment complete",
    enrollmentOutcomeUnderReview: "Enrollment under review",
    enrollmentOutcomeRejected: "Enrollment not approved",
    enrollmentOutcomeValidated: "Enrollment",

    registerFinishTitle: "Congratulations!",
    registerFinishMessage:
        "You are successfully validated.<br>Please expect to receive an SMS or email for further instructions.<br>Thank you!",
    registerFinishManualTitle: "Manual Verification required",
    registerFinishManualMessage:
        "Please expect to receive SMS or email for further instructions.<br>Thank you!",
    registerFinishRejectedTitle: "Application Disapproved",
    registerFinishRejectedMessage:
        "Please expect to receive an SMS or email for further instructions.<br>Thank you!",
    rejectReasonListItems: "The following fields did not match in the registry:",
    messageFinishTitle: "Register",
    messageFinish: "Your record has already been validated.<br>Thank you.",
    empty: "Empty",

    INSUFFICIENT_INFORMATION:
        "It seems some required information is missing. Please ensure you fill out all the necessary fields in the form to proceed with your enrollment.",
    NO_VOTER: "The data provided for enrollment does not match any existing user in the registry.",
    ALREADY_APPROVED:
        "Our records indicate that you have already completed the enrollment process. If you believe this is an error, please contact support for further assistance.",
    OTHER: "An unexpected issue occurred while processing your enrollment. Please try again later. If the problem persists, reach out to our support team for assistance.",
} as const

export const enrollmentSpanish: Record<keyof typeof enrollmentEnglish, string> = {
    enrollmentStepDetails: "Sus datos",
    enrollmentPasswordStrength: "Seguridad de la contraseña",
    enrollmentOutcomeEnrolled: "Inscripción completada",
    enrollmentOutcomeUnderReview: "Inscripción en revisión",
    enrollmentOutcomeRejected: "Inscripción no aprobada",
    enrollmentOutcomeValidated: "Inscripción",

    registerFinishTitle: "¡Enhorabuena!",
    registerFinishMessage:
        "Su identidad se ha validado correctamente.<br>Recibirá un SMS o un email con más instrucciones.<br>¡Gracias!",
    registerFinishManualTitle: "Se requiere verificación manual",
    registerFinishManualMessage: "Recibirá un SMS o un email con más instrucciones.<br>¡Gracias!",
    registerFinishRejectedTitle: "Solicitud no aprobada",
    registerFinishRejectedMessage: "Recibirá un SMS o un email con más instrucciones.<br>¡Gracias!",
    rejectReasonListItems: "Los siguientes campos no coinciden con el registro:",
    messageFinishTitle: "Registro",
    messageFinish: "Su registro ya ha sido validado.<br>Gracias.",
    empty: "Vacío",

    INSUFFICIENT_INFORMATION:
        "Parece que falta información obligatoria. Complete todos los campos necesarios del formulario para continuar con su inscripción.",
    NO_VOTER: "Los datos proporcionados no coinciden con ningún votante del registro.",
    ALREADY_APPROVED:
        "Según nuestros registros, ya completó el proceso de inscripción. Si cree que es un error, contacte con soporte.",
    OTHER: "Se produjo un problema inesperado al procesar su inscripción. Inténtelo de nuevo más tarde. Si el problema continúa, contacte con nuestro equipo de soporte.",
}
