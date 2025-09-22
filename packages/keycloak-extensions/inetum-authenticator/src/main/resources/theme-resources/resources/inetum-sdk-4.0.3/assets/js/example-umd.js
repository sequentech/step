class DobBroadcastManager {
  onReceive(message) {
    console.log('[TAG] ' + message.tag + ' [LEVEL] ' + message.level + ' [MESSAGE] ' + message.message);
  }
}

document.addEventListener("DOMContentLoaded", function (event) {
  // DobModels - Steps
  var InitialStep = DobModels.InitialStep;
  var InstructionsStep = DobModels.InstructionsStep;
  var DocCaptureStep = DobModels.DocCaptureStep;
  var FaceCaptureStep = DobModels.FaceCaptureStep;
  var VideoIdentificationStep = DobModels.VideoIdentificationStep;
  var AttachStep = DobModels.AttachStep
  var EndStep = DobModels.EndStep;
  // DobModels - Enums
  var InstructionsResourceType = DobModels.InstructionsResourceType;
  var DocSide = DobModels.DocSide;
  var VideoType = DobModels.VideoType;
  var Evidence = DobModels.Evidence;
  var VoiceLanguage = DobModels.VoiceLanguage;
  var ExceptionType = DobModels.ExceptionType;
  var IOSBrowser = DobModels.IOSBrowser;
  var DesignType = DobModels.DesignType;
  // DobModels - Utils
  var SDKUtils = DobModels.SDKUtils;

  // For this example we use the capture design and dni flow
  var design = DesignType.capture;
  var isPassportFlow = false;

  function flow() {
    if (design === DesignType.capture) {
      if (isPassportFlow === true) {
        // Example passport flow
        return [
          new InitialStep('passport-initial'),
          new InstructionsStep('passport-instructions-doc', 'instructions_face_passport.gif', InstructionsResourceType.image, -1),
          new DocCaptureStep('passport-capture', DocSide.front, Evidence.imgPassport, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, -1),
          new DocCaptureStep('certificate-capture', DocSide.front, Evidence.imgEUResidenceCertificate, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, -1),
          new InstructionsStep('passport-instructions-videoselfie', SDKUtils.isMobile() ? 'videoidentification_white' : 'videoidentification_desktop', InstructionsResourceType.video, -1),
          new VideoIdentificationStep('video-obverse-capture', 'user', VideoType.webrtc, DocSide.front, Evidence.imgPassport, 22),
          new VideoIdentificationStep('video-reverse-capture', 'user', VideoType.webrtc, DocSide.front, Evidence.imgEUResidenceCertificate, 22),
          new FaceCaptureStep('face-capture', 'user', VideoType.photo, 30),
          new EndStep('passport-end')
        ];
      } else {
        // Example dni flow
        return [
          new InitialStep('dni-initial'),
          new InstructionsStep('dni-instructions-doc', SDKUtils.isMobile() ? 'showfrontdesktop' : 'showfrontdesktop', InstructionsResourceType.video, -1),
          new DocCaptureStep('obverse-capture', DocSide.front, Evidence.imgDocFront, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, 10),
          new DocCaptureStep('reverse-capture', DocSide.back, Evidence.imgDocReverse, SDKUtils.isMobile() ? 'environment' : 'user', VideoType.photo, true, 10),
          new InstructionsStep('dni-instructions-videoselfie', SDKUtils.isMobile() ? 'videoidentification_white' : 'videoidentification_desktop', InstructionsResourceType.video, -1),
          new VideoIdentificationStep('video-obverse-capture', 'user', VideoType.webrtc, DocSide.front, Evidence.imgDocFront, 22),
          new VideoIdentificationStep('video-reverse-capture', 'user', VideoType.webrtc, DocSide.back, Evidence.imgDocReverse, 22),
          new FaceCaptureStep('face-capture', 'user', VideoType.photo, 30),
          new EndStep('dni-end')
        ];
      }
    } else if (design === DesignType.attach) {
      // Example attach flow
      var isEU = false;
      return [
        isEU ? new AttachStep('passport-attachment', Evidence.imgPassport, true, true) : new AttachStep('obverse-attachment', Evidence.imgDocFront, true, true),
        isEU ? new AttachStep('certificate-attachment', Evidence.imgEUResidenceCertificate, true, true) : new AttachStep('reverse-attachment', Evidence.imgDocReverse, true, true),
        //new AttachStep('video-attachment', Evidence.videoSelfie, false, false),
        new AttachStep('face-attachment', Evidence.imgSelfie, false, true),
        new EndStep('attachment-end')
      ];
    }
  }

  // Custom strings
  var customStrings = {
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
    'attach_title_doc': 'Adjuntar documento',
    'attach_title_video': 'Adjuntar vídeo',
    'attach_title_face': 'Adjuntar selfie',
    'attach_description_doc_front': 'Sube la parte delantera',
    'attach_description_doc_back': 'Sube la parte trasera',
    'attach_description_doc_passport': 'Sube la parte delantera',
    'attach_description_doc_certificate': 'Sube la parte delantera',
    'attach_description_video': 'Sube un vídeo',
    'attach_description_face': 'Sube un seflie',
    'attach_button_text': 'Buscar el archivo',
    'attach_file_successfully_submited': '¡Tu archivo ha sido subido con éxito!',
    'attach_on_uploading_file': 'Subiendo el archivo, un momento por favor...',
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
    'otp_button': 'Continuar',
    // END STEP
    'end_title': 'Identificación digital',
    'end_description': 'El proceso de identificación es muy sencillo. Sólo necesitas tener a mano tu documento de identidad y capturar con la cámara del móvil lo siguiente:',
    'end_subtitle': '¡Proceso finalizado!',
    'end_button_text': 'Finalizar',
    // INITIAL STEP & END STEP
    'intro_row_obverse': 'Anverso del documento',
    'intro_row_reverse': 'Reverso del documento',
    'intro_row_face': 'Rostro e Identidad',
    'intro_row_passport': 'Pasaporte',
    'intro_row_residence_certificate': 'Certificado de residencia',
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
    'media_device_progress_description': 'Obteniendo multimedia...',
    'background_progress_description': 'Toca la pantalla para continuar...',
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
    // TOOLTIP COMPONENT
    'dob_tooltip_show_help': 'Ver ayuda',
    'dob_tooltip_leave_process': 'Salir',
    'dob_tootltip_take_photo': '¡Haz una foto!',
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
    // CUSTOM STRINGS INSTRUCTIONS STEP - ANVERSO / REVERSO - FLUJO DNI
    'custom_instructions_obverse_reverse_dni_title': 'Paso 1 de 2',
    'custom_instructions_obverse_reverse_dni_description': 'A continuación vamos a capturar el anverso y el reverso. Si es posible, busca un fondo oscuro y apoya el documento en una superficie plana.',
    'custom_instructions_obverse_reverse_dni_button': 'Continuar',
    // CUSTOM STRINGS INSTRUCTIONS STEP - VIDEO
    'custom_instructions_video_title': 'Paso 2 de 2',
    'custom_instructions_video_description': 'Ahora vamos a grabarte en vídeo. Tendrás que mostrar tu documento. Después te tomaremos un selfie y tendrás que hacer coincidir los dos óvalos que verás en pantalla. Busca un sitio bien iluminado. Solo podrá aparecer la persona que se está identificando.',
    'custom_instructions_video_button': 'Grabar',
    // CUSTOM STRINGS INSTRUCTIONS  STEP - ANVERSO / REVERSO - FLUJO PASAPORTE
    'custom_instructions_passport_title': 'Paso 1 de 2',
    'custom_instructions_passport_description': 'A continuación vamos a capturar el pasaporte y el certificado de residencia. Si es posible, busca un fondo oscuro y apoya el documento en una superficie plana.',
    'custom_instructions_passport_button': 'Continuar',
    // CUSTOM STRINGS INITIAL STEP - ANVERSO - FLUJO PASAPORTE
    'custom_initial_passport_intro_row_obverse': 'Pasaporte',
    'custom_initial_certificate_intro_row_reverse': 'Certificado de residencia'
  };

  // Config SDK
  var sdk = {
    ak: 'bff31040-4da9-34ab-accc-ceac47c58885',
    applicationId: '6099c471-03c0-4d76-90ba-ef14c5247b22',
    flow: flow()
  };

  var env_config = {
    environment: 0,
    customTextsConfig: customStrings,
    baseAssetsUrl: '../../../',
    uploadAndCheckIdentifiers: ['ESP'],
    showLogs: false,
    logTypes: ['ERROR', 'INFO'],
    design: design,
    bamEnabled: true,
    ocrCountdown: false,
    videoSelfieShowDNI: true,
    cancelProcessButton: true,
    showPermissionsHelp: true,
    qrEnabled: false,
    voiceEnabled: true,
    voiceLanguage: VoiceLanguage.spanishSpain,
    customIOSBrowsersConfig: [IOSBrowser.safari],
    otpEmailAddress: '',
    otpPhoneNumber: '',
    broadcast: new DobBroadcastManager()
  };

  var session = {
    tokenDOB: window.DOB_DATA.td,
    userID: window.DOB_DATA.uid
  };

  // Load config
  var dobSdk = document.getElementById("dob-sdk");
  dobSdk.session = session;
  dobSdk.sdk = sdk;
  dobSdk.env_config = env_config;

  // Listen to status event
  dobSdk.addEventListener("status", status => {
    var parsedStatus = status.detail;
    // FLUJO DNI: Este es un ejemplo de como cambiar las instrucciones en el flujo dni, según el nombre del step
    if (parsedStatus.step === 'dni-instructions-doc') {
      customStrings['instructions_step_title'] = customStrings['custom_instructions_obverse_reverse_dni_title'];
      customStrings['instructions_step_description'] =
        customStrings['custom_instructions_obverse_reverse_dni_description'];
      customStrings['instructions_step_button'] = customStrings['custom_instructions_obverse_reverse_dni_button'];
      dobSdk.env_config ? dobSdk.env_config.customTextsConfig = customStrings : '';
    }
    if (parsedStatus.step === 'dni-instructions-videoselfie') {
      customStrings['instructions_step_title'] = customStrings['custom_instructions_video_title'];
      customStrings['instructions_step_description'] = customStrings['custom_instructions_video_description'];
      customStrings['instructions_step_button'] = customStrings['custom_instructions_video_button'];
      dobSdk.env_config ? dobSdk.env_config.customTextsConfig = customStrings : '';
    }
    if (parsedStatus.step === 'dni-instructions-videoselfie') {
      // Descomentar en caso de querar dejar las instrucciones por defecto (Hacen referencia al videoselfie)
      // delete customStrings['instructions_step_title'];
      // delete customStrings['instructions_step_description'];
      // customStrings['instructions_step_button'] = customStrings['instructions_step_button'];
      // dobSdk.env_config ? dobSdk.env_config.customTextsConfig = customStrings : '';
    }

    // FLUJO PASAPORTE: Este es un ejemplo de como cambiar las instrucciones en el flujo pasaporte, según el nombre del step
    if (parsedStatus.step === 'passport-initial' || parsedStatus.step === 'passport-end') {
      customStrings['intro_row_obverse'] = customStrings['custom_initial_passport_intro_row_obverse'];
      customStrings['intro_row_reverse'] = customStrings['custom_initial_certificate_intro_row_reverse'];
      customStrings['speech_synthesis_capture_doc_front_manual'] = '';
      dobSdk.env_config ? dobSdk.env_config.customTextsConfig = customStrings : '';
    }
    if (parsedStatus.step === 'passport-instructions-doc') {
      customStrings['instructions_step_title'] = customStrings['custom_instructions_passport_title'];
      customStrings['instructions_step_description'] = customStrings['custom_instructions_passport_description'];
      customStrings['instructions_step_button'] = customStrings['custom_instructions_passport_button'];
      dobSdk.env_config ? dobSdk.env_config.customTextsConfig = customStrings : '';
    }
    if (parsedStatus.step === 'passport-instructions-videoselfie') {
      customStrings['instructions_step_title'] = customStrings['custom_instructions_video_title'];
      customStrings['instructions_step_description'] = customStrings['custom_instructions_video_description'];
      customStrings['instructions_step_button'] = customStrings['custom_instructions_video_button'];
      dobSdk.env_config ? dobSdk.env_config.customTextsConfig = customStrings : '';
    }
  });

  // Listen to evidence event: base64 format images
  dobSdk.addEventListener("evidence", evidence => {
    var parsedEvidence = evidence.detail;
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

  // Listen to success event
  dobSdk.addEventListener("success", success => {
    console.log('SDK-Web onSuccess()');
    window.location.href = 'https://www.inetum.com.es';
  });

  // Listen to failure event
  dobSdk.addEventListener("failure", error => {
    var parsedFailure = error.detail;
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
          setTimeout(() => { window.location.replace('https://google.es'); }, 1500);
        }
        */
        break;
    }
  });
});
