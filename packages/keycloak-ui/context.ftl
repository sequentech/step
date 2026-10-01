<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
 SPDX-License-Identifier: AGPL-3.0-only
-->
<script>
// The UI receives only the presentation policies it implements, never the realm attribute map.
window.kcContext.sequent = {
    loginValidationPolicy: "${(realm.attributes['login-validation-policy']!'BROWSER')?js_string}",
    loginHintUsernamePolicy: "${(realm.attributes['loginHintUsernamePolicy']!'EDITABLE')?js_string}"
};
<#if courier??>
window.kcContext.courier = "${courier?string?js_string}";
</#if>
<#-- Keys only known at run time: the voter's document type and the attributes read from it. -->
<#assign scanovateKeys = []>
<#if scanovate?? && scanovate.documentType??>
<#assign scanovateKeys = scanovateKeys + ["scanovateDocumentType." + scanovate.documentType]>
</#if>
<#if documentType?? && documentType?is_string>
<#assign scanovateKeys = scanovateKeys + ["scanovateDocumentType." + documentType]>
</#if>
<#if error?? && error?is_string>
<#assign scanovateKeys = scanovateKeys + [error]>
</#if>
<#if storedAttributes??>
<#list storedAttributes as attribute>
<#if attribute.key??>
<#assign scanovateKeys = scanovateKeys + [attribute.key]>
</#if>
</#list>
</#if>
<#list [
    "loginAccountTitle", "doLogIn", "username", "usernameOrEmail", "email", "password",
    "rememberMe", "doForgotPassword", "noAccount", "doRegister", "doSubmit", "languages",
    "doContinue", "identity-provider-login-label",
    "system.version", "system.hash",
    "otpDigit", "otpCodeLabel",
    "invalidCredentialsMessage", "messageOtp.auth.title", "messageOtp.otl.title",
    "messageOtp.auth.address", "messageOtp.auth.instructionBoth",
    "messageOtp.auth.instructionSms", "messageOtp.auth.instructionEmail", "messageOtp.auth.ttlTime",
    "messageOtp.auth.resend.button", "messageOtp.auth.resend.timer", "messageOtp.otl.address",
    "messageOtp.otl.instructionBoth", "messageOtp.otl.instructionSms", "messageOtp.otl.instructionEmail",
    "messageOtp.otl.ttlTime", "messageOtp.otl.resend.button", "messageOtp.otl.resend.timer",
    "scanovateStepEyebrow", "scanovateStepVerifyIdentity", "scanovateStepConfirm",
    "scanovateProgressLabel", "scanovateDocumentGeneric",
    "scanovateDocumentType.philippinePassport", "scanovateDocumentType.driversLicense",
    "scanovateDocumentType.philSysID", "scanovateDocumentType.iBP",
    "scanovateDocumentType.seamanBook", "scanovateIntroTitle", "scanovateIntroLead",
    "scanovateIntroDocumentBothSides", "scanovateIntroDocumentFrontSide",
    "scanovateIntroDocumentHint", "scanovateIntroFace", "scanovateIntroFaceHint",
    "scanovateIntroVideo", "scanovateIntroVideoHint", "scanovateIntroBeforeTitle",
    "scanovateIntroTipLight", "scanovateIntroTipCoverings", "scanovateIntroTipDocument",
    "scanovateIntroPrivacy", "scanovateIntroStart", "scanovateCameraStarting",
    "scanovateCameraDeniedTitle", "scanovateCameraDeniedText", "scanovateCameraNotFoundTitle",
    "scanovateCameraNotFoundText", "scanovateCameraInUseTitle", "scanovateCameraInUseText",
    "scanovateInsecureContextTitle", "scanovateInsecureContextText", "scanovateCameraFailedTitle",
    "scanovateCameraFailedText", "scanovateAnalyzerFailedTitle", "scanovateAnalyzerFailedText",
    "scanovateRecorderUnsupportedTitle", "scanovateRecorderUnsupportedText", "scanovateTryAgain",
    "scanovateBackToStart", "scanovateCaptureFrontTitle", "scanovateCaptureBackTitle",
    "scanovateCaptureFaceTitle", "scanovateCaptureVideoTitle", "scanovateCaptureCounter",
    "scanovateCaptureFrontHeading", "scanovateCaptureFrontText", "scanovateCaptureBackHeading",
    "scanovateCaptureBackText", "scanovateCaptureFaceHeading", "scanovateCaptureFaceText",
    "scanovateCaptureVideoHeading", "scanovateCaptureVideoText", "scanovateChipFront",
    "scanovateChipBack", "scanovateChipFace", "scanovateCaptureSecure", "scanovateTakePhoto",
    "scanovateStop", "scanovateHelp", "scanovateCaptured", "scanovateRecording",
    "scanovateRecordingStatus", "scanovateStepDone", "scanovateStepCurrent",
    "scanovateGuideLoading", "scanovateGuidePlaceDocument", "scanovateGuideTurnDocument",
    "scanovateGuideDocumentTooFar", "scanovateGuideDocumentTooClose",
    "scanovateGuideDocumentNotAligned", "scanovateGuideTooDark", "scanovateGuideDocumentTooBright",
    "scanovateGuideGlare", "scanovateGuideDocumentBlurry", "scanovateGuideDocumentHoldStill",
    "scanovateGuidePlaceFace", "scanovateGuideMultipleFaces", "scanovateGuideFaceTooFar",
    "scanovateGuideFaceTooClose", "scanovateGuideFaceOffCenter", "scanovateGuideTurnToCamera",
    "scanovateGuideFaceTooBright", "scanovateGuideFaceBlurry", "scanovateGuideFaceHoldStill",
    "scanovateGuideHoldDocument", "scanovateGuideRecording", "scanovateGuideKeepInView",
    "scanovateGuideCaptured", "scanovateHelpTitle", "scanovateHelpDocumentSurface",
    "scanovateHelpDocumentCorners", "scanovateHelpDocumentGlare", "scanovateHelpDocumentLight",
    "scanovateHelpShutter", "scanovateHelpFaceLevel", "scanovateHelpFaceCoverings",
    "scanovateHelpFaceLight", "scanovateHelpFaceAlone", "scanovateHelpVideoDocument",
    "scanovateHelpVideoFace", "scanovateHelpVideoStill", "scanovateHelpClose",
    "scanovateStopTitle", "scanovateStopText", "scanovateStopConfirm", "scanovateStopCancel",
    "scanovateCheckingTitle", "scanovateCheckingLead", "scanovateCheckingReceived",
    "scanovateCheckingVerifying", "scanovateCheckingReading", "scanovateErrorHeading",
    "scanovateErrorFinalHeading", "scanovateErrorTipsTitle", "scanovateTipLight",
    "scanovateTipCorners", "scanovateTipSteady", "scanovateTipValidDocument",
    "scanovateTipFaceUncovered", "scanovateTipLookAtCamera", "scanovateTipSameDocument",
    "scanovateTipCheckDetails", "scanovateTipCamera", "scanovateTipConnection", "scanovateTipWait",
    "scanovateAttemptsLeft", "scanovateAttemptsLeftOne", "scanovateSupportReferenceLabel",
    "scanovateCopy", "scanovateCopyReference", "scanovateCopied", "scanovateCopyFailed",
    "scanovateInternalError", "scanovateVerificationFailedError",
    "scanovateDocumentAuthenticationError", "scanovateMaxTrialsError", "scanovateAttributesError",
    "scanovateScoringError", "scanovateMaxRetriesError", "scanovateCaptureInvalidError",
    "scanovateLivenessError", "scanovateLivenessFailedTitle", "scanovateLivenessFailedText",
    "scanovateLivenessExpiredTitle", "scanovateLivenessExpiredText", "scanovateStartOver",
    "scanovateUploadFailedTitle", "scanovateUploadFailedText", "scanovateCaptureExpiredTitle",
    "scanovateCaptureExpiredText",
    "scanovateCheckingReceivedLiveness", "scanovateLivenessChecking",
    "scanovateFaceMismatchError", "scanovateFaceNotFoundError",
    "scanovateConfirmTitle", "scanovateConfirmLead", "scanovateConfirmDocument",
    "scanovateConfirmSubmit", "scanovateConfirmRetry", "dateOfBirth",
    "sequent.read-only.id-card-number"
] + scanovateKeys as key>
<#if msg(key) != key>
window.kcContext["x-keycloakify"].messages["${key}"] = decodeHtmlEntities("${msg(key)?js_string}");
</#if>
</#list>
</script>
