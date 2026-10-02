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
    "messageChannel.EMAIL", "messageChannel.SMS", "messageChannel.WHATSAPP",
    "messageChannel.VIBER", "messageChannel.MESSENGER",
    "messageOtp.auth.sentTo", "messageOtp.auth.sentToChannel", "messageOtp.auth.openApp",
    "messageOtp.otherWay.title", "messageOtp.otherWay.help", "messageOtp.otherWay.send",
    "messageOtp.choose.title", "messageOtp.choose.help", "messageOtp.choose.option",
    "messageOtp.delivery.unknown", "messageOtp.delivery.failed",
    "messageOtp.messenger.title", "messageOtp.messenger.intro", "messageOtp.messenger.connect",
    "messageOtp.messenger.step1", "messageOtp.messenger.step2", "messageOtp.messenger.step3",
    "messageOtp.messenger.word", "messageOtp.messenger.scan", "messageOtp.messenger.check",
    "messageOtp.messenger.pending", "messageOtp.messenger.codeSent", "messageOtp.messenger.expired"
] as key>
<#if msg(key) != key>
window.kcContext["x-keycloakify"].messages["${key}"] = decodeHtmlEntities("${msg(key)?js_string}");
</#if>
</#list>
</script>
