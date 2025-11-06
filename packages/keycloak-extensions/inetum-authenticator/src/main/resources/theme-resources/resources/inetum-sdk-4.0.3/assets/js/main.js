// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// DobModels - Steps
import {
    InitialStep,
    InstructionsStep,
    DocCaptureStep,
    FaceCaptureStep,
    VideoIdentificationStep,
    AttachStep,
    EndStep,
} from "../../../inetum-sdk-4.0.3/assets/js/dob-models-1.1.20.esm.js";
// DobModels - Enums
import {
    EventType,
    InstructionsResourceType,
    DocSide,
    VideoType,
    Evidence,
    VoiceLanguage,
    ExceptionType,
    IOSBrowser,
    CountryCode,
    DesignType,
} from "../../../inetum-sdk-4.0.3/assets/js/dob-models-1.1.20.esm.js";
// DobModels - Utils
import { SDKUtils } from "../../../inetum-sdk-4.0.3/assets/js/dob-models-1.1.20.esm.js";

class LocalBroadcastManager {
    onReceive(message) {
        console.log(
            "[TAG] " +
                message.tag +
                " [LEVEL] " +
                message.level +
                " [MESSAGE] " +
                message.message
        );
    }
}

let design = DesignType.capture;
/*
  // Ejemplo con responsive
  let design = DesignType.attach;
*/

let showBackStep = true;

let isPassportFlow =
    window.DOB_DOC_ID_TYPE === "philippinePassport" ||
    window.DOB_DOC_ID_TYPE === "seamanBook";
let intro_row_passport_en =
    window.DOB_DOC_ID_TYPE === "seamanBook" ? `Seaman's Book` : "Passport";
let intro_row_passport_tl =
    window.DOB_DOC_ID_TYPE === "seamanBook" ? `Seaman's Book` : "Pasaporte";

/*
  // Ejemplo con pasaporte (revisar tambien estilos de ejemplo en dob-style.css y descomentarlos)
 let isPassportFlow = true;
*/
let info_title;
let info_description;

let sdk; // Configuration parameters for DoBSDK (obligatorio)
let session; // Session parametes (obligatorio)
let env_config; // Configuration parametres for Environments

function flow() {
    let disableStreaming = env_config.sequent?.disableStreaming ?? false;
    let videoStepLength = env_config.sequent?.videoStepLength ?? 22;
    let photoStepLength = env_config.sequent?.photoStepLength ?? 10;
    let userPhotoLength = env_config.sequent?.userPhotoLength ?? 30;
    if (isPassportFlow === true) {
        // Passportflow
        return [
            ...[
                new InitialStep("permissions-passport"),
                new DocCaptureStep(
                    "passport-capture",
                    DocSide.front,
                    Evidence.imgPassport,
                    SDKUtils.isMobile() ? "environment" : "user",
                    VideoType.photo,
                    true,
                    -1
                ),
            ],
            ...(disableStreaming
                ? []
                : [
                      new InstructionsStep(
                          "instructions-face",
                          "videoidentification_desktop",
                          InstructionsResourceType.video,
                          -1
                      ),
                      new VideoIdentificationStep(
                          "show_front",
                          "user",
                          VideoType.webrtc,
                          DocSide.front,
                          Evidence.imgPassport,
                          videoStepLength
                      ),
                  ]),
            ...[
                new FaceCaptureStep(
                    "face-capture",
                    "user",
                    VideoType.photo,
                    userPhotoLength
                ),
            ],
        ];
    } else {
        return [
            ...[new InitialStep("permissions")],
            ...[
                new DocCaptureStep(
                    "front-capture",
                    DocSide.front,
                    Evidence.imgDocFront,
                    SDKUtils.isMobile() ? "environment" : "user",
                    VideoType.photo,
                    true,
                    photoStepLength
                ),
            ],
            ...(showBackStep
                ? [
                      new DocCaptureStep(
                          "back-capture",
                          DocSide.back,
                          Evidence.imgDocReverse,
                          SDKUtils.isMobile() ? "environment" : "user",
                          VideoType.photo,
                          true,
                          photoStepLength
                      ),
                  ]
                : []),
            ...(disableStreaming
                ? []
                : [
                      new InstructionsStep(
                          "instructions-face",
                          "videoidentification_desktop",
                          InstructionsResourceType.video,
                          -1
                      ),
                      new VideoIdentificationStep(
                          "show_front",
                          "user",
                          VideoType.webrtc,
                          DocSide.front,
                          Evidence.imgDocFront,
                          videoStepLength
                      ),
                  ]),
            ...(showBackStep && !disableStreaming
                ? [
                      new VideoIdentificationStep(
                          "show_back",
                          "user",
                          VideoType.webrtc,
                          DocSide.back,
                          Evidence.imgDocReverse,
                          videoStepLength
                      ),
                  ]
                : []),
            ...[
                new FaceCaptureStep(
                    "face-capture",
                    "user",
                    VideoType.photo,
                    userPhotoLength
                ),
            ],
        ];
    }
}

function setStringHtmlValues(title, description) {
    info_title = document.getElementById("info_title");
    info_description = document.getElementById("info_description");
    info_title.innerHTML = title;
    info_description.innerHTML = description;
}

function removeMessagesAttach() {
    const messages = document.getElementsByClassName("dob-attach-messages")[0];
    if (messages) {
        messages.parentElement.removeChild(messages);
    }
}

// Actualizamos DOM | para el ejemplo en caso de responsive
if (design === DesignType.capture) {
    removeMessagesAttach();
}

let myStringsLocaleMap = {
    en: {
        // DOB API
        dob_api_unknown_error: "Unknown error",
        dob_api_incorrect_parameters: "Incorrect input parameters",
        dob_api_incorrect_token: "Incorrect provided token",
        dob_api_unknown_uid: "Unknown identifier",
        dob_api_not_available_evidence: "Evidence not available",
        dob_api_incorrect_upload: "Document not received correctly",
        dob_api_empty_request: "Empty query result",
        dob_api_unknown_action: "Unknown action",
        dob_api_so_big_evidence: "Evidence size too big",
        dob_api_evidences_already_uploaded: "Evidences already uploaded",
        dob_api_not_identify_document: "Document type is not identified",
        dob_api_invalid_evidence: "Evidence type not allowed",
        dob_api_back_doc_not_match:
            "The face of the document does not match the evidence sent",
        dob_api_expired_document: "Expired document",
        dob_api_bad_quality_doc: "Low image quality",
        dob_api_config_not_enable: "Configuration not enabled",
        dob_api_transaction_not_finish:
            "The transaction has not been processed yet",
        dob_api_not_available_zip: "ZIP not available",
        dob_api_incorrect_mrz:
            "Wow! It seems that the capture is not of sufficient quality. Please repeat the capture.",
        dob_api_transaction_exists: "Registration process already exists.",
        dob_api_younger: "The attached document corresponds to a minor user",
        dob_api_incorrect_evidence:
            "The evidence does not correspond to the requested evidence.",
        dob_api_incorrect_document: "Unsupported document model",
        // INITIAL STEP
        initial_title_mobile: "Digital ID",
        initial_description_mobile:
            "The identification process is very simple. You just need to have your identification document on hand. We will guide you to capture the front and back, and then you will have to take a video selfie with the document in your hand. Find a well-lit place and... smile!",
        initial_button_mobile: "Start",
        // INSTRUCTIONS STEP
        instructions_step_title: "Video Instructions",
        passport_custom_instructions_step_title: "Video Instructions",
        instructions_step_description:
            "Now we are going to video you. You will have to show your document. Then we will take a selfie and you will have to match the two ovals that you will see on the screen. Find a well-lit place. Only the person who is identifying themselves may appear in the video.",
        passport_custom_instructions_step_description:
            "Now we are going to video you. You will have to show your document. Then we will take a selfie and you will have to match the two ovals that you will see on the screen. Find a well-lit place. Only the person who is identifying themselves may appear in the video.",
        instructions_step_button: "Continue",
        passport_custom_instructions_step_button: "Continue",
        // DOC-CAPTURE STEP
        card_detector_evidence_not_focus: "The image is out of focus",
        card_detector_no_detect_docs: "We are not detecting anything...",
        card_detector_in_progress: "Scanning...",
        card_detector_adjust_doc: "Adjust the document a little more...",
        card_detector_more_near_doc: "Near the document...",
        card_detector_automatic_help_text:
            "Set the document to do automatic capture",
        card_detector_manual_help_text: "Set the document to do manual capture",
        card_detector_passport_manual_help_text: "Set the passport to capture",
        card_detector_certificate_manual_help_text:
            "Set the residence certificate to capture",
        // VIDEO IDENTIF. STEP
        video_identification_message_front: "Shows the front of the document",
        video_identification_message_back: "Turn the document over",
        video_identification_message_passport: "Show passport",
        video_identification_message_certificate: "Show the certificate",
        // FACE-CAPTURE STEP
        permissions_orientation_dialog_title:
            "To improve your experience, we need you to give us access to your phone's orientation",
        permissions_orientation_dialog_button: "OK",
        face_capture_device_motion_down: "Lower device",
        face_capture_device_motion_up: "Raise the device",
        face_capture_detection_closer: "Get closer",
        face_capture_detection_further: "Stay away",
        face_capture_detection_multi_face: "Only one face please",
        // DOC-CAPTURE STEP & IDEO IDENTIF. STEP & FACE-CAPTURE STEP
        media_recorder_unknown_exception:
            "The device or browser does not support video recording",
        media_recorder_experimental_feature_exception:
            "Video recording is not enabled on your device. Please go to Settings>Safari>Advanced>Experimental Features and activate the MediaRecorder flag",
        // ATTACH STEP
        attach_button_text: "Browse the file",
        attach_file_successfully_submitted:
            "Your file has been successfully uploaded!",
        attach_on_uploading_file: "Uploading the file, one moment please",
        attach_multi_files_upload_error: "Only one file upload is allowed",
        attach_invalid_file_type_error: "There was an error reading this file",
        attach_processing_file: "Processing file...",
        attach_default_error:
            "The attachment does not appear to meet the required criteria",
        attach_error_button_text: "RETRY",
        attach_not_supported_file: "Not supported file type",
        attach_title_doc: "Attach document",
        attach_title_video: "Attach video",
        attach_title_face: "Attach selfie",
        attach_description_doc_front: "Upload the front",
        attach_description_doc_back: "Upload the back",
        attach_description_doc_passport: "Upload the front",
        attach_description_doc_certificate: "Upload the back",
        attach_description_video: "Upload a video",
        attach_description_face: "Upload a selfie",
        attach_button_text: "Search a file",
        // OTP STEP
        otp_verification_title: "OTP Verification",
        otp_verification_email:
            "We have sent you the access code by email for verification.",
        otp_verification_sms:
            "We have sent you the access code via SMS for verification.",
        otp_verification_resend_question: "Have you not received the OTP code?",
        otp_verification_resended_email:
            "A new OTP code has been sent to your email address.",
        otp_verification_resended_sms:
            "A new OTP code has been sent to your mobile number.",
        otp_verification_invalid: "OTP code is invalid",
        otp_verification_expired: "OTP code has expired",
        otp_button: "Continue",
        // END STEP
        end_title: "Digital ID",
        end_description:
            "The identification process is very simple. You just need to have your identity document on hand and capture the following with your mobile camera:",
        end_subtitle: "Process finished!",
        end_button_text: "End",
        // INITIAL STEP & END STEP
        intro_row_obverse: "Front of document",
        intro_row_reverse: "Back of document",
        intro_row_face: "Face and Identity",
        intro_row_passport: `${intro_row_passport_en}`,
        intro_row_residence_certificate: "Residence certificate",
        // EXCEPTION VIEW
        exception_button_text_retry: "Retry",
        exception_button_text_go_init: "Return",
        new_flow_exception_tips: [
            "Could not start a new identification process from the device.",
            "Please press retry or try again later",
        ],
        not_readable_exception_tips: [
            "Your audio/video device is being used by another application.",
            "Please close the application that is using your device and restart the process.",
        ],
        recording_exception_tips: [
            "The device or browser does not support video recording",
            "The browsers that support video recording are: ",
            [
                "Chrome Desktop (>v49)",
                "Firefox Desktop (>v29)",
                "Edge Desktop(>v76)",
                "Safari Desktop (>v13)",
                "Opera Desktop (>v62)",
                "Chrome for Android",
            ],
        ],
        connection_generic_error_tips: [
            "Oh! It seems there was an error trying to connect to the server",
            "Please check your internet connection or try again in five minutes.",
        ],
        unknown_media_exception_tips: [
            "There was an unknown error while trying to access your video/audio device",
            "Please restart the video identification process",
        ],
        unknown_attach_exception_tips: [
            "There was an unknown error when attaching the evidence",
            "Please try again later",
        ],
        webrtc_exception_tips: [
            "The connection to the streaming server has been interrupted.",
            [
                "It is possible that a network failure occurred that caused the loss.",
                "Please try again in a few moments.",
            ],
        ],
        global_timeout_exception_tips: [
            "Total flow time has been exceeded",
            "Please press retry to restart the flow",
        ],
        no_devices_found_tips: [
            "No audio/video device found.",
            "Please connect a device and restart the process.",
        ],
        upload_exception_tips: [
            "A connection error occurred while sending identification evidence.",
            [
                "It is possible that a network failure occurred that caused the loss.",
                "Please try again in a few moments.",
            ],
        ],
        upload_check_exception_tips: [
            "An error occurred while verifying the document. This could be due to:",
            [
                "The image is not of sufficient quality. Remember that it must be well focused.",
                "The document is not identified as a valid type.",
                "The document is expired",
                "The face of the document does not correspond to the evidence sent",
            ],
            "Please try again by doing the following:",
            [
                "Place the ID on a flat, bright surface",
                "Make sure you have enough light",
                "Make sure there are no highlights or dark areas in the document",
                "Match the silhouette drawn on the screen with the image on your camera.",
            ],
        ],
        face_detector_timeout_exception_tips: [
            "Oh! Something is wrong, we couldn't capture his face.",
            "Please try again by doing the following:",
            [
                "Place yourself in a bright place",
                "Try to place your face within the guide that appears on the screen",
                "Look straight into the camera.",
            ],
        ],
        manual_face_detector_exception_tips: [
            "No face detected in selfie photo:",
            ["Please focus on your face when taking the photo."],
        ],
        manual_multi_face_detector_exception_tips: [
            "Several faces have been detected in the selfie photo:",
            ["Please, only one face can appear."],
        ],
        card_detector_timeout_exception_tips: [
            "Oh! Something is wrong, we couldn't capture your ID.",
            "Please try again by doing the following:",
            [
                "Place the ID document on a flat, bright surface.",
                "Match the silhouette drawn on the screen with the image on your camera.",
            ],
        ],
        not_allowed_permission_exception_tips: [
            "We have not been able to access the device because you have not allowed access to the camera and/or microphone.",
            "Please enable camera and audio permissions and restart the process.",
        ],
        overconstraint_exception_tips: [
            "Your device does not meet the minimum requirements to carry out the process.",
            "Please change devices and repeat the process",
        ],
        invalid_flow_exception: [
            "The configured flow is invalid.",
            "Please contact your system administrator.",
        ],
        unsupported_browser_exception_tips: [
            "Browser not compatible with VideoStreaming.",
            "Please use another browser such as Chrome or Firefox.",
        ],
        unsupported_browser_exception_ios_tips: [
            "Browser does not support VideoStreaming.",
            "Please use Safari.",
        ],
        unupdate_browser_exception_tips: [
            "Your browser version is not compatible.",
            "Please update your browser and try again",
        ],
        dob_api_face_too_close_tips: [
            "The face is too close in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_eyes_closed_tips: [
            "The selfie came out with my eyes closed.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_face_close_to_border_tips: [
            "The face is too close to the edge.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_face_cropped_tips: [
            "The face is cut off in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_face_is_occluded_tips: [
            "The face is covered in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_face_not_found_tips: [
            "We did not detect a face in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_too_many_faces_tips: [
            "There are too many faces in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_face_too_small_tips: [
            "The face is too small in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_face_angle_too_large_tips: [
            "The camera has been tilted in the selfie.",
            "Center your face in the oval when taking the photo and make sure it looks good.",
        ],
        dob_api_non_configured_otp_contact_method: [
            "OTP contact method not configured.",
            "Please contact your system administrator.",
        ],
        dob_api_maximum_number_of_otp_forwards_has_been_exceeded: [
            "The maximum number of OTP forwards has been exceeded.",
        ],
        dob_api_maximum_number_of_otp_reintent_has_been_exceeded: [
            "The maximum number of OTP retries has been exceeded.",
        ],
        dob_api_contact_method_does_not_exist: [
            "Contact method does not exist.",
            "Please contact your system administrator.",
        ],
        dob_api_mandatory_otp_phone_number_not_informed: [
            "OTP phone number required, not reported.",
            "Please contact your system administrator or try again.",
        ],
        dob_api_mandatory_otp_email_not_informed: [
            "OTP email required, not reported.",
            "Please contact your system administrator or try again.",
        ],
        dob_api_non_valid_otp_phone_number: [
            "Invalid OTP phone number.",
            "Please contact your system administrator or try again.",
        ],
        dob_api_non_valid_otp_email: [
            "Invalid OTP email.",
            "Please contact your system administrator or try again.",
        ],
        dob_api_otp_has_already_been_validated: [
            "The OTP has already been validated.",
            "Please contact your system administrator.",
        ],
        dob_api_transaction_does_not_exist: [
            "The transaction does not exist.",
            "Please contact your system administrator or try again.",
        ],
        // HELP DIALOG VIEW
        secondarytoolbar_help_title: "INFORMATION",
        default_instructions_docs: [
            "Place the identity document on a flat, bright surface",
            "The document should not have shine or reflections that could make reading difficult",
            "Match the silhouette drawn on the screen with the image on your camera",
            "The capture will be done automatically when the silhouette and image match",
        ],
        default_instructions_face: [
            "Place yourself in a bright place",
            "Try to place your face within the guide that appears on the screen",
            "Look straight into the camera",
            "Remember to show your ID on both sides",
            "Remember that no more people should appear in the video",
        ],
        secondarytoolbar_help_button: "CLOSE",
        // MANUAL CAPTURE VIEW
        manual_capture_doc_title_text: "Manual capture mode",
        manual_capture_doc_lead_text: "Frame the document and press the button",
        manual_capture_face_title_text: "Manual selfie capture mode",
        manual_capture_face_lead_text:
            "Focus on the face to take a selfie and click the button to capture the photo",
        // PREVIEW VIEW
        attach_preview_retry_button: "Repeat",
        attach_preview_continue_button: "Continue",
        // PREVIEW VIEW | ONLY DESIGN GENERIC
        preview_capture_doc_text:
            "Check that the photo is readable, not blurry, in focus and glare-free",
        // PREVIEW VIEW | ONLY RESPONSIVE DESIGN
        attach_preview_text:
            "Remember that the image must be oriented correctly and that it must display correctly",
        // LOADER VIEW
        default_progress_description: "Connecting...",
        video_progress_description: "Connecting...",
        end_progress_description: "Ending...",
        new_device_progress_description: "Starting new flow from device...",
        otp_configuration_progress_description: "Loading configuration...",
        otp_forwarding_progress_description: "Sending OTP...",
        otp_verification_progress_description: "Verifying OTP...",
        media_device_progress_description: "Obtaining media device...",
        background_progress_description: "Touch the screen to continue...",
        // TOOLBAR COMPONENT
        secondarytoolbar_identification_error: "Identification error",
        secondarytoolbar_obverse: "Document Front",
        secondarytoolbar_reverse: "Document Back",
        secondarytoolbar_face: "Face and Identity",
        secondarytoolbar_passport: `${intro_row_passport_en}`,
        secondarytoolbar_certificate: "Residency certificate",
        secondarytoolbar_exit_button: "Exit",
        dob_tooltip_show_help: "Show Help",
        dob_tooltip_leave_process: "Leave",
        dob_tootltip_take_photo: "Capture a photo!",
        // INFOBAR COMPONENT
        card_detector_verifying: "Verifying document...",
        infobar_start_text: "Configuring scanner",
        infobar_working_card_capture_text:
            "Place the ID card on a flat, bright surface",
        infobar_uploading_text: "Sending files...",
        infobar_finish_text: "Perfect!",
        infobar_passport_start_text:
            "Place the passport on a flat, bright surface",
        infobar_passport_working_card_capture_text:
            "Place the passport on a flat, bright surface",
        infobar_passport_uploading_text: "We are sending the photo",
        infobar_passport_finish_text: "Perfect!",
        infobar_passport_end_text: "We already have the passport",
        infobar_certificate_start_text:
            "Place the certificate on a flat, bright surface",
        infobar_certificate_working_card_capture_text:
            "Place the certificate on a flat, bright surface",
        infobar_certificate_uploading_text: "We are sending the photograph",
        infobar_certificate_finish_text: "Perfect!",
        infobar_certificate_end_text: "We already have the certificate",
        infobar_working_face_capture_text:
            "Try to place your face within the guide that appears on the screen",
        infobar_video_identification_front_text:
            "Hold the ID by the edge, showing the front and position yourself for the video",
        infobar_video_identification_back_text:
            "Now show the back of the identity document. Don't forget to grab it by the edge",
        infobar_face_detector_verifying: "Verifying selfie...",
        // SPEECH SYNTHESIS
        speech_synthesis_capture_doc_front:
            "Match the front of your document to the silhouette",
        speech_synthesis_capture_doc_front_manual:
            "Take a photo of the front of your document",
        speech_synthesis_capture_doc_front_qr:
            "Use your mobile to scan the qr code and take a photo of the front of the document",
        speech_synthesis_capture_doc_back:
            "Match the back of your document to the silhouette",
        speech_synthesis_capture_doc_back_manual:
            "Take a photo of the back of your document",
        speech_synthesis_capture_doc_back_qr:
            "Now with your mobile, take a photo of the back of the document",
        speech_synthesis_capture_doc_passport: "Take a passport photo",
        speech_synthesis_capture_doc_passport_qr:
            "Use your mobile to scan the qr code and take a photo of the passport",
        speech_synthesis_capture_doc_certificate:
            "Take a photo of the residence certificate",
        speech_synthesis_capture_doc_certificate_qr:
            "Now with your mobile, take a photo of the certificate",
        speech_synthesis_capture_doc_finish_qr:
            "Continue processing for this device",
        speech_synthesis_video_identification_front:
            "Show the front of the document",
        speech_synthesis_video_identification_back:
            "Show the back of the document",
        speech_synthesis_video_identification_passport: "Show passport",
        speech_synthesis_video_identification_certificate:
            "Show the certificate",
        speech_synthesis_face_capture: "Get ready to take a selfie",
        speech_synthesis_face_capture_manual:
            "Take a selfie by clicking the capture button",
        speech_synthesis_attach_doc_front:
            "Attach a photo of the front of the document",
        speech_synthesis_attach_doc_back:
            "Attach a photo of the back of the document",
        speech_synthesis_attach_video_identification:
            "Attach a video identifier with your face",
        speech_synthesis_attach_face: "Attach a photo of your face",
        speech_synthesis_attach_passport: "Attach a passport photo",
        speech_synthesis_attach_ue:
            "Attach a photo of the European Union residence certificate",
        // HELP ORIENTATION
        helporientation_title: "Please change the orientation to portrait",
        helporientation_button: "CLOSE",
        // HELP PERMISSIONS VIEW
        help_permissions_title: "Camera and microphone permissions",
        help_permissions_description:
            "We need you to accept the permissions to access the camera and microphone. They are essential to carry out video identification.",
        // EXIT DIALOG VIEW
        exit_dialog_title: "Notice",
        exit_dialog_subtitle: "Do you want to cancel the process?",
        exit_dialog_accept: "Yes",
        exit_dialog_cancel: "No",
        // QR VIEW
        qr_capture_doc_front:
            "Scan the QR code with your mobile to take a photo of the front of the document.",
        qr_capture_doc_back:
            "Now take a photo of the back of the document with your mobile.",
        qr_connect_error:
            "Connection problems, please scan the QR code again in a few moments.",
        // CUSTOM STRINGS INSTRUCTIONS STEP
        custom_instructions_doc_title: "Digital Identification",
        custom_instructions_doc_description:
            "Next we are going to capture the front and back. If possible, find a dark background and place the document on a flat surface.",
        custom_instructions_doc_button: "Continue",
        // CUSTOM STRINGS INITIAL/END STEP - EXAMPLE: PASSPORT
        custom_intro_row_obverse: `${intro_row_passport_en}`,
        custom_intro_row_reverse: "Face with Passport",
        // CUSTOM STRINGS INSTRUCTIONS STEP - EXAMPLE: PASSPORT
        custom_instructions_step_title: "Passport Instructions",
        custom_instructions_step_description:
            "First we will start with capturing the passport.",
    },
    tl: {
        // DOB API
        dob_api_unknown_error: "Hindi kilalang error",
        dob_api_incorrect_parameters: "Maling input parameters",
        dob_api_incorrect_token: "Maling token na ibinigay",
        dob_api_unknown_uid: "Hindi kilalang identifier",
        dob_api_not_available_evidence: "Hindi available ang ebidensya",
        dob_api_incorrect_upload: "Hindi maayos na natanggap ang dokumento",
        dob_api_empty_request: "Walang resulta mula sa query",
        dob_api_unknown_action: "Hindi kilalang aksyon",
        dob_api_so_big_evidence: "Sobrang laki ng ebidensya",
        dob_api_evidences_already_uploaded: "Na-upload na ang ebidensya",
        dob_api_not_identify_document: "Hindi matukoy ang uri ng dokumento",
        dob_api_invalid_evidence: "Hindi pinapayagan ang uri ng ebidensya",
        dob_api_back_doc_not_match:
            "Ang mukha ng dokumento ay hindi tugma sa ipinadalang ebidensya",
        dob_api_expired_document: "Paso na ang dokumento",
        dob_api_bad_quality_doc: "Mababa ang kalidad ng imahe",
        dob_api_config_not_enable: "Hindi naka-enable ang configuration",
        dob_api_transaction_not_finish: "Hindi pa natapos ang transaksyon",
        dob_api_not_available_zip: "Hindi available ang ZIP",
        dob_api_incorrect_mrz:
            "Mukhang hindi sapat ang kalidad ng capture. Paki-ulit ang capture.",
        dob_api_transaction_exists:
            "Mayroon nang umiiral na proseso ng rehistrasyon.",
        dob_api_younger:
            "Ang kalakip na dokumento ay para sa isang menor de edad",
        dob_api_incorrect_evidence:
            "Ang ebidensya ay hindi tumutugma sa hinihiling na ebidensya.",
        dob_api_incorrect_document: "Hindi suportado ang modelo ng dokumento",
        // INITIAL STEP
        initial_title_mobile: "Digital ID",
        initial_description_mobile:
            "Madali lang magpa-verify. Ihanda lang ang ID at sundan ang mga instructions. Humanap ng maliwanag na lugar at... ngumiti!",
        initial_button_mobile: "Simulan",
        // INSTRUCTIONS STEP
        instructions_step_title: "Mga Tagubilin sa Video",
        passport_custom_instructions_step_title: "Mga Tagubilin sa Video",
        instructions_step_description:
            "Ngayon, kukunan ka ng video habang ipinapakita mo ang iyong dokumento. Siguraduhing maliwanag ang larawan. Tanging ang may-ari ng dokumento lamang ang dapat nasa video",
        passport_custom_instructions_step_description:
            "Ngayon, kukunan ka ng video habang ipinapakita mo ang iyong dokumento. Siguraduhing maliwanag ang larawan. Tanging ang may-ari ng dokumento lamang ang dapat nasa video",
        instructions_step_button: "Magpatuloy",
        passport_custom_instructions_step_button: "Magpatuloy",
        // DOC-CAPTURE STEP
        card_detector_evidence_not_focus: "Malabo ang imahe",
        card_detector_no_detect_docs: "Wala kaming nadedetect...",
        card_detector_in_progress: "Ina-scan...",
        card_detector_adjust_doc: "Ayusin pa ang dokumento...",
        card_detector_more_near_doc: "Lapitan pa ang dokumento...",
        card_detector_automatic_help_text:
            "Itakda ang dokumento para sa awtomatikong capture",
        card_detector_manual_help_text: "Ihanda ang kukunang dokumento",
        card_detector_passport_manual_help_text: "Ihanda ang pasaporte",
        card_detector_certificate_manual_help_text:
            "Itakda ang certificate of residence para sa capture",
        // VIDEO IDENTIF. STEP
        video_identification_message_front: "Ipakita ang harap ng dokumento",
        video_identification_message_back: "Baliktarin ang dokumento",
        video_identification_message_passport: "Ipakita ang passport",
        video_identification_message_certificate: "Ipakita ang certificate",
        // FACE-CAPTURE STEP
        permissions_orientation_dialog_title:
            "Upang mapahusay ang iyong karanasan, kailangan naming bigyan mo kami ng access sa orientation ng iyong telepono",
        permissions_orientation_dialog_button: "OK",
        face_capture_device_motion_down: "Ibaba ang device",
        face_capture_device_motion_up: "Itinaas ang device",
        face_capture_detection_closer: "Lumapit",
        face_capture_detection_further: "Lumayo",
        face_capture_detection_multi_face: "Isa lang na mukha ang dapat",
        // DOC-CAPTURE STEP & VIDEO IDENTIF. STEP & FACE-CAPTURE STEP
        media_recorder_unknown_exception:
            "Ang device o browser ay hindi sumusuporta sa video recording",
        media_recorder_experimental_feature_exception:
            "Ang video recording ay hindi naka-enable sa iyong device. Pumunta sa Settings > Safari > Advanced > Experimental Features at i-activate ang MediaRecorder flag",
        // ATTACH STEP
        attach_button_text: "Hanapin ang file",
        attach_file_successfully_submitted:
            "Matagumpay na na-upload ang iyong file!",
        attach_on_uploading_file: "Ina-upload ang file, sandali lang",
        attach_multi_files_upload_error:
            "Isang file lang ang pinapayagang i-upload",
        attach_invalid_file_type_error: "May error sa pagbabasa ng file na ito",
        attach_processing_file: "Pinoproseso ang file...",
        attach_default_error:
            "Ang attachment ay mukhang hindi akma sa mga kinakailangang pamantayan",
        attach_error_button_text: "ULITIN",
        attach_not_supported_file: "Hindi suportadong uri ng file",
        attach_title_doc: "I-upload ang dokumento",
        attach_title_video: "I-upload ang video",
        attach_title_face: "I-upload ang selfie",
        attach_description_doc_front: "I-upload ang harap",
        attach_description_doc_back: "I-upload ang likod",
        attach_description_doc_passport: "I-upload ang harap",
        attach_description_doc_certificate: "I-upload ang likod",
        attach_description_video: "I-upload ang video",
        attach_description_face: "I-upload ang selfie",
        attach_button_text: "Hanapin ang file",
        // OTP STEP
        otp_verification_title: "Pag-verify ng OTP",
        otp_verification_email:
            "Ipinadala namin ang access code sa iyong email para sa verification.",
        otp_verification_sms:
            "Ipinadala namin ang access code sa iyong SMS para sa verification.",
        otp_verification_resend_question: "Hindi mo ba natanggap ang OTP code?",
        otp_verification_resended_email:
            "Isang bagong OTP code ang ipinadala sa iyong email address.",
        otp_verification_resended_sms:
            "Isang bagong OTP code ang ipinadala sa iyong mobile number.",
        otp_verification_invalid: "Hindi wasto ang OTP code",
        otp_verification_expired: "Paso na ang OTP code",
        otp_button: "Magpatuloy",
        // END STEP
        end_title: "Digital ID",
        end_description:
            "Napakadali ng proseso ng pagkakakilanlan. Kailangan mo lang ng iyong identity document at kumuha ng mga sumusunod gamit ang camera ng iyong telepono:",
        end_subtitle: "Natapos ang proseso!",
        end_button_text: "Tapos",
        // INITIAL STEP & END STEP
        intro_row_obverse: "Harap ng dokumento",
        intro_row_reverse: "Likod ng dokumento",
        intro_row_face: "Mukha at Pagkakakilanlan",
        intro_row_passport: `${intro_row_passport_tl}`,
        intro_row_residence_certificate: "Sertipiko ng paninirahan",
        // EXCEPTION VIEW
        exception_button_text_retry: "Subukan Muli",
        exception_button_text_go_init: "Bumalik",
        new_flow_exception_tips: [
            "Hindi masimulan ang bagong proseso ng pagkakakilanlan mula sa device.",
            "Pindutin ang subukang muli o subukan ulit mamaya",
        ],
        not_readable_exception_tips: [
            "Ang iyong audio/video device ay ginagamit ng ibang aplikasyon.",
            "Isara ang aplikasyon na gumagamit ng iyong device at i-restart ang proseso.",
        ],
        recording_exception_tips: [
            "Ang device o browser ay hindi sumusuporta sa pagre-record ng video",
            "Ang mga browser na sumusuporta sa pagre-record ng video ay:",
            [
                "Chrome Desktop (>v49)",
                "Firefox Desktop (>v29)",
                "Edge Desktop(>v76)",
                "Safari Desktop (>v13)",
                "Opera Desktop (>v62)",
                "Chrome para sa Android",
            ],
        ],
        connection_generic_error_tips: [
            "Oh! Mukhang nagkaroon ng error sa pagkonekta sa server",
            "Paki-check ang iyong koneksyon sa internet o subukan ulit sa loob ng limang minuto.",
        ],
        unknown_media_exception_tips: [
            "Nagkaroon ng hindi kilalang error sa pag-access ng iyong video/audio device",
            "Pakisubukang i-restart ang proseso ng pagkakakilanlan ng video",
        ],
        unknown_attach_exception_tips: [
            "Nagkaroon ng hindi kilalang error sa pag-attach ng ebidensya",
            "Pakisubukang muli mamaya",
        ],
        webrtc_exception_tips: [
            "Ang koneksyon sa streaming server ay naputol.",
            [
                "Maaaring may nangyaring problema sa network na naging sanhi ng pagkawala.",
                "Pakisubukang muli sa loob ng ilang sandali.",
            ],
        ],
        global_timeout_exception_tips: [
            "Naubos na ang kabuuang oras ng proseso",
            "Pindutin ang subukang muli upang i-restart ang proseso",
        ],
        no_devices_found_tips: [
            "Walang natagpuang audio/video device.",
            "Ikonekta ang isang device at i-restart ang proseso.",
        ],
        upload_exception_tips: [
            "Nagkaroon ng error sa koneksyon habang ipinapadala ang ebidensya ng pagkakakilanlan.",
            [
                "Maaaring may nangyaring problema sa network na naging sanhi ng pagkawala.",
                "Pakisubukang muli sa loob ng ilang sandali.",
            ],
        ],
        upload_check_exception_tips: [
            "Hindi maayos ang pagbasa sa iyong dokumento. Ito ay maaring dahil sa:",
            [
                "Hindi sapat ang kalidad ng litrato. Tandaan na ito ay dapat malinaw at nababasa.",
                "Ang dokumento ay hindi kasama sa mga tinatanggap na uri.",
                "Ang dokumento ay nag-expire na",
                "Ang mukha sa dokumento ay hindi tugma sa naunang larawan",
            ],
            "Pakisubukang muli sa pamamagitan ng paggawa ng mga sumusunod:",
            [
                "Ilagay ang ID sa isang maliwanag at patag na lugar",
                "Siguraduhing may sapat na ilaw",
                "Siguraduhing hindi nakakasilaw at walang anino ang litrato",
                "Itapat ang dokumento sa kuwadro na nasa screen",
            ],
        ],
        face_detector_timeout_exception_tips: [
            "Oh! May mali, hindi namin nakuha ang kanyang mukha.",
            "Pakisubukang muli sa pamamagitan ng paggawa ng mga sumusunod:",
            [
                "Pumunta sa isang maliwanag na lugar",
                "Subukang i-place ang iyong mukha sa loob ng gabay na lumalabas sa screen",
                "Tumingin nang diretso sa camera.",
            ],
        ],
        manual_face_detector_exception_tips: [
            "Walang mukhang nakita sa selfie photo:",
            ["Pakituunan ang iyong mukha habang kinukunan ang larawan."],
        ],
        manual_multi_face_detector_exception_tips: [
            "Maraming mukha ang nakita sa selfie photo:",
            ["Pakisiguro na isang mukha lang ang dapat lumitaw."],
        ],
        card_detector_timeout_exception_tips: [
            "Oh! May mali, hindi namin nakuha ang iyong ID.",
            "Pakisubukang muli sa pamamagitan ng paggawa ng mga sumusunod:",
            [
                "Ilagay ang dokumento ng ID sa isang patag, maliwanag na ibabaw.",
                "I-align ang silweta na iginuhit sa screen sa imahe sa iyong camera.",
            ],
        ],
        not_allowed_permission_exception_tips: [
            "Hindi namin ma-access ang device dahil hindi mo pinahintulutan ang pag-access sa camera at/o mikropono.",
            "Paki-enable ang mga pahintulot sa camera at audio at i-restart ang proseso.",
        ],
        overconstraint_exception_tips: [
            "Ang iyong device ay hindi nakakatugon sa minimum na mga kinakailangan upang maisagawa ang proseso.",
            "Paki-palitan ang device at ulitin ang proseso",
        ],
        invalid_flow_exception: [
            "Ang na-configure na daloy ay hindi valid.",
            "Paki-kontakin ang iyong system administrator.",
        ],
        unsupported_browser_exception_tips: [
            "Ang browser ay hindi compatible sa VideoStreaming.",
            "Paki-gamit ang ibang browser tulad ng Chrome o Firefox.",
        ],
        unsupported_browser_exception_ios_tips: [
            "Ang browser ay hindi sumusuporta sa VideoStreaming.",
            "Paki-gamit ang Safari.",
        ],
        unupdate_browser_exception_tips: [
            "Ang bersyon ng iyong browser ay hindi compatible.",
            "Pakibago ang iyong browser at subukang muli",
        ],
        dob_api_face_too_close_tips: [
            "Masyadong malapit ang mukha sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_eyes_closed_tips: [
            "Nakasara ang mga mata sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_face_close_to_border_tips: [
            "Masyadong malapit ang mukha sa gilid.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_face_cropped_tips: [
            "Napuputol ang mukha sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_face_is_occluded_tips: [
            "Natatakpan ang mukha sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_face_not_found_tips: [
            "Hindi namin nakita ang mukha sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_too_many_faces_tips: [
            "Maraming mukha ang nasa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_face_too_small_tips: [
            "Masyadong maliit ang mukha sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_face_angle_too_large_tips: [
            "Masyadong nakahilig ang camera sa selfie.",
            "I-center ang iyong mukha sa oval kapag kumukuha ng larawan at siguraduhing maayos ang itsura.",
        ],
        dob_api_non_configured_otp_contact_method: [
            "Ang OTP contact method ay hindi na-configure.",
            "Paki-kontakin ang iyong system administrator.",
        ],
        dob_api_maximum_number_of_otp_forwards_has_been_exceeded: [
            "Naabot na ang maximum na bilang ng OTP forwards.",
        ],
        dob_api_maximum_number_of_otp_reintent_has_been_exceeded: [
            "Naabot na ang maximum na bilang ng OTP retries.",
        ],
        dob_api_contact_method_does_not_exist: [
            "Ang contact method ay hindi umiiral.",
            "Paki-kontakin ang iyong system administrator.",
        ],
        dob_api_mandatory_otp_phone_number_not_informed: [
            "Kailangan ang OTP phone number, ngunit hindi ito naiulat.",
            "Paki-kontakin ang iyong system administrator o subukang muli.",
        ],
        dob_api_mandatory_otp_email_not_informed: [
            "Kailangan ang OTP email, ngunit hindi ito naiulat.",
            "Paki-kontakin ang iyong system administrator o subukang muli.",
        ],
        dob_api_non_valid_otp_phone_number: [
            "Hindi valid ang OTP phone number.",
            "Paki-kontakin ang iyong system administrator o subukang muli.",
        ],
        dob_api_non_valid_otp_email: [
            "Hindi valid ang OTP email.",
            "Paki-kontakin ang iyong system administrator o subukang muli.",
        ],
        dob_api_otp_has_already_been_validated: [
            "Na-validate na ang OTP.",
            "Paki-kontakin ang iyong system administrator.",
        ],
        dob_api_transaction_does_not_exist: [
            "Hindi umiiral ang transaksyon.",
            "Paki-kontakin ang iyong system administrator o subukang muli.",
        ],
        // HELP DIALOG VIEW
        secondarytoolbar_help_title: "IMPORMASYON",
        default_instructions_docs: [
            "Ilagay ang dokumento ng pagkakakilanlan sa isang patag, malinaw na ibabaw",
            "Ang dokumento ay hindi dapat may shine o mga reflection na maaaring magpahirap sa pagbabasa",
            "Pumantay ang silweta na nasa screen sa imahe ng iyong kamera",
            "Ang capture ay magagawa nang awtomatiko kapag ang silweta at imahe ay pumantay",
        ],
        default_instructions_face: [
            "Ilagay ang iyong sarili sa isang malinaw na lugar",
            "Subukang ilagay ang iyong mukha sa loob ng gabay na lumilitaw sa screen",
            "Tumingin ng diretso sa kamera",
            "Alalahanin na ipakita ang iyong ID sa parehong panig",
            "Alalahanin na walang iba pang tao ang dapat lumitaw sa video",
        ],
        secondarytoolbar_help_button: "SARADO",
        // MANUAL CAPTURE VIEW
        manual_capture_doc_title_text: "Manual na pag-capture ng dokumento",
        manual_capture_doc_lead_text:
            "Ilagay ang dokumento sa loob ng frame at pindutin ang button",
        manual_capture_face_title_text: "Manual na pag-capture ng selfie",
        manual_capture_face_lead_text:
            "Pumokus sa mukha upang kumuha ng selfie at pindutin ang button upang makapag-capture ng litrato",
        // PREVIEW VIEW
        attach_preview_retry_button: "ULITIN",
        attach_preview_continue_button: "MAGPATULOY",
        // PREVIEW VIEW | ONLY DESIGN GENERIC
        preview_capture_doc_text: "Siguraduhing nababasa at malinaw ang kuha",
        // PREVIEW VIEW | ONLY RESPONSIVE DESIGN
        attach_preview_text:
            "Alalahanin na ang imahe ay dapat na naka-orient sa tama at naipakita nang wasto",
        // LOADER VIEW
        default_progress_description: "Kumokonekta...",
        video_progress_description: "Kumokonekta...",
        end_progress_description: "Patapos na...",
        new_device_progress_description:
            "Nagsisimula ng bagong flow mula sa device...",
        otp_configuration_progress_description:
            "Naglo-load ng configuration...",
        otp_forwarding_progress_description: "Nagpapadala ng OTP...",
        otp_verification_progress_description: "Nagve-verify ng OTP...",
        media_device_progress_description: "Sandali lang...",
        background_progress_description:
            "Hawakan ang screen upang magpatuloy...",
        // TOOLBAR COMPONENT
        secondarytoolbar_identification_error: "Kamalian sa pagkakakilanlan",
        secondarytoolbar_obverse: "Harapan ng Dokumento",
        secondarytoolbar_reverse: "Likuran ng Dokumento",
        secondarytoolbar_face: "Mukha at Pagkakakilanlan",
        secondarytoolbar_passport: `${intro_row_passport_tl}`,
        secondarytoolbar_certificate: "Sertipiko ng Pagtira",
        secondarytoolbar_exit_button: "LUMABAS",
        dob_tooltip_show_help: "Ipakita ang Tulong",
        dob_tooltip_leave_process: "Lumabas sa Proseso",
        dob_tootltip_take_photo: "Kumuha ng Litrato",
        // INFOBAR COMPONENT
        card_detector_verifying: "Sinusuri ang dokumento...",
        infobar_start_text: "Kinukonfigura ang scanner",
        infobar_working_card_capture_text:
            "Ilagay ang ID card sa patag at maliwanag na ibabaw",
        infobar_uploading_text: "Ipinapadala ang mga file...",
        infobar_finish_text: "Perpekto!",
        infobar_passport_start_text:
            "Ilagay ang pasaporte sa patag at maliwanag na lugar",
        infobar_passport_working_card_capture_text:
            "Ilagay ang pasaporte sa patag at maliwanag na lugar",
        infobar_passport_uploading_text: "Ipinapadala namin ang litrato",
        infobar_passport_finish_text: "Perpekto!",
        infobar_passport_end_text: "Nakuha na namin ang pasaporte",
        infobar_certificate_start_text:
            "Ilagay ang sertipiko sa patag at maliwanag na ibabaw",
        infobar_certificate_working_card_capture_text:
            "Ilagay ang sertipiko sa patag at maliwanag na ibabaw",
        infobar_certificate_uploading_text: "Ipinapadala namin ang litrato",
        infobar_certificate_finish_text: "Perpekto!",
        infobar_certificate_end_text: "Nakuha na namin ang sertipiko",
        infobar_working_face_capture_text:
            "Subukang ilagay ang iyong mukha sa loob ng gabay na makikita sa screen",
        infobar_video_identification_front_text:
            "Hawakan ang ID sa gilid, ipakita ang harap at pumuwesto para sa video",
        infobar_video_identification_back_text:
            "Ngayon ipakita ang likod ng dokumento ng pagkakakilanlan. Huwag kalimutang hawakan ito sa gilid",
        infobar_face_detector_verifying: "Sinusuri ang selfie...",
        // SPEECH SYNTHESIS
        speech_synthesis_capture_doc_front:
            "Itugma ang harap ng iyong dokumento sa silweta",
        speech_synthesis_capture_doc_front_manual:
            "Kuhanan ng litrato ang harap ng iyong dokumento",
        speech_synthesis_capture_doc_front_qr:
            "Gamitin ang iyong mobile upang i-scan ang QR code at kunan ng litrato ang harap ng dokumento",
        speech_synthesis_capture_doc_back:
            "Itugma ang likod ng iyong dokumento sa silweta",
        speech_synthesis_capture_doc_back_manual:
            "Kuhanan ng litrato ang likod ng iyong dokumento",
        speech_synthesis_capture_doc_back_qr:
            "Ngayon gamit ang iyong mobile, kunan ng litrato ang likod ng dokumento",
        speech_synthesis_capture_doc_passport:
            "Kuhanan ng litrato ang pasaporte",
        speech_synthesis_capture_doc_passport_qr:
            "Gamitin ang iyong mobile upang i-scan ang QR code at kunan ng litrato ang pasaporte",
        speech_synthesis_capture_doc_certificate:
            "Kuhanan ng litrato ang sertipiko ng tirahan",
        speech_synthesis_capture_doc_certificate_qr:
            "Ngayon gamit ang iyong mobile, kunan ng litrato ang sertipiko",
        speech_synthesis_capture_doc_finish_qr:
            "Ipagpatuloy ang pagproseso gamit ang aparatong ito",
        speech_synthesis_video_identification_front:
            "Ipakita ang harap ng dokumento",
        speech_synthesis_video_identification_back:
            "Ipakita ang likod ng dokumento",
        speech_synthesis_video_identification_passport: "Ipakita ang pasaporte",
        speech_synthesis_video_identification_certificate:
            "Ipakita ang sertipiko",
        speech_synthesis_face_capture: "Maghanda upang kumuha ng selfie",
        speech_synthesis_face_capture_manual:
            "Kumuha ng selfie sa pamamagitan ng pag-click sa capture button",
        speech_synthesis_attach_doc_front:
            "I-attach ang litrato ng harap ng dokumento",
        speech_synthesis_attach_doc_back:
            "I-attach ang litrato ng likod ng dokumento",
        speech_synthesis_attach_video_identification:
            "I-attach ang video identifier kasama ang iyong mukha",
        speech_synthesis_attach_face: "I-attach ang litrato ng iyong mukha",
        speech_synthesis_attach_passport: "I-attach ang litrato ng pasaporte",
        speech_synthesis_attach_ue:
            "I-attach ang litrato ng sertipiko ng tirahan ng European Union",
        // HELP ORIENTATION
        helporientation_title: "Pakibago ang oryentasyon sa portrait",
        helporientation_button: "ISARA",
        // HELP PERMISSIONS VIEW
        help_permissions_title: "Mga pahintulot para sa kamera at mikropono",
        help_permissions_description:
            "Kailangan naming payagan mo ang mga pahintulot upang ma-access ang kamera at mikropono. Mahalaga ang mga ito upang maisagawa ang video identification.",
        // EXIT DIALOG VIEW
        exit_dialog_title: "Abiso",
        exit_dialog_subtitle: "Gusto mo bang kanselahin ang proseso?",
        exit_dialog_accept: "Oo",
        exit_dialog_cancel: "Hindi",
        // QR VIEW
        qr_capture_doc_front:
            "I-scan ang QR code gamit ang iyong mobile upang kunan ng litrato ang harap ng dokumento.",
        qr_capture_doc_back:
            "Ngayon kunan ng litrato ang likod ng dokumento gamit ang iyong mobile.",
        qr_connect_error:
            "Mga problema sa koneksyon, pakisubukang i-scan muli ang QR code pagkatapos ng ilang sandali.",
        // CUSTOM STRINGS INSTRUCTIONS STEP
        custom_instructions_doc_title: "Digital na Pagkakakilanlan",
        custom_instructions_doc_description:
            "Susunod naming kukunan ang harap at likod. Kung maaari, maghanap ng madilim na background at ilagay ang dokumento sa patag na ibabaw.",
        custom_instructions_doc_button: "Magpatuloy",
        // CUSTOM STRINGS INITIAL/END STEP - EXAMPLE: PASSPORT
        custom_intro_row_obverse: "Pasaporte",
        custom_intro_row_reverse: "Mukha na may Pasaporte",
        // CUSTOM STRINGS INSTRUCTIONS STEP - EXAMPLE: PASSPORT
        custom_instructions_step_title: "Mga Tagubilin para sa Pasaporte",
        custom_instructions_step_description:
            "Una, sisimulan natin sa pagkuha ng litrato ng pasaporte.",
    },
};

console.log("LOCALE: ", window.LOCALE);
let myStrings = myStringsLocaleMap[window.LOCALE];
env_config = eval("(" + window.DOB_ENV_CONFIG + ")");

// Configuramos el SDK
sdk = {
    ak: window.DOB_API_KEY,
    flow: flow(),
    applicationId: window.DOB_APP_ID,
};

// Add default assets if not set in configuration
if (!env_config.baseAssetsUrl) {
    env_config.baseAssetsUrl = window.ASSETS_URL;
}

session = {
    tokenDOB: window.DOB_DATA.td,
    userID: window.DOB_DATA.uid,
};

let dobSdk = document.getElementById("dob-sdk");
dobSdk.session = session;
dobSdk.sdk = sdk;
dobSdk.env_config = env_config;

// Listen to status changes
dobSdk.addEventListener("status", (status) => {
    const parsedStatus = status.detail;
    // No face falta filtrar por tipo, es solo para el ejemplo.
    // Esto simplemente es un ejemplo para imprimir la INFO que enviamos en cada paso del SDK.
    switch (parsedStatus.type) {
        case EventType.load:
            console.log(
                "load: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.working:
            console.log(
                "working: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.uploadEvidenceStart:
            console.log(
                "Upload evidence start: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.uploadEvidenceEnd:
            console.log(
                "Upload evidence end: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.verifying:
            console.log(
                "Verifying: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.error:
            console.log(
                "error: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.exception:
            console.log(
                "exception: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            break;
        case EventType.end:
            console.log(
                "End: " +
                    parsedStatus.step +
                    " extra-info: " +
                    parsedStatus.info
            );
            // Ocultamos strings de flujo responsive al terminar de subir el selfie
            if (parsedStatus.step === "attach_selfie") {
                removeMessagesAttach();
            }
            break;
    }
    // No face falta filtrar por step, es solo para el ejemplo.
    // Esto simplemente es un ejemplo para cambiar strings en caso de querer crearlos si usamos el diseño responsive.
    switch (parsedStatus.step) {
        case "attach_front":
            setStringHtmlValues(
                "Front of the document",
                "Attach a photo of your Identity Document face up. Make sure the image is of sufficient quality and the information and photo it contains are clearly visible."
            );
            break;
        case "attach_back":
            setStringHtmlValues(
                "Back of the document",
                "Now attach a photo of your Identity Document face down. Make sure the image is of sufficient quality and the information is clearly visible."
            );
            break;
        case "attach_passport":
            setStringHtmlValues(
                "Passport",
                "Please attach an image of your passport. Make sure the image is correctly visible and of sufficient quality."
            );
            break;
        case "attach_ue":
            setStringHtmlValues(
                "European Union Resident Certificate",
                "Now we need you to attach your European Union resident certificate."
            );
            break;
        case "attach_video":
            setStringHtmlValues(
                "Video of your face",
                "Now, take a video of your face and attach the video so we can verify that the document belongs to you."
            );
            break;
        case "attach_selfie":
            setStringHtmlValues(
                "Photo of your face",
                "Now, take a selfie of your face and attach the photo so we can verify that the document belongs to you. Make sure the image is of sufficient quality."
            );
            break;
    }
    // Esto simplemente es un ejemplo para detectar que el último paso a terminado [En caso de no informar el EndStep ni urls de callbacks]
    if (
        parsedStatus.type === EventType.end &&
        parsedStatus.step === sdk.flow[sdk.flow.length - 1].getStepName()
    ) {
        console.log(
            "End: " + parsedStatus.step + " - is last step, do something..."
        );
    }
    // Esto simplemente es un ejemplo de como cambiar los textos en la vista de instrucciones en caso del documento, según el nombre del step
    if (parsedStatus.step === "show-doc-front") {
        myStrings["instructions_step_title"] =
            myStrings["custom_instructions_doc_title"];
        myStrings["instructions_step_description"] =
            myStrings["custom_instructions_doc_description"];
        myStrings["instructions_step_button"] =
            myStrings["custom_instructions_doc_button"];
        dobSdk.env_config
            ? (dobSdk.env_config.customTextsConfig = myStrings)
            : "";
    }
    // Esto simplemente es un ejemplo de como cambiar los textos e iconos en la vista inicial y final en caso de capturar un pasaporte, según el nombre del step
    if (
        parsedStatus.step === "permissions-passport" ||
        parsedStatus.step === "end-passport"
    ) {
        // initial styles
        const initial_step = document.getElementById(
            "dob-initial-information-layout"
        );
        if (initial_step) {
            const initial_card_front = initial_step.getElementsByClassName(
                "dob-card-front-icon"
            )[0];
            const initial_card_back =
                initial_step.getElementsByClassName("dob-card-back-icon")[0];
        }
        // end styles
        const end_step = document.getElementById("dob-end-information-layout");
        if (end_step) {
            const end_card_front = end_step.getElementsByClassName(
                "dob-card-front-icon"
            )[0];
            const end_card_back =
                end_step.getElementsByClassName("dob-card-back-icon")[0];
        }
        // Strings
        myStrings["intro_row_obverse"] = myStrings["custom_intro_row_obverse"];
        myStrings["intro_row_reverse"] = myStrings["custom_intro_row_reverse"];
        myStrings["speech_synthesis_capture_doc_front_manual"] = "";
        dobSdk.env_config
            ? (dobSdk.env_config.customTextsConfig = myStrings)
            : "";
    }

    // Esto simplemente es un ejemplo de como cambiar los textos en la vista de instrucciones en caso de capturar un pasaporte, según el nombre del step
    if (parsedStatus.step === "instructions-passport") {
        myStrings["instructions_step_title"] =
            myStrings["custom_instructions_step_title"];
        myStrings["instructions_step_description"] =
            myStrings["custom_instructions_step_description"];
        dobSdk.env_config
            ? (dobSdk.env_config.customTextsConfig = myStrings)
            : "";
    }

    if (parsedStatus.step === "instructions-face" && isPassportFlow) {
        myStrings["instructions_step_title"] =
            myStrings["passport_custom_instructions_step_title"];
        myStrings["instructions_step_description"] =
            myStrings["passport_custom_instructions_step_description"];
        dobSdk.env_config
            ? (dobSdk.env_config.customTextsConfig = myStrings)
            : "";
    }
    // Esto simplemente es un ejemplo de como cambiar los iconos en el componente secundary toolbar en caso de capturar un pasaporte, según el nombre del step
    if (
        parsedStatus.step === "passport-capture" ||
        parsedStatus.step === "certificate-capture"
    ) {
        const secondary_toolbar = document.getElementById(
            "dob-secondary-toolbar"
        );
        if (secondary_toolbar) {
            const secondary_toolbar_card_front =
                secondary_toolbar.getElementsByClassName(
                    "dob-card-front-icon"
                )[0];
            const secondary_toolbar_card_back =
                secondary_toolbar.getElementsByClassName(
                    "dob-card-back-icon"
                )[0];
            if (secondary_toolbar_card_front) {
                // Passport
                secondary_toolbar_card_front.style.background =
                    'url("assets/images/icons/passport_icon.svg") no-repeat center';
                secondary_toolbar_card_front.style.backgroundSize = "100% 70%";
            } else {
                // Certificate
                secondary_toolbar_card_back.style.background =
                    'url("assets/images/icons/passport_icon.svg") no-repeat center';
                secondary_toolbar_card_front.style.backgroundSize = "100% 70%";
            }
        }
    }
});

// Listen to evidence changes
dobSdk.addEventListener("evidence", (evidence) => {
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

// Listen when the SDK has finished successfully
dobSdk.addEventListener("success", (success) => {
    console.log("sdk-success: SDK-Web onSuccess()");
    document.getElementById("kc-inetum-success-form").submit();
});

let attempt = 0;
let maxRetries = 3;
// Listen to failure events
dobSdk.addEventListener("failure", (error) => {
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
            console.log("SDK-Web onFailure()");
            // En caso de que el usuario pulse el boton redirigimos
            if (parsedFailure.clickedButton) {
                window.location.replace("https://comelec.gov.ph/");
            }
            /*
      //En caso de querer forzar la redireccion
      if (!parsedFailure.clickedButton) {
        setTimeout(() => {  window.location.replace('https://google.es'); }, 1500);
      }
      */
            break;
        case ExceptionType.uploadAndCheckException:
            attempt++;
            if (attempt >= maxRetries) {
                // Submit the form with the failure reason
                const form = document.getElementById("kc-inetum-success-form");
                let errorInput = document.createElement("input");
                errorInput.type = "hidden";
                errorInput.name = "error_code";
                errorInput.value = parsedFailure.code;
                form.appendChild(errorInput);
                form.submit();
            }
    }
});
