// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The identity verification pages. The error keys repeat the
// scanovate-authenticator bundle so the browser fallback matches the server.
export const scanovateEnglish = {
    scanovateStepEyebrow: "Step {0} of {1} · {2}",
    scanovateStepVerifyIdentity: "Verify identity",
    scanovateStepConfirm: "Confirm",
    scanovateProgressLabel: "Enrollment progress",

    scanovateDocumentGeneric: "ID",
    "scanovateDocumentType.philippinePassport": "Passport",
    "scanovateDocumentType.driversLicense": "Driver’s License",
    "scanovateDocumentType.philSysID": "PhilSys ID",
    "scanovateDocumentType.iBP": "IBP ID",
    "scanovateDocumentType.seamanBook": "Seafarer’s Book",

    scanovateIntroTipLight: "Find a well-lit place.",
    scanovateTryAgain: "Try again",

    scanovateDocumentStepTitle: "Scan your ID",
    scanovateDocumentStepLead:
        "Follow the instructions below to scan your {0}. Then we will check your face.",
    scanovateDocumentFrameTitle: "ID scan",
    scanovateFaceStepTitle: "Check your face",
    scanovateFaceStepLead:
        "Your ID is scanned. Follow the instructions below so we can confirm it is you in front of the camera.",
    scanovateFaceFrameTitle: "Face check",
    scanovateHoldingStepTitle: "Take a photo holding your ID",
    scanovateHoldingStepLead:
        "Hold your {0} open next to your face, with both your face and its photo in view, and take the photo.",
    scanovateHoldingCameraLabel: "Camera",
    scanovateHoldingPhotoLabel: "Your photo holding your ID",
    scanovateHoldingTake: "Take photo",
    scanovateHoldingRetake: "Take it again",
    scanovateHoldingUse: "Use this photo",
    scanovateUploadProblemTitle: "We couldn’t send your photo",
    scanovateUploadProblemText: "Check your internet connection and try again.",
    scanovateCameraProblemTitle: "We couldn’t use your camera",
    scanovateCameraProblemText:
        "Allow camera access for this site in your browser settings, close other apps that are using the camera, then try again. If your device has no camera, open this page on a phone.",
    scanovateSessionExpiredTitle: "The scan timed out",
    scanovateSessionExpiredText:
        "The scan took too long and was stopped. Try again when you are ready.",
    scanovateStartAgainTitle: "Start the verification again",
    scanovateStartAgainText:
        "This scan can’t be continued. Start again to get a new one; it doesn’t use up one of your attempts.",
    scanovateClosedTitle: "The scan was closed",
    scanovateClosedText: "The scan was closed before it finished. Try again when you are ready.",
    scanovateProblemTitle: "The scan didn’t finish",
    scanovateProblemText: "Check your internet connection and try again.",
    scanovateCheckingScanned: "ID scan and face check finished",

    scanovateCheckingTitle: "Checking your identity",
    scanovateCheckingLead: "This usually takes a few seconds. Keep this page open.",
    scanovateCheckingVerifying: "Checking your ID and your face",
    scanovateCheckingReading: "Reading your details",

    scanovateErrorHeading: "We couldn’t verify your ID",
    scanovateErrorFinalHeading: "We couldn’t verify your identity",
    scanovateErrorTipsTitle: "Before you try again",
    scanovateTipLight: "Use a well-lit place, without direct light on the ID.",
    scanovateTipCorners: "Keep all four corners inside the frame.",
    scanovateTipSteady: "Hold your phone steady until the photo is taken.",
    scanovateTipValidDocument: "Use your own ID, and check that it hasn’t expired.",
    scanovateTipFaceUncovered: "Keep your face well lit and uncovered.",
    scanovateTipLookAtCamera: "Look straight at the camera during the video.",
    scanovateTipSameDocument: "Use the same ID you chose in the enrollment form.",
    scanovateTipCheckDetails: "Check that the details you entered match your ID.",
    scanovateTipConnection: "Check your internet connection.",
    scanovateTipWait: "Wait a few minutes before trying again.",
    scanovateAttemptsLeft: "You can try {0} more times.",
    scanovateAttemptsLeftOne: "You can try 1 more time.",
    scanovateSupportReferenceLabel: "Need help? Contact support and give them this reference code:",
    scanovateCopy: "Copy",
    scanovateCopyReference: "Copy support reference",
    scanovateCopied: "Reference copied.",
    scanovateCopyFailed: "Couldn’t copy. Select the code and copy it instead.",
    scanovateInternalError: "An internal error has occurred. Please try again later.",
    scanovateVerificationFailedError: "We could not verify your identity. Please try again.",
    scanovateDocumentAuthenticationError:
        "We could not authenticate your identity document. Please make sure it is valid, well lit and fully visible, and try again.",
    scanovateDocumentUnreadableError:
        "We couldn’t read your ID. Scan it again with the whole ID in the frame, well lit and in focus.",
    scanovateAttributesError:
        "The information extracted by our system doesn’t match the details you provided in the form. Try again or restart registration and double-check your entries.",
    scanovateScoringError:
        "The quality of the provided images needs to be improved. Please try again.",
    scanovateMaxRetriesError:
        "Your identity could not be verified after the maximum number of attempts, resulting in an unsuccessful enrollment.",
    scanovateCaptureInvalidError: "We couldn’t use the photos. Please take them again.",

    scanovateConfirmTitle: "Check the details from your ID",
    scanovateConfirmLead:
        "We read these details from your {0}. They can’t be changed here; if something is wrong, scan your ID again.",
    scanovateConfirmDocument: "ID",
    scanovateConfirmSubmit: "Confirm and enroll",
    scanovateConfirmRetry: "Scan my ID again",
    scanovateStartOver: "Start over",
    scanovateLivenessError:
        "We could not confirm that it was you in front of the camera. Please try again.",
    scanovateFaceMismatchError: "The face in your photos doesn’t match the photo on your ID.",
    scanovateFaceNotFoundError:
        "We couldn’t find your face on your ID or in the photo of you holding it. Make sure the photo on your ID is clear and not covered.",
    dateOfBirth: "Date of birth",
    "sequent.read-only.id-card-number": "ID number",
} as const

export const scanovateSpanish: Record<keyof typeof scanovateEnglish, string> = {
    scanovateStepEyebrow: "Paso {0} de {1} · {2}",
    scanovateStepVerifyIdentity: "Verificar identidad",
    scanovateStepConfirm: "Confirmar",
    scanovateProgressLabel: "Progreso de la inscripción",

    scanovateDocumentGeneric: "documento de identidad",
    "scanovateDocumentType.philippinePassport": "pasaporte",
    "scanovateDocumentType.driversLicense": "permiso de conducir",
    "scanovateDocumentType.philSysID": "PhilSys ID",
    "scanovateDocumentType.iBP": "IBP ID",
    "scanovateDocumentType.seamanBook": "libreta de marino",

    scanovateIntroTipLight: "Busque un lugar bien iluminado.",
    scanovateTryAgain: "Volver a intentarlo",

    scanovateDocumentStepTitle: "Escanee su documento",
    scanovateDocumentStepLead:
        "Siga las instrucciones de abajo para escanear su {0}. Después comprobaremos su cara.",
    scanovateDocumentFrameTitle: "Escaneo del documento",
    scanovateFaceStepTitle: "Compruebe su cara",
    scanovateFaceStepLead:
        "Su documento está escaneado. Siga las instrucciones de abajo para confirmar que es usted quien está frente a la cámara.",
    scanovateFaceFrameTitle: "Comprobación facial",
    scanovateHoldingStepTitle: "Hágase una foto con su documento",
    scanovateHoldingStepLead:
        "Sostenga su {0} abierto junto a su cara, con su cara y la foto del documento a la vista, y haga la foto.",
    scanovateHoldingCameraLabel: "Cámara",
    scanovateHoldingPhotoLabel: "Su foto con su documento",
    scanovateHoldingTake: "Hacer la foto",
    scanovateHoldingRetake: "Repetirla",
    scanovateHoldingUse: "Usar esta foto",
    scanovateUploadProblemTitle: "No pudimos enviar su foto",
    scanovateUploadProblemText: "Compruebe su conexión a internet e inténtelo de nuevo.",
    scanovateCameraProblemTitle: "No pudimos usar su cámara",
    scanovateCameraProblemText:
        "Permita el acceso a la cámara para este sitio en la configuración de su navegador, cierre otras aplicaciones que la estén usando y vuelva a intentarlo. Si su dispositivo no tiene cámara, abra esta página en un teléfono.",
    scanovateSessionExpiredTitle: "Se agotó el tiempo del escaneo",
    scanovateSessionExpiredText:
        "El escaneo tardó demasiado y se detuvo. Vuelva a intentarlo cuando esté listo.",
    scanovateStartAgainTitle: "Vuelva a empezar la verificación",
    scanovateStartAgainText:
        "Este escaneo no se puede continuar. Vuelva a empezar para obtener uno nuevo; no gasta ninguno de sus intentos.",
    scanovateClosedTitle: "Se cerró el escaneo",
    scanovateClosedText:
        "El escaneo se cerró antes de terminar. Vuelva a intentarlo cuando esté listo.",
    scanovateProblemTitle: "El escaneo no terminó",
    scanovateProblemText: "Compruebe su conexión a internet y vuelva a intentarlo.",
    scanovateCheckingScanned: "Escaneo del documento y comprobación facial terminados",

    scanovateCheckingTitle: "Comprobando su identidad",
    scanovateCheckingLead: "Suele tardar unos segundos. Mantenga esta página abierta.",
    scanovateCheckingVerifying: "Comprobando su documento y su cara",
    scanovateCheckingReading: "Leyendo sus datos",

    scanovateErrorHeading: "No pudimos verificar su documento",
    scanovateErrorFinalHeading: "No pudimos verificar su identidad",
    scanovateErrorTipsTitle: "Antes de volver a intentarlo",
    scanovateTipLight: "Use un lugar bien iluminado, sin luz directa sobre el documento.",
    scanovateTipCorners: "Mantenga las cuatro esquinas dentro del marco.",
    scanovateTipSteady: "Sujete el teléfono firme hasta que se tome la foto.",
    scanovateTipValidDocument: "Use su propio documento y compruebe que no ha caducado.",
    scanovateTipFaceUncovered: "Mantenga su cara bien iluminada y descubierta.",
    scanovateTipLookAtCamera: "Mire de frente a la cámara durante el vídeo.",
    scanovateTipSameDocument: "Use el mismo documento que eligió en el formulario de inscripción.",
    scanovateTipCheckDetails: "Compruebe que los datos que introdujo coinciden con su documento.",
    scanovateTipConnection: "Compruebe su conexión a internet.",
    scanovateTipWait: "Espere unos minutos antes de volver a intentarlo.",
    scanovateAttemptsLeft: "Puede intentarlo {0} veces más.",
    scanovateAttemptsLeftOne: "Puede intentarlo 1 vez más.",
    scanovateSupportReferenceLabel:
        "¿Necesita ayuda? Contacte con soporte y facilíteles este código de referencia:",
    scanovateCopy: "Copiar",
    scanovateCopyReference: "Copiar la referencia de soporte",
    scanovateCopied: "Referencia copiada.",
    scanovateCopyFailed: "No se pudo copiar. Seleccione el código y cópielo.",
    scanovateInternalError: "Se ha producido un error interno. Vuelva a intentarlo más tarde.",
    scanovateVerificationFailedError: "No pudimos verificar su identidad. Vuelva a intentarlo.",
    scanovateDocumentAuthenticationError:
        "No pudimos autenticar su documento de identidad. Asegúrese de que es válido, está bien iluminado y se ve completo, y vuelva a intentarlo.",
    scanovateDocumentUnreadableError:
        "No pudimos leer su documento. Vuelva a escanearlo con todo el documento en el encuadre, bien iluminado y enfocado.",
    scanovateAttributesError:
        "Los datos extraídos de su documento no coinciden con los que introdujo en el formulario. Vuelva a intentarlo o reinicie la inscripción y revise sus datos.",
    scanovateScoringError: "Hay que mejorar la calidad de las imágenes. Vuelva a intentarlo.",
    scanovateMaxRetriesError:
        "No se pudo verificar su identidad tras el número máximo de intentos, por lo que la inscripción no se ha completado.",
    scanovateCaptureInvalidError: "No pudimos usar las fotos. Vuelva a tomarlas.",

    scanovateConfirmTitle: "Revise los datos de su documento",
    scanovateConfirmLead:
        "Hemos leído estos datos de su {0}. No se pueden cambiar aquí; si algo no es correcto, vuelva a escanear su documento.",
    scanovateConfirmDocument: "Documento",
    scanovateConfirmSubmit: "Confirmar e inscribirme",
    scanovateConfirmRetry: "Volver a escanear mi documento",
    scanovateStartOver: "Volver a empezar",
    scanovateLivenessError:
        "No pudimos confirmar que era usted quien estaba frente a la cámara. Vuelva a intentarlo.",
    scanovateFaceMismatchError: "La cara de sus fotos no coincide con la foto de su documento.",
    scanovateFaceNotFoundError:
        "No encontramos su cara en su documento ni en la foto en la que lo sostiene. Asegúrese de que la foto de su documento se vea bien y no esté tapada.",
    dateOfBirth: "Fecha de nacimiento",
    "sequent.read-only.id-card-number": "Número de documento",
}
