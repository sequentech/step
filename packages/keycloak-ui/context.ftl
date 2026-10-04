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
<#-- The registration form's settings, which Keycloakify can't read from the data model. -->
<#if profile??>
window.kcContext.sequentRegistration = {
    <#if formMode?? && formMode?is_string>formMode: "${formMode?js_string}",</#if>
    credentialFieldPosition: "${(realm.attributes['credential-field-position']!'LAST')?js_string}",
    hiddenAttributes: [<#list hiddenProfileAttributes![] as name>decodeHtmlEntities("${name?js_string}")<#sep>, </#list>],
    lockedAttributes: [<#list loginHintReadOnlyAttributes![] as name>decodeHtmlEntities("${name?js_string}")<#sep>, </#list>]
};
</#if>
<#-- Fields without a value are listed too. -->
<#if (rejectReason?? && rejectReason?is_string) || mismatchedFields??>
window.kcContext.enrollmentOutcome = {
    <#if rejectReason?? && rejectReason?is_string>reason: decodeHtmlEntities("${rejectReason?js_string}"),</#if>
    mismatchedFields: [<#if mismatchedFields??><#list mismatchedFields?keys as name>{name: decodeHtmlEntities("${name?js_string}"), value: <#if mismatchedFields[name]??>decodeHtmlEntities("${mismatchedFields[name]?js_string}")<#else>null</#if>}<#sep>, </#list></#if>]
};
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
<#if rejectReason?? && rejectReason?is_string>
<#assign scanovateKeys = scanovateKeys + [rejectReason]>
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
    "messageOtp.auth.codeProgress", "messageOtp.auth.codeNext", "messageOtp.auth.codeLast",
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
    "scanovateTryAgain",
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
    "scanovateGuideDocumentNotAligned", "scanovateGuideDocumentTilted", "scanovateGuideTooDark",
    "scanovateGuideDocumentTooBright",
    "scanovateGuideSunglasses", "scanovateGuideFaceCovered",
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
    "scanovateCheckingTitle", "scanovateCheckingLead",
    "scanovateCheckingVerifying", "scanovateCheckingReading", "scanovateErrorHeading",
    "scanovateErrorFinalHeading", "scanovateErrorTipsTitle", "scanovateTipLight",
    "scanovateTipCorners", "scanovateTipSteady", "scanovateTipValidDocument",
    "scanovateTipFaceUncovered", "scanovateTipLookAtCamera", "scanovateTipSameDocument",
    "scanovateTipCheckDetails", "scanovateTipConnection", "scanovateTipWait",
    "scanovateAttemptsLeft", "scanovateAttemptsLeftOne", "scanovateSupportReferenceLabel",
    "scanovateCopy", "scanovateCopyReference", "scanovateCopied", "scanovateCopyFailed",
    "scanovateInternalError", "scanovateVerificationFailedError",
    "scanovateDocumentAuthenticationError", "scanovateDocumentUnreadableError",
    "scanovateAttributesError",
    "scanovateScoringError", "scanovateMaxRetriesError", "scanovateCaptureInvalidError",
    "scanovateLivenessError", "scanovateLivenessFailedTitle", "scanovateLivenessFailedText",
    "scanovateLivenessExpiredTitle", "scanovateLivenessExpiredText", "scanovateStartOver",
    "scanovateUploadFailedTitle", "scanovateUploadFailedText", "scanovateCaptureExpiredTitle",
    "scanovateCaptureExpiredText",
    "scanovateCheckingReceivedLiveness", "scanovateLivenessChecking",
    "scanovateFaceMismatchError", "scanovateFaceNotFoundError",
    "scanovateConfirmTitle", "scanovateConfirmLead", "scanovateConfirmDocument",
    "scanovateConfirmSubmit", "scanovateConfirmRetry", "dateOfBirth",
    "sequent.read-only.id-card-number",
    "registerTitle", "backToLogin", "passwordConfirm", "showPassword", "hidePassword",
    "requiredFields", "enrollmentStepDetails", "enrollmentPasswordStrength",
    "enrollmentOutcomeEnrolled", "enrollmentOutcomeUnderReview", "enrollmentOutcomeRejected",
    "enrollmentOutcomeValidated", "registerFinishTitle", "registerFinishMessage",
    "registerFinishManualTitle", "registerFinishManualMessage", "registerFinishRejectedTitle",
    "registerFinishRejectedMessage", "rejectReasonListItems", "messageFinishTitle",
    "messageFinish", "pageExpiredMsg2", "doClickHere", "empty"
] + scanovateKeys as key>
<#if msg(key) != key>
window.kcContext["x-keycloakify"].messages["${key}"] = decodeHtmlEntities("${msg(key)?js_string}");
</#if>
</#list>
</script>
