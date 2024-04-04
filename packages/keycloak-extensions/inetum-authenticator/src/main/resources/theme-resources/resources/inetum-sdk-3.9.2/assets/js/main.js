// DobModels - Steps
import { InitialStep, InstructionsStep, DocCaptureStep, FaceCaptureStep, VideoIdentificationStep, AttachStep, EndStep } from './dob-models-1.1.19.esm.js';
// DobModels - Enums
import { EventType, InstructionsResourceType, DocSide, VideoType, Evidence, VoiceLanguage, ExceptionType, IOSBrowser, CountryCode } from './dob-models-1.1.19.esm.js';
// DobModels - Utils
import { SDKUtils } from './dob-models-1.1.19.esm.js';

class LocalBroadcastManager {
  onReceive(message) {
    if (message.level === 'BAM' || message.level === 'ERROR') {
      console.log('[TAG] ' + message.tag + ' [LEVEL] ' + message.level + ' [MESSAGE] ' + message.message);
    }
  }
}

let design = 'generic';
/*
  // Ejemplo con responsive
  let design = 'responsive';
*/
let isPassportFlow = false;
/*
  // Ejemplo con pasaporte (revisar tambien estilos de ejemplo en dob-style.css y descomentarlos)
 let isPassportFlow = true;
*/
let info_title;
let info_description;

let sdk;          // Configuration parameters for DoBSDK (obligatorio)
let session;      // Session parametes (obligatorio)
let env_config;   // Configuration parametres for Environments

function flow() {
  if (design === 'generic') {
    if (isPassportFlow === true) {
      // Esto simplemente es un ejemplo en caso de ser un flujo para pasaporte
      return [
        new InitialStep('permissions-passport'),
        new InstructionsStep('instructions-passport', 'instructions_face_passport.gif', InstructionsResourceType.image, -1),
        new DocCaptureStep('passport-capture', DocSide.front, Evidence.imgPassport, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, -1),
        new DocCaptureStep('certificate-capture', DocSide.front, Evidence.imgEUResidenceCertificate, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, -1),
        new InstructionsStep('instructions-face', SDKUtils.isMobile() ? 'videoidentification_white' : 'videoidentification_desktop', InstructionsResourceType.video, -1),
        new VideoIdentificationStep('show_front', 'user', VideoType.webrtc, DocSide.front, Evidence.imgPassport, 22),
        new VideoIdentificationStep('show_front', 'user', VideoType.webrtc, DocSide.front, Evidence.imgEUResidenceCertificate, 22),
        new FaceCaptureStep('face-capture', 'user', VideoType.photo, 30),
        new EndStep('end-passport')
      ];
    } else {
      // Esto simplemente es un ejemplo en caso de ser un flujo para dni
      return [
        new InitialStep('permissions'),
        new InstructionsStep('show-doc-front', SDKUtils.isMobile() ? 'showfrontdesktop' : 'showfrontdesktop', InstructionsResourceType.video, -1),
        new DocCaptureStep('front-capture', DocSide.front, Evidence.imgDocFront, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, 10),
        new DocCaptureStep('back-capture', DocSide.back, Evidence.imgDocReverse, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, 10),
        new InstructionsStep('instructions-face', SDKUtils.isMobile() ? 'videoidentification_white' : 'videoidentification_desktop', InstructionsResourceType.video, -1),
        new VideoIdentificationStep('show_front', 'user', VideoType.webrtc, DocSide.front, Evidence.imgDocFront, 22),
        new VideoIdentificationStep('show_back', 'user', VideoType.webrtc, DocSide.back, Evidence.imgDocReverse, 22),
        new FaceCaptureStep('face-capture', 'user', VideoType.photo, 30),
        new EndStep('EndStep')
      ];
    }
  } else if (design === 'responsive') {
    // Por ejemplo: si el ciudadano no es de la UE pedimos el Documento de Identidad, en caso contrario pedimos pasaporte y certificado de residencia
    const isEU = false;
    return [
      isEU ? new AttachStep('attach_passport', Evidence.imgPassport, true, true) : new AttachStep('attach_front', Evidence.imgDocFront, true, true),
      isEU ? new AttachStep('attach_ue', Evidence.imgEUResidenceCertificate, true, true) : new AttachStep('attach_back', Evidence.imgDocReverse, true, true),
      //new AttachStep('attach_video', Evidence.videoSelfie, false, false),
      new AttachStep('attach_selfie', Evidence.imgSelfie, false, true),
      new EndStep('EndStep')
    ];
  }
}

function setStringHtmlValues(title, description) {
  info_title = document.getElementById('info_title');
  info_description = document.getElementById('info_description');
  info_title.innerHTML = title;
  info_description.innerHTML = description;
}

function removeMessagesAttach() {
  const messages = document.getElementsByClassName('dob-attach-messages')[0];
  if (messages) {
    messages.parentElement.removeChild(messages);
  }
}

// Actualizamos DOM | para el ejemplo en caso de responsive
if (design === 'generic') {
  removeMessagesAttach();
}

// Load custom strings
let myStrings = {
  // DOB API
  'dob_api_unknown_error': 'Error desconocido',
  'dob_api_incorrect_parameters': 'Parámetros de entrada incorrectos',
  'dob_api_incorrect_token': 'Token proporcionado incorrecto',
  'dob_api_unknown_uid': 'Identificador desconocido',
  'dob_api_not_available_evidence': 'Evidencia no disponible',
  'dob_api_incorrect_upload': 'Documento no recibido correctamente',
  'dob_api_empty_request': 'Resultado de consulta vacío',
  'dob_api_unknown_action': 'Acción no reconocida',
  'dob_api_so_big_evidence': 'Tamaño de evidencia demasiado grande',
  'dob_api_evidences_already_uploaded': 'Evidencias ya cargadas',
  'dob_api_not_identify_document': 'No se identifica el tipo de documento',
  'dob_api_invalid_evidence': 'Tipo de evidencia no permitida',
  'dob_api_back_doc_not_match': 'La cara del documento no se corresponde con la evidencia enviada',
  'dob_api_expired_document': 'Documento caducado',
  'dob_api_bad_quality_doc': 'Calidad de imagen baja',
  'dob_api_config_not_enable': 'Configuración no habilitada',
  'dob_api_transaction_not_finish': 'La transacción aún no ha sido procesada',
  'dob_api_not_available_zip': 'ZIP no disponible',
  'dob_api_incorrect_mrz': '¡Vaya! Parece que la captura no tiene la calidad suficiente. Por favor, repita la captura.',
  'dob_api_transaction_exists': 'Proceso de alta ya existente.',
  'dob_api_younger': 'El documento adjunto corresponde a un usuario menor de edad',
  'dob_api_incorrect_evidence': 'La evidencia no se corresponde con la evidencia solicitada.',
  'dob_api_incorrect_document': 'Modelo de documento no admitido',
  // INITIAL STEP
  'initial_title_mobile': 'Identificación digital',
  'initial_description_mobile': 'El proceso de identificación es muy sencillo. Sólo necesitas tener a mano tu documento de identidad. Te guiaremos para capturar el anverso y reverso, y luego tendrás que hacerte un vídeo-selfie con el documento en la mano. Busca un lugar bien iluminado y... ¡sonríe!',
  'initial_button_mobile': 'Comenzar',
  // INSTRUCTIONS STEP
  'instructions_step_title': 'Instrucciones Vídeo',
  'instructions_step_description': 'Ahora vamos a grabarte en vídeo. Tendrás que mostrar tu documento. Después tomaremos un selfie y tendrás que hacer coincidir los dos óvalos que verás en pantalla. Busca un sitio bien iluminado. En el vídeo solo podrá aparecer la persona que se está identificando.',
  'instructions_step_button': 'Continuar',
  // DOC-CAPTURE STEP
  'card_detector_evidence_not_focus': 'La imagen está desenfocada',
  'card_detector_no_detect_docs': 'No estamos detectando nada...',
  'card_detector_in_progress': 'Escaneando...',
  'card_detector_adjust_doc': 'Ajusta un poco más el documento...',
  'card_detector_more_near_doc': 'Acerca el documento...',
  'card_detector_automatic_help_text': 'Ajusta el documento para hacer la captura automática',
  'card_detector_manual_help_text': 'Ajusta el documento para hacer la captura manual',
  'card_detector_passport_manual_help_text': 'Ajusta el pasaporte para hacer la captura',
  'card_detector_certificate_manual_help_text': 'Ajusta el certificado de residencia para hacer la captura',
  // VIDEO IDENTIF. STEP
  'video_identification_message_front': 'Muestra el anverso del documento',
  'video_identification_message_back': 'Da la vuelta al documento',
  'video_identification_message_passport': 'Muestra el pasaporte',
  'video_identification_message_certificate': 'Muestra el certificado',
  // FACE-CAPTURE STEP
  'permissions_orientation_dialog_title': 'Para mejorar tu experiencia, necesitamos que nos proporciones acceso a la orientación de tu teléfono',
  'permissions_orientation_dialog_button': 'OK',
  'face_capture_device_motion_down': 'Baja el dispositivo',
  'face_capture_device_motion_up': 'Eleva el dispositivo',
  'face_capture_detection_closer': 'Acércate',
  'face_capture_detection_further': 'Aléjate',
  'face_capture_detection_multi_face': 'Solo una cara por favor',
  // DOC-CAPTURE STEP & IDEO IDENTIF. STEP & FACE-CAPTURE STEP
  'media_recorder_unknown_exception': 'El dispositivo o navegador no soporta la grabación de vídeo',
  'media_recorder_experimental_feature_exception': 'La grabación de vídeo no está habilitada en su dispositivo. Por favor, vaya a Ajustes>Safari>Avanzado>Experimental Features y active el flag MediaRecorder',
  // ATTACH STEP
  'attach_instructions_text': 'Arrastra y suelta el archivo aquí o',
  'attach_button_text': 'Busca el archivo',
  'attach_file_successfully_submited': '¡Tu archivo ha sido subido con éxito!',
  'attach_on_uploading_file': 'Subiendo el archivo, un momento por favor',
  'attach_multi_files_upload_error': 'Solo se permite subir un archivo',
  'attach_invalid_file_type_error': 'Hubo un error leyendo este archivo',
  'attach_processing_file': 'Procesando archivo...',
  'attach_default_error': 'Parece que el archivo adjunto no cumple con los criterios necesarios',
  'attach_error_button_text': 'REINTENTAR',
  'attach_not_supported_file': 'Tipo de archivo no permitido',
  // OTP STEP
  'otp_verification_title': 'Verificación OTP',
  'otp_verification_email': 'Te hemos enviado el código de acceso por correo electrónico para la verificación.',
  'otp_verification_sms': 'Te hemos enviado el código de acceso por SMS para la verificación.',
  'otp_verification_resend_question': '¿No has recibido el código OTP?',
  'otp_verification_resended_email': 'Se ha enviado un nuevo código OTP a su dirección de correo electrónico.',
  'otp_verification_resended_sms': 'Se ha enviado un nuevo código OTP a su número de teléfono móvil.',
  'otp_verification_invalid': 'El código OTP no es válido',
  'otp_verification_expired': 'El código OTP ha expirado',
  // END STEP
  'end_title': 'Identificación digital',
  'end_description': 'El proceso de identificación es muy sencillo. Sólo necesitas tener a mano tu documento de identidad y capturar con la cámara del móvil lo siguiente:',
  'end_subtitle': '¡Proceso finalizado!',
  'end_button_text': 'Finalizar',
  // INITIAL STEP & END STEP
  'intro_row_obverse': 'Anverso del documento',
  'intro_row_reverse': 'Reverso del documento',
  'intro_row_face': 'Rostro e Identidad',
  // EXCEPTION VIEW
  'exception_button_text_retry': 'Reintentar',
  'exception_button_text_go_init': 'Volver',
  'new_flow_exception_tips': [
    'No se pudo iniciar un nuevo proceso de identificación desde el dispositivo.',
    'Por favor, pulse reintentar o inténtelo de nuevo más tarde'
  ],
  'not_readable_exception_tips': [
    'Tu dispositivo de audio/vídeo está siendo utilizado por otra aplicación.',
    'Por favor, cierra la aplicación que está utilizando tu dispositivo y reinicia el proceso.'
  ],
  'recording_exception_tips': [
    'El dispositivo o navegador no soporta la grabación de vídeo',
    'Los navegadores que soportan la grabación de vídeo son: ',
    [
      'Chrome Desktop (>v49)',
      'Firefox Desktop (>v29)',
      'Edge Desktop(>v76)',
      'Safari Desktop (>v13)',
      'Opera Desktop (>v62)',
      'Chrome for Android'
    ]
  ],
  'connection_generic_error_tips': [
    '¡Vaya! Parece que hubo un error al intentar establecer conexión con el servidor',
    'Por favor, revisa tu conexión a internet o vuelve a intentarlo en cinco minutos.'
  ],
  'unknown_media_exception_tips': [
    'Hubo un error desconocido al intentar acceder a tu dispositivo de vídeo/audio',
    'Por favor, reinicia el proceso de video-identificación'
  ],
  'unknown_attach_exception_tips': [
    'Hubo un error desconocido al adjuntar la evidencia',
    'Por favor, reinténtelo de nuevo más tarde'
  ],
  'webrtc_exception_tips': [
    'Se ha interrumpido la conexión con el servidor de streaming.',
    [
      'Es posible que ocurriera un fallo de red que provocó la perdida.',
      'Por favor, vuelve a intentarlo en unos momentos.'
    ]
  ],
  'global_timeout_exception_tips': [
    'Se ha excedido el tiempo total de flujo',
    'Por favor, pulse reintentar para reiniciar el flujo'
  ],
  'no_devices_found_tips': [
    'No se ha encontrado ningún dispositivo de audio/vídeo.',
    'Por favor, conecta un dispositivo y reinicia el proceso.'
  ],
  'upload_exception_tips': [
    'Se produjo un error de conexión mientras se enviaba la evidencia de identificación.',
    [
      'Es posible que ocurriera un fallo de red que provocó la perdida.',
      'Por favor, vuelve a intentarlo en unos momentos.'
    ]
  ],
  'upload_check_exception_tips': [
    'Se produjo un error al verificar el documento. Esto podría deberse a:',
    [
      'La imagen no tiene la calidad suficiente. Recuerde que debe estar bien enfocada.',
      'El documento no se identifica como un tipo válido.',
      'El documento está caducado',
      'La cara del documento no se corresponde con la evidencia enviada'
    ],
    'Por favor, prueba de nuevo haciendo lo siguiente:',
    [
      'Coloca el documento de identidad en una superficie plana y luminosa',
      'Asegurate de que tienes suficiente luz',
      'Asegúrate de que no hay brillos o zonas oscuras en el documento',
      'Haz coincidir la silueta dibujada en la pantalla con la imagen de tu cámara.'
    ]
  ],
  'face_detector_timeout_exception_tips': [
    '¡Vaya! Algo no va bien, no conseguimos capturar su rostro.',
    'Por favor, prueba de nuevo haciendo lo siguiente:',
    [
      'Colócate en un lugar luminoso',
      'Procura colocar tu rostro dentro de la guía que aparece en la pantalla',
      'Mira de frente a la cámara.'
    ]
  ],
  'manual_face_detector_exception_tips': [
    'No se ha detectado una cara en la foto selfie:',
    [
      'Por favor, enfocate a la cara al hacerte la foto.',
    ]
  ],
  'manual_multi_face_detector_exception_tips': [
    'Se han detectado varias caras en la foto selfie:',
    [
      'Por favor, solo puede aparecer una cara.',
    ]
  ],
  'card_detector_timeout_exception_tips':
    [
      '¡Vaya! Algo no va bien, no conseguimos capturar tu documento de identidad.',
      'Por favor, prueba de nuevo haciendo lo siguiente:',
      [
        'Situa el documento de identidad en una superficie plana y luminosa.',
        'Haz coincidir la silueta dibujada en la pantalla con la imagen de tu cámara.'
      ]
    ],
  'not_allowed_permission_exception_tips': [
    'No hemos podido acceder al dispositivo porque no has permitido el acceso a la cámara y/o al micrófono.',
    'Por favor, habilita los permisos de cámara y audio y reinicia el proceso.'
  ],
  'overconstraint_exception_tips': [
    'Tu dispositivo no cumple los requisitos mínimos para realizar el proceso.',
    'Por favor, cambie de dispositivo y repita el proceso'
  ],
  'invalid_flow_exception': [
    'El flujo configurado es inválido.',
    'Por favor, contacte con el administrador del sistema.'
  ],
  'unsupported_browser_exception_tips': [
    'Navegador no compatible con el VideoStreaming.', 'Por favor, utilice otro como Chrome o Firefox.'
  ],
  'unsupported_browser_exception_ios_tips': [
    'Navegador no compatible con el VideoStreaming.', 'Por favor, utilice Safari.'
  ],
  'unupdate_browser_exception_tips': [
    'La versión de tú navegador no es compatible.',
    'Por favor, actualiza tu navegador y vuelve a intentarlo'
  ],
  'dob_api_face_too_close_tips': [
    'La cara está demasiado cerca en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_eyes_closed_tips': [
    'El selfie ha salido con los ojos cerrados.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_face_close_to_border_tips': [
    'La cara está demasiado cerca del borde.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_face_cropped_tips': [
    'La cara sale cortada en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_face_is_occluded_tips': [
    'La cara sale tapada en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_face_not_found_tips': [
    'No detectamos cara en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_too_many_faces_tips': [
    'Hay demasiadas caras en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_face_too_small_tips': [
    'La cara es demasiado pequeña en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_face_angle_too_large_tips': [
    'Se ha inclinado la cámara en el selfie.',
    'Centra la cara en el óvalo al hacer la foto y asegúrate de que se te ve bien.'
  ],
  'dob_api_non_configured_otp_contact_method': [
    'Método de contacto OTP no configurado.',
    'Por favor, contacte con el administrador del sistema.'
  ],
  'dob_api_maximum_number_of_otp_forwards_has_been_exceeded': [
    'Se ha superado el número máximo de reenvíos OTP.'
  ],
  'dob_api_maximum_number_of_otp_reintent_has_been_exceeded': [
    'Se ha superado el número máximo de reintentos OTP.'
  ],
  'dob_api_contact_method_does_not_exist': [
    'Método de contacto no existe.',
    'Por favor, contacte con el administrador del sistema.'
  ],
  'dob_api_mandatory_otp_phone_number_not_informed': [
    'Número de teléfono OTP obligatorio, no informado.',
    'Por favor, contacte con el administrador del sistema o vuelva a intentarlo.'
  ],
  'dob_api_mandatory_otp_email_not_informed': [
    'Correo electrónico OTP obligatorio, no informado.',
    'Por favor, contacte con el administrador del sistema o vuelva a intentarlo.'
  ],
  'dob_api_non_valid_otp_phone_number': [
    'Número de teléfono OTP no válido.',
    'Por favor, contacte con el administrador del sistema o vuelva a intentarlo.'
  ],
  'dob_api_non_valid_otp_email': [
    'Correo electrónico OTP no válido.',
    'Por favor, contacte con el administrador del sistema o vuelva a intentarlo.'
  ],
  'dob_api_otp_has_already_been_validated': [
    'El OTP ya ha sido validado.',
    'Por favor, contacte con el administrador del sistema.'
  ],
  'dob_api_transaction_does_not_exist': [
    'La transacción no existe.',
    'Por favor, contacte con el administrador del sistema o vuelva a intentarlo.'
  ],
  // HELP DIALOG VIEW
  'secondarytoolbar_help_title': 'INFORMACIÓN',
  'default_instructions_docs': [
    'Sitúa el documento de identidad en una superficie plana y luminosa',
    'El documento no debe tener brillos o reflejos que puedan dificultar la lectura',
    'Haz coincidir la silueta dibujada en la pantalla con la imagen de tu cámara',
    'La captura se realizará de forma automática cuando silueta e imagen coincidan'
  ],
  'default_instructions_face': [
    'Colócate en un lugar luminoso',
    'Procura colocar tu rostro dentro de la guía que aparece en la pantalla',
    'Mira de frente a la cámara',
    'Recuerda enseñar el documento de identidad por las dos caras',
    'Recuerda que no deben aparecer más personas en el vídeo'
  ],
  'secondarytoolbar_help_button': 'CERRAR',
  // MANUAL CAPTURE VIEW
  'manual_capture_doc_title_text': 'Modo de captura manual',
  'manual_capture_doc_lead_text': 'Encuadra el documento y pulsa el botón',
  'manual_capture_face_title_text': 'Modo de captura selfie manual',
  'manual_capture_face_lead_text': 'Enfocate a la cara para hacerte un selfie y haz click en el botón para capturar la foto',
  // PREVIEW VIEW
  'attach_preview_retry_button': 'Repetir',
  'attach_preview_continue_button': 'Continuar',
  // PREVIEW VIEW | ONLY DESIGN GENERIC
  'preview_capture_doc_text': 'Compruebe que la foto es legible, no se ve borrosa, está bien enfocada y no tiene brillos',
  // PREVIEW VIEW | ONLY DESIGN RESPONSIVE
  'attach_preview_text': 'Recuerde que la imagen debe estar orientada correctamente y que debe visualizarse correctamente',
  // LOADER VIEW
  'default_progress_description': 'Conectando...',
  'video_progress_description': 'Conectando...',
  'end_progress_description': 'Finalizando...',
  'new_device_progress_description': 'Iniciando nuevo flujo desde dispositivo...',
  'otp_configuration_progress_description': 'Cargando configuración...',
  'otp_forwarding_progress_description': 'Enviando OTP...',
  'otp_verification_progress_description': 'Verificando OTP...',
  // TOOLBAR COMPONENT
  'secondarytoolbar_identification_error': 'Error de identificación',
  'secondarytoolbar_obverse': 'Anverso del documento',
  'secondarytoolbar_reverse': 'Reverso del documento',
  'secondarytoolbar_face': 'Rostro e Identidad',
  'secondarytoolbar_passport': 'Pasaporte',
  'secondarytoolbar_certificate': 'Certificado de residencia',
  'secondarytoolbar_exit_button': 'Salir',
  // INFOBAR COMPONENT
  'card_detector_verifying': 'Verificando documento...',
  'infobar_start_text': 'Configurando escáner',
  'infobar_working_card_capture_text': 'Coloca el documento de identidad en una superficie plana y luminosa',
  'infobar_uploading_text': 'Enviando archivos...',
  'infobar_finish_text': '¡Perfecto!',
  'infobar_passport_start_text': 'Coloca el pasaporte en una superfice plana y luminosa',
  'infobar_passport_working_card_capture_text': 'Coloca el pasaporte en una superfice plana y luminosa',
  'infobar_passport_uploading_text': 'Estamos enviando la fotografía',
  'infobar_passport_finish_text': '¡Perfecto!',
  'infobar_passport_end_text': 'Ya tenemos el pasaporte',
  'infobar_certificate_start_text': 'Coloca el certificado en una superfice plana y luminosa',
  'infobar_certificate_working_card_capture_text': 'Coloca el certificado en una superfice plana y luminosa',
  'infobar_certificate_uploading_text': 'Estamos enviando la fotografía',
  'infobar_certificate_finish_text': '¡Perfecto!',
  'infobar_certificate_end_text': 'Ya tenemos el certificado',
  'infobar_working_face_capture_text': 'Procura colocar tu rostro dentro de la guía que aparece en la pantalla',
  'infobar_video_identification_front_text': 'Coge el documento de identidad por el borde, enseñando la parte delantera y colócate para el vídeo',
  'infobar_video_identification_back_text': 'Ahora enseña la parte trasera del documento de indentidad. No olvides cogerlo por el borde',
  'infobar_face_detector_verifying': 'Verificando selfie...',
  // SPEECH SYNTHESIS
  'speech_synthesis_capture_doc_front': 'Haz que coincida la parte delantera de tu documento con la silueta',
  'speech_synthesis_capture_doc_front_manual': 'Haz una foto de la parte delantera de tu documento',
  'speech_synthesis_capture_doc_front_qr': 'Utilice el móvil, para escanear el código qr y hacer una foto del anverso del documento',
  'speech_synthesis_capture_doc_back': 'Haz que coincida la parte trasera de tu documento con la silueta',
  'speech_synthesis_capture_doc_back_manual': 'Haz una foto de la parte trasera de tu documento',
  'speech_synthesis_capture_doc_back_qr': 'Ahora con el móvil, realiza una foto del reverso del documento',
  'speech_synthesis_capture_doc_passport': 'Haz una foto del pasaporte',
  'speech_synthesis_capture_doc_passport_qr': 'Utilice el móvil, para escanear el código qr y hacer una foto del pasaporte',
  'speech_synthesis_capture_doc_certificate': 'Haz una foto del certificado de residencia',
  'speech_synthesis_capture_doc_certificate_qr': 'Ahora con el móvil, realiza una foto del certificado',
  'speech_synthesis_capture_doc_finish_qr': 'Continua el proceso por este dispositivo',
  'speech_synthesis_video_identification_front': 'Muestra la parte delantera del documento',
  'speech_synthesis_video_identification_back': 'Muestra la parte trasera del documento',
  'speech_synthesis_video_identification_passport': 'Muestra el pasaporte',
  'speech_synthesis_video_identification_certificate': 'Muestra el certificado',
  'speech_synthesis_face_capture': 'Prepárate para hacerte un selfie',
  'speech_synthesis_face_capture_manual': 'Hazte un selfie haciendo click en el botón de captura',
  'speech_synthesis_attach_doc_front': 'Adjunta una foto del anverso del documento',
  'speech_synthesis_attach_doc_back': 'Adjunta una foto del reverso del documento',
  'speech_synthesis_attach_video_identification': 'Adjunta un vídeo identificador con tu rostro',
  'speech_synthesis_attach_face': 'Adjunta una foto de tu rostro',
  'speech_synthesis_attach_passport': 'Adjunta una foto del pasaporte',
  'speech_synthesis_attach_ue': 'Adjunta una foto del certificado de residencia de la unión europea',
  // HELP ORIENTATION
  'helporientation_title': 'Por favor cambia la orientación a vertical',
  'helporientation_button': 'CERRAR',
  // HELP PERMISSIONS VIEW
  'help_permissions_title': 'Permisos cámara y micrófono',
  'help_permissions_description': 'Necesitamos que aceptes los permisos para acceder a la cámara y al micrófono. Son imprescindibles para realizar la videoidentificación.',
  // EXIT DIALOG VIEW
  'exit_dialog_title': 'Aviso',
  'exit_dialog_subtitle': '¿Desea cancelar el proceso?',
  'exit_dialog_accept': 'Sí',
  'exit_dialog_cancel': 'No',
  // QR VIEW
  'qr_capture_doc_front': 'Escanea el código QR con el móvil para hacer una foto del anverso del documento.',
  'qr_capture_doc_back': 'Ahora con el móvil realiza una foto del reverso del documento.',
  'qr_connect_error': 'Problemas de conexión, por favor vuelve a escanear el código QR en unos momentos.',
  // CUSTOM STRINGS INSTRUCTIONS STEP
  'custom_instructions_doc_title': 'Identificación Digital',
  'custom_instructions_doc_description': 'A continuación vamos a capturar el anverso y el reverso. Si es posible, busca un fondo oscuro y apoya el documento en una superficie plana.',
  'custom_instructions_doc_button': 'Continuar',
  // CUSTOM STRINGS INITIAL/END STEP - EXAMPLE: PASSPORT
  'custom_intro_row_obverse': 'Pasaporte',
  'custom_intro_row_reverse': 'Certificado de residencia',
  // CUSTOM STRINGS INSTRUCTIONS STEP - EXAMPLE: PASSPORT
  'custom_instructions_step_title': 'Instrucciones Pasaporte',
  'custom_instructions_step_description': 'Primero comenzaremos con la captura del pasaporte.'
};

// Configuramos el SDK
sdk = {
  ak: window.DOB_API_KEY,
  flow: flow()
};

env_config = eval("(" + window.DOB_ENV_CONFIG + ")");

session = {
  tokenDOB: window.DOB_DATA.td,
  userID: window.DOB_DATA.uid
};

let dobSdk = document.getElementById("dob-sdk");
dobSdk.session = session;
dobSdk.sdk = sdk;
dobSdk.env_config = env_config;

// Listen to status changes
dobSdk.addEventListener("status", status => {
  const parsedStatus = status.detail;
  // No face falta filtrar por tipo, es solo para el ejemplo.
  // Esto simplemente es un ejemplo para imprimir la INFO que enviamos en cada paso del SDK.
  switch (parsedStatus.type) {
    case EventType.load:
      console.log('load: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.working:
      console.log('working: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.uploadEvidenceStart:
      console.log('Upload evidence start: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.uploadEvidenceEnd:
      console.log('Upload evidence end: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.verifying:
      console.log('Verifying: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.error:
      console.log('error: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.exception:
      console.log('exception: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      break;
    case EventType.end:
      console.log('End: ' + parsedStatus.step + ' extra-info: ' + parsedStatus.info);
      // Ocultamos strings de flujo responsive al terminar de subir el selfie
      if (parsedStatus.step === 'attach_selfie') {
        removeMessagesAttach();
      }
      break;
  }
  // No face falta filtrar por step, es solo para el ejemplo.
  // Esto simplemente es un ejemplo para cambiar strings en caso de querer crearlos si usamos el diseño responsive.
  switch (parsedStatus.step) {
    case 'attach_front':
      setStringHtmlValues('Anverso del documento', 'Adjunta una foto de tu Documento de Identidad boca arriba. Asegúrate de que la imagen tenga calidad suficiente y se vea bien la información y la foto que contiene.');
      break;
    case 'attach_back':
      setStringHtmlValues('Reverso del documento', 'Ahora adjunta una foto de tu Documento de Identidad boca abajo. Asegúrate de que la imagen tenga calidad suficiente y se vea bien la información.');
      break;
    case 'attach_passport':
      setStringHtmlValues('Pasaporte', 'Por favor, adjunta una imagen de tu pasaporte. Asegúrate de que la imagen se vea correctamente y tenga calidad suficiente.');
      break;
    case 'attach_ue':
      setStringHtmlValues('Certificado de Residentes de la Unión Europea', 'Ahora necesitamos que adjuntes tu certificado de residente en la unión europea.');
      break;
    case 'attach_video':
      setStringHtmlValues('Video de tu rostro', 'Ahora, tome un video de su cara y adjunte el video para que podamos verificar que el documento le pertenece. ');
      break;
    case 'attach_selfie':
      setStringHtmlValues('Foto de tu rostro', 'Ahora, hazte un selfie de tu rostro y adjunta la foto para que podamos verificar que el documento te pertenece. Asegúrate de que la imagen tenga calidad suficiente.');
      break;
  }
  // Esto simplemente es un ejemplo para detectar que el último paso a terminado [En caso de no informar el EndStep ni urls de callbacks]
  if (parsedStatus.type === EventType.end && parsedStatus.step === sdk.flow[sdk.flow.length - 1].getStepName()) {
    console.log('End: ' + parsedStatus.step + ' - is last step, do something...');
  }
  // Esto simplemente es un ejemplo de como cambiar los textos en la vista de instrucciones en caso del documento, según el nombre del step
  if (parsedStatus.step === 'show-doc-front') {
    myStrings['instructions_step_title'] = myStrings['custom_instructions_doc_title'];
    myStrings['instructions_step_description'] = myStrings['custom_instructions_doc_description'];
    myStrings['instructions_step_button'] = myStrings['custom_instructions_doc_button'];
    dobSdk.env_config ? dobSdk.env_config.customTextsConfig = myStrings : '';
  }
  // Esto simplemente es un ejemplo de como mostrar los texto de instrucciones por defecto, según el nombre del step
  if (parsedStatus.step === 'instructions-face') {
    delete myStrings['instructions_step_title'];
    delete myStrings['instructions_step_description'];
    delete myStrings['instructions_step_button'];
    dobSdk.env_config ? dobSdk.env_config.customTextsConfig = myStrings : '';
  }
  // Esto simplemente es un ejemplo de como cambiar los textos e iconos en la vista inicial y final en caso de capturar un pasaporte, según el nombre del step
  if (parsedStatus.step === 'permissions-passport' || parsedStatus.step === 'end-passport') {
    // initial styles
    const initial_step = document.getElementById('dob-initial-information-layout');
    if (initial_step) {
      const initial_card_front = initial_step.getElementsByClassName('dob-card-front-icon')[0];
      const initial_card_back = initial_step.getElementsByClassName('dob-card-back-icon')[0];
      initial_card_front.style.background = 'url("assets/images/instructions/passport_icon.svg") no-repeat center';
      initial_card_back.style.background = 'url("assets/images/instructions/passport_icon.svg") no-repeat center';
    }
    // end styles
    const end_step = document.getElementById('dob-end-information-layout');
    if (end_step) {
      const end_card_front = end_step.getElementsByClassName('dob-card-front-icon')[0];
      const end_card_back = end_step.getElementsByClassName('dob-card-back-icon')[0];
      end_card_front.style.background = 'url("assets/images/instructions/passport_icon.svg") no-repeat center';
      end_card_back.style.background = 'url("assets/images/instructions/passport_icon.svg") no-repeat center';
    }
    // Strings
    myStrings['intro_row_obverse'] = myStrings['custom_intro_row_obverse'];
    myStrings['intro_row_reverse'] = myStrings['custom_intro_row_reverse'];
    myStrings['speech_synthesis_capture_doc_front_manual'] = '';
    dobSdk.env_config ? dobSdk.env_config.customTextsConfig = myStrings : '';
  }
  // Esto simplemente es un ejemplo de como cambiar los textos en la vista de instrucciones en caso de capturar un pasaporte, según el nombre del step
  if (parsedStatus.step === 'instructions-passport') {
    myStrings['instructions_step_title'] = myStrings['custom_instructions_step_title'];
    myStrings['instructions_step_description'] = myStrings['custom_instructions_step_description'];
    dobSdk.env_config ? dobSdk.env_config.customTextsConfig = myStrings : '';
  }
  // Esto simplemente es un ejemplo de como cambiar los iconos en el componente secundary toolbar en caso de capturar un pasaporte, según el nombre del step
  if (parsedStatus.step === 'passport-capture' || parsedStatus.step === 'certificate-capture') {
    const secondary_toolbar = document.getElementById('dob-secondary-toolbar');
    if (secondary_toolbar) {
      const secondary_toolbar_card_front = secondary_toolbar.getElementsByClassName('dob-card-front-icon')[0];
      const secondary_toolbar_card_back = secondary_toolbar.getElementsByClassName('dob-card-back-icon')[0];
      if (secondary_toolbar_card_front) {
        // Passport
        secondary_toolbar_card_front.style.background = 'url("assets/images/instructions/passport_icon.svg") no-repeat center';
        secondary_toolbar_card_front.style.backgroundSize = '100% 70%';
      } else {
        // Certificate
        secondary_toolbar_card_back.style.background = 'url("assets/images/instructions/passport_icon.svg") no-repeat center';
        secondary_toolbar_card_front.style.backgroundSize = '100% 70%';
      }
    }
  }
});

// Listen to evidence changes
dobSdk.addEventListener("evidence", evidence => {
  const parsedEvidence = evidence.detail;
  // Esto simplemente es un ejemplo para imprimir la EVIDENCIA en base64 que enviamos en cada paso del SDK.
  switch (parsedEvidence.type) {
    case Evidence.imgDocFront:
    case Evidence.imgDocReverse:
    case Evidence.imgPassport:
    case Evidence.imgEUResidenceCertificate:
    case Evidence.imgSelfie:
      console.log(parsedEvidence.evidence);
      break;
  }
});

// Listen when the SDK has finished
dobSdk.addEventListener("success", success => {
  console.log('SDK-Web onSuccess()');
  window.location.href = 'https://www.inetum.com.es';
});

// Listen to failure changes
dobSdk.addEventListener("failure", error => {
  const parsedFailure = error.detail;
  switch (parsedFailure.code) {
    case ExceptionType.notAllowedPermissionException:
    case ExceptionType.overconstraintException:
    case ExceptionType.invalidFlow:
    case ExceptionType.unsupportedBrowserException:
    case ExceptionType.unupdateBrowserException:
    case ExceptionType.forceExitProcess:
    case ExceptionType.dobApiNonConfiguredOtpContactMethod:
    case ExceptionType.dobApiMaximumNumberOfOtpForwardsHasBeenExceeded:
    case ExceptionType.dobApiMaximumNumberOfOtpReintentHasBeenExceeded:
    case ExceptionType.dobApiContactMethodDoesNotExist:
    case ExceptionType.dobApiMandatoryOtpPhoneNumberNotInformed:
    case ExceptionType.dobApiMandatoryOtpEmailNotInformed:
    case ExceptionType.dobApiNonValidOtpPhoneNumber:
    case ExceptionType.dobApiNonValidOtpEmail:
    case ExceptionType.dobApiOtpHasAlreadyBeenValidated:
    case ExceptionType.dobApiTransactionDoesNotExist:
      console.log('SDK-Web onFailure()');
      // En caso de que el usuario pulse el boton redirigimos
      if (parsedFailure.clickedButton) {
        window.location.replace('https://google.es');
      }
      /*
      //En caso de querer forzar la redireccion
      if (!parsedFailure.clickedButton) {
        setTimeout(() => {  window.location.replace('https://google.es'); }, 1500);
      }
      */
      break;
  }
});
