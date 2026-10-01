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

    scanovateIntroTitle: "Verify your identity",
    scanovateIntroLead:
        "We will take photos of your {0} and a short video of your face to confirm it is you. It takes about 2 minutes.",
    scanovateIntroDocumentBothSides: "Front and back of your ID",
    scanovateIntroDocumentFrontSide: "Your ID",
    scanovateIntroDocumentHint: "On a flat, dark surface, with all four corners visible.",
    scanovateIntroFace: "Your face",
    scanovateIntroFaceHint: "A few seconds looking at the camera.",
    scanovateIntroVideo: "You holding your ID",
    scanovateIntroVideoHint: "A short video with the ID next to your face.",
    scanovateIntroBeforeTitle: "Before you start",
    scanovateIntroTipLight: "Find a well-lit place.",
    scanovateIntroTipCoverings: "Remove sunglasses, hats and face coverings.",
    scanovateIntroTipDocument: "Have your {0} with you.",
    scanovateIntroPrivacy:
        "Your photos and video are deleted after verification. We keep the details from your ID.",
    scanovateIntroStart: "Start",

    scanovateCameraStarting: "Starting the camera…",
    scanovateCameraDeniedTitle: "Allow access to your camera",
    scanovateCameraDeniedText:
        "We need your camera to take the photos. Allow camera access for this site in your browser settings, then try again.",
    scanovateCameraNotFoundTitle: "We couldn’t find a camera",
    scanovateCameraNotFoundText:
        "Connect a camera, or open this page on a phone or tablet with a camera, then try again.",
    scanovateCameraInUseTitle: "Your camera is in use",
    scanovateCameraInUseText:
        "Close other apps or browser tabs that are using the camera, then try again.",
    scanovateInsecureContextTitle: "The camera can’t be used on this page",
    scanovateInsecureContextText:
        "Your browser only allows the camera on secure pages. Open the link you received again, or contact support.",
    scanovateCameraFailedTitle: "We couldn’t start your camera",
    scanovateCameraFailedText: "Check that your camera works and try again.",
    scanovateAnalyzerFailedTitle: "We couldn’t prepare the camera check",
    scanovateAnalyzerFailedText: "Check your internet connection and try again.",
    scanovateRecorderUnsupportedTitle: "This browser can’t record video",
    scanovateRecorderUnsupportedText:
        "Open this page in an up-to-date version of Chrome, Safari, Firefox or Edge.",
    scanovateTryAgain: "Try again",
    scanovateBackToStart: "Back to start",

    scanovateCaptureFrontTitle: "Front of your ID",
    scanovateCaptureBackTitle: "Back of your ID",
    scanovateCaptureFaceTitle: "Your face",
    scanovateCaptureVideoTitle: "You holding your ID",
    scanovateCaptureCounter: "{0} · {1} of {2}",
    scanovateCaptureFrontHeading: "Place the front of your ID in the frame",
    scanovateCaptureFrontText:
        "Use a dark surface, avoid glare and keep all four corners visible. The photo is taken automatically.",
    scanovateCaptureBackHeading: "Now the back of your ID",
    scanovateCaptureBackText: "As before: all four corners inside the frame and no glare.",
    scanovateCaptureFaceHeading: "Keep your face inside the oval",
    scanovateCaptureFaceText:
        "Make sure your face is well lit and nothing covers it. This takes a few seconds.",
    scanovateCaptureVideoHeading: "Hold your ID next to your face",
    scanovateCaptureVideoText:
        "Keep your face and the front of your ID in view until the recording ends.",
    scanovateChipFront: "Front",
    scanovateChipBack: "Back",
    scanovateChipFace: "Face",
    scanovateCaptureSecure: "Secure identity verification",
    scanovateTakePhoto: "Take photo",
    scanovateStop: "Stop and go back",
    scanovateHelp: "Help",
    scanovateCaptured: "Photo taken",
    scanovateRecording: "REC {0}",
    scanovateRecordingStatus: "Recording",
    scanovateStepDone: "Done",
    scanovateStepCurrent: "In progress",

    scanovateGuideLoading: "Getting ready…",
    scanovateGuidePlaceDocument: "Place your ID in the frame",
    scanovateGuideTurnDocument: "Turn your ID over",
    scanovateGuideDocumentTooFar: "Move your ID closer",
    scanovateGuideDocumentTooClose: "Move your ID a little farther away",
    scanovateGuideDocumentNotAligned: "Line up your ID with the frame",
    scanovateGuideTooDark: "Find a brighter place",
    scanovateGuideDocumentTooBright: "Move away from direct light",
    scanovateGuideGlare: "Tilt your ID to remove the glare",
    scanovateGuideDocumentBlurry: "Hold steady so your ID is sharp",
    scanovateGuideDocumentHoldStill: "Hold still, capturing…",
    scanovateGuidePlaceFace: "Place your face in the oval",
    scanovateGuideMultipleFaces: "Only you should be in the frame",
    scanovateGuideFaceTooFar: "Move a little closer",
    scanovateGuideFaceTooClose: "Move back a little",
    scanovateGuideFaceOffCenter: "Center your face in the oval",
    scanovateGuideTurnToCamera: "Look straight at the camera",
    scanovateGuideFaceTooBright: "Avoid strong light on your face",
    scanovateGuideFaceBlurry: "Hold your camera steady",
    scanovateGuideFaceHoldStill: "Look at the camera and hold still",
    scanovateGuideHoldDocument: "Hold your ID next to your face",
    scanovateGuideRecording: "Recording, keep still",
    scanovateGuideKeepInView: "Keep your face and ID in view",
    scanovateGuideCaptured: "Got it",

    scanovateHelpTitle: "Tips for this step",
    scanovateHelpDocumentSurface: "Place your ID on a flat, dark surface.",
    scanovateHelpDocumentCorners: "Keep all four corners inside the frame.",
    scanovateHelpDocumentGlare:
        "If you see reflections, tilt the ID slightly or turn off lights above it.",
    scanovateHelpDocumentLight: "Move to a place with even light.",
    scanovateHelpShutter:
        "You can also take the photo yourself with the round button once the frame turns green.",
    scanovateHelpFaceLevel: "Hold the camera at eye level, about an arm’s length away.",
    scanovateHelpFaceCoverings: "Remove sunglasses, hats and face coverings.",
    scanovateHelpFaceLight: "Face a window or a lamp so your face is evenly lit.",
    scanovateHelpFaceAlone: "Make sure nobody else is in the frame.",
    scanovateHelpVideoDocument:
        "Hold the front of your ID next to your face, inside the small frame.",
    scanovateHelpVideoFace: "Keep your face inside the oval.",
    scanovateHelpVideoStill: "Stay still until the recording ends.",
    scanovateHelpClose: "Got it",

    scanovateStopTitle: "Stop verifying your identity?",
    scanovateStopText: "The photos taken so far will be discarded.",
    scanovateStopConfirm: "Stop",
    scanovateStopCancel: "Keep going",

    scanovateCheckingTitle: "Checking your identity",
    scanovateCheckingLead: "This usually takes a few seconds. Keep this page open.",
    scanovateCheckingReceived: "Photos and video received",
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
    scanovateTipCamera: "Allow camera access for this page when your browser asks.",
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
    scanovateMaxTrialsError:
        "The identity verification could not be completed after several tries. Please check your camera permissions and connection, and try again.",
    scanovateAttributesError:
        "The information extracted by our system doesn’t match the details you provided in the form. Try again or restart registration and double-check your entries.",
    scanovateScoringError:
        "The quality of the provided images needs to be improved. Please try again.",
    scanovateMaxRetriesError:
        "Your identity could not be verified after the maximum number of attempts, resulting in an unsuccessful enrollment.",
    scanovateCaptureInvalidError:
        "We couldn’t use the photos or the video. Please take them again.",

    scanovateConfirmTitle: "Check the details from your ID",
    scanovateConfirmLead:
        "We read these details from your {0}. They can’t be changed here; if something is wrong, scan your ID again.",
    scanovateConfirmDocument: "ID",
    scanovateConfirmSubmit: "Confirm and enroll",
    scanovateConfirmRetry: "Scan my ID again",
    scanovateLivenessFailedTitle: "The face check didn’t finish",
    scanovateLivenessFailedText:
        "Check your internet connection, stay in a well-lit place and try the face check again.",
    scanovateLivenessExpiredTitle: "Start the verification again",
    scanovateLivenessExpiredText:
        "The face check can’t be tried again from here. Start over to take the photos of your ID and check your face again.",
    scanovateStartOver: "Start over",
    scanovateUploadFailedTitle: "We couldn’t send your photos",
    scanovateUploadFailedText: "Check your connection and try again.",
    scanovateCaptureExpiredTitle: "Start the verification again",
    scanovateCaptureExpiredText:
        "Your photos can’t be sent from this page anymore. Start over to take them again.",
    scanovateCheckingReceivedLiveness: "Photos and face check received",
    scanovateLivenessError:
        "We could not confirm that it was you in front of the camera. Please try again.",
    scanovateLivenessChecking: "Checking your face, hold still…",
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

    scanovateIntroTitle: "Verifique su identidad",
    scanovateIntroLead:
        "Tomaremos fotos de su {0} y un vídeo corto de su cara para confirmar que es usted. Tarda unos 2 minutos.",
    scanovateIntroDocumentBothSides: "Anverso y reverso de su documento",
    scanovateIntroDocumentFrontSide: "Su documento",
    scanovateIntroDocumentHint:
        "Sobre una superficie plana y oscura, con las cuatro esquinas visibles.",
    scanovateIntroFace: "Su cara",
    scanovateIntroFaceHint: "Unos segundos mirando a la cámara.",
    scanovateIntroVideo: "Usted con su documento",
    scanovateIntroVideoHint: "Un vídeo corto con el documento junto a su cara.",
    scanovateIntroBeforeTitle: "Antes de empezar",
    scanovateIntroTipLight: "Busque un lugar bien iluminado.",
    scanovateIntroTipCoverings:
        "Quítese gafas de sol, gorros y cualquier cosa que le cubra la cara.",
    scanovateIntroTipDocument: "Tenga a mano su {0}.",
    scanovateIntroPrivacy:
        "Sus fotos y su vídeo se eliminan tras la verificación. Guardamos los datos de su documento.",
    scanovateIntroStart: "Empezar",

    scanovateCameraStarting: "Iniciando la cámara…",
    scanovateCameraDeniedTitle: "Permita el acceso a su cámara",
    scanovateCameraDeniedText:
        "Necesitamos su cámara para tomar las fotos. Permita el acceso a la cámara para este sitio en la configuración de su navegador y vuelva a intentarlo.",
    scanovateCameraNotFoundTitle: "No encontramos ninguna cámara",
    scanovateCameraNotFoundText:
        "Conecte una cámara, o abra esta página en un teléfono o una tableta con cámara, y vuelva a intentarlo.",
    scanovateCameraInUseTitle: "Su cámara está en uso",
    scanovateCameraInUseText:
        "Cierre otras aplicaciones o pestañas que estén usando la cámara y vuelva a intentarlo.",
    scanovateInsecureContextTitle: "No se puede usar la cámara en esta página",
    scanovateInsecureContextText:
        "Su navegador solo permite la cámara en páginas seguras. Abra de nuevo el enlace que recibió o contacte con soporte.",
    scanovateCameraFailedTitle: "No pudimos iniciar su cámara",
    scanovateCameraFailedText: "Compruebe que su cámara funciona y vuelva a intentarlo.",
    scanovateAnalyzerFailedTitle: "No pudimos preparar la comprobación de la cámara",
    scanovateAnalyzerFailedText: "Compruebe su conexión a internet y vuelva a intentarlo.",
    scanovateRecorderUnsupportedTitle: "Este navegador no puede grabar vídeo",
    scanovateRecorderUnsupportedText:
        "Abra esta página en una versión actualizada de Chrome, Safari, Firefox o Edge.",
    scanovateTryAgain: "Volver a intentarlo",
    scanovateBackToStart: "Volver al inicio",

    scanovateCaptureFrontTitle: "Anverso de su documento",
    scanovateCaptureBackTitle: "Reverso de su documento",
    scanovateCaptureFaceTitle: "Su cara",
    scanovateCaptureVideoTitle: "Usted con su documento",
    scanovateCaptureCounter: "{0} · {1} de {2}",
    scanovateCaptureFrontHeading: "Coloque el anverso de su documento en el marco",
    scanovateCaptureFrontText:
        "Use una superficie oscura, evite los reflejos y mantenga visibles las cuatro esquinas. La foto se toma automáticamente.",
    scanovateCaptureBackHeading: "Ahora el reverso de su documento",
    scanovateCaptureBackText: "Como antes: las cuatro esquinas dentro del marco y sin reflejos.",
    scanovateCaptureFaceHeading: "Mantenga su cara dentro del óvalo",
    scanovateCaptureFaceText:
        "Asegúrese de que su cara esté bien iluminada y descubierta. Tarda unos segundos.",
    scanovateCaptureVideoHeading: "Sostenga su documento junto a su cara",
    scanovateCaptureVideoText:
        "Mantenga su cara y el anverso de su documento a la vista hasta que termine la grabación.",
    scanovateChipFront: "Anverso",
    scanovateChipBack: "Reverso",
    scanovateChipFace: "Cara",
    scanovateCaptureSecure: "Verificación de identidad segura",
    scanovateTakePhoto: "Tomar foto",
    scanovateStop: "Detener y volver",
    scanovateHelp: "Ayuda",
    scanovateCaptured: "Foto tomada",
    scanovateRecording: "REC {0}",
    scanovateRecordingStatus: "Grabando",
    scanovateStepDone: "Hecho",
    scanovateStepCurrent: "En curso",

    scanovateGuideLoading: "Preparando…",
    scanovateGuidePlaceDocument: "Coloque su documento en el marco",
    scanovateGuideTurnDocument: "Dé la vuelta a su documento",
    scanovateGuideDocumentTooFar: "Acerque su documento",
    scanovateGuideDocumentTooClose: "Aleje un poco su documento",
    scanovateGuideDocumentNotAligned: "Alinee su documento con el marco",
    scanovateGuideTooDark: "Busque un lugar con más luz",
    scanovateGuideDocumentTooBright: "Aléjese de la luz directa",
    scanovateGuideGlare: "Incline su documento para quitar el reflejo",
    scanovateGuideDocumentBlurry: "Sujételo firme para que se vea nítido",
    scanovateGuideDocumentHoldStill: "No se mueva, capturando…",
    scanovateGuidePlaceFace: "Coloque su cara en el óvalo",
    scanovateGuideMultipleFaces: "Solo usted debe aparecer en la imagen",
    scanovateGuideFaceTooFar: "Acérquese un poco",
    scanovateGuideFaceTooClose: "Aléjese un poco",
    scanovateGuideFaceOffCenter: "Centre su cara en el óvalo",
    scanovateGuideTurnToCamera: "Mire de frente a la cámara",
    scanovateGuideFaceTooBright: "Evite la luz fuerte sobre su cara",
    scanovateGuideFaceBlurry: "Sujete la cámara firme",
    scanovateGuideFaceHoldStill: "Mire a la cámara y no se mueva",
    scanovateGuideHoldDocument: "Sostenga su documento junto a su cara",
    scanovateGuideRecording: "Grabando, no se mueva",
    scanovateGuideKeepInView: "Mantenga su cara y su documento a la vista",
    scanovateGuideCaptured: "Listo",

    scanovateHelpTitle: "Consejos para este paso",
    scanovateHelpDocumentSurface: "Coloque su documento sobre una superficie plana y oscura.",
    scanovateHelpDocumentCorners: "Mantenga las cuatro esquinas dentro del marco.",
    scanovateHelpDocumentGlare:
        "Si ve reflejos, incline un poco el documento o apague las luces que tenga encima.",
    scanovateHelpDocumentLight: "Busque un lugar con luz uniforme.",
    scanovateHelpShutter:
        "También puede tomar la foto usted mismo con el botón redondo cuando el marco se ponga verde.",
    scanovateHelpFaceLevel: "Sujete la cámara a la altura de los ojos, a un brazo de distancia.",
    scanovateHelpFaceCoverings:
        "Quítese gafas de sol, gorros y cualquier cosa que le cubra la cara.",
    scanovateHelpFaceLight:
        "Póngase frente a una ventana o una lámpara para que su cara esté bien iluminada.",
    scanovateHelpFaceAlone: "Asegúrese de que no aparece nadie más en la imagen.",
    scanovateHelpVideoDocument:
        "Sostenga el anverso de su documento junto a su cara, dentro del marco pequeño.",
    scanovateHelpVideoFace: "Mantenga su cara dentro del óvalo.",
    scanovateHelpVideoStill: "No se mueva hasta que termine la grabación.",
    scanovateHelpClose: "Entendido",

    scanovateStopTitle: "¿Dejar de verificar su identidad?",
    scanovateStopText: "Se descartarán las fotos tomadas hasta ahora.",
    scanovateStopConfirm: "Detener",
    scanovateStopCancel: "Continuar",

    scanovateCheckingTitle: "Comprobando su identidad",
    scanovateCheckingLead: "Suele tardar unos segundos. Mantenga esta página abierta.",
    scanovateCheckingReceived: "Fotos y vídeo recibidos",
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
    scanovateTipCamera: "Permita el acceso a la cámara cuando su navegador lo pida.",
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
    scanovateMaxTrialsError:
        "La verificación de identidad no pudo completarse tras varios intentos. Compruebe los permisos de la cámara y su conexión, y vuelva a intentarlo.",
    scanovateAttributesError:
        "Los datos extraídos de su documento no coinciden con los que introdujo en el formulario. Vuelva a intentarlo o reinicie la inscripción y revise sus datos.",
    scanovateScoringError: "Hay que mejorar la calidad de las imágenes. Vuelva a intentarlo.",
    scanovateMaxRetriesError:
        "No se pudo verificar su identidad tras el número máximo de intentos, por lo que la inscripción no se ha completado.",
    scanovateCaptureInvalidError: "No pudimos usar las fotos o el vídeo. Vuelva a tomarlos.",

    scanovateConfirmTitle: "Revise los datos de su documento",
    scanovateConfirmLead:
        "Hemos leído estos datos de su {0}. No se pueden cambiar aquí; si algo no es correcto, vuelva a escanear su documento.",
    scanovateConfirmDocument: "Documento",
    scanovateConfirmSubmit: "Confirmar e inscribirme",
    scanovateConfirmRetry: "Volver a escanear mi documento",
    scanovateLivenessFailedTitle: "La comprobación facial no terminó",
    scanovateLivenessFailedText:
        "Compruebe su conexión a internet, busque un lugar bien iluminado y vuelva a intentar la comprobación facial.",
    scanovateLivenessExpiredTitle: "Vuelva a empezar la verificación",
    scanovateLivenessExpiredText:
        "La comprobación facial no se puede repetir desde aquí. Vuelva a empezar para fotografiar su documento y comprobar su cara de nuevo.",
    scanovateStartOver: "Volver a empezar",
    scanovateUploadFailedTitle: "No hemos podido enviar sus fotos",
    scanovateUploadFailedText: "Compruebe su conexión y vuelva a intentarlo.",
    scanovateCaptureExpiredTitle: "Vuelva a empezar la verificación",
    scanovateCaptureExpiredText:
        "Sus fotos ya no se pueden enviar desde esta página. Vuelva a empezar para tomarlas de nuevo.",
    scanovateCheckingReceivedLiveness: "Fotos y comprobación facial recibidas",
    scanovateLivenessError:
        "No pudimos confirmar que era usted quien estaba frente a la cámara. Vuelva a intentarlo.",
    scanovateLivenessChecking: "Comprobando su cara, no se mueva…",
    scanovateFaceMismatchError: "La cara de sus fotos no coincide con la foto de su documento.",
    scanovateFaceNotFoundError:
        "No encontramos su cara en su documento ni en la foto en la que lo sostiene. Asegúrese de que la foto de su documento se vea bien y no esté tapada.",
    dateOfBirth: "Fecha de nacimiento",
    "sequent.read-only.id-card-number": "Número de documento",
}
