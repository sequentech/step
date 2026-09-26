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
    "invalidCredentialsMessage", "messageOtp.auth.address", "messageOtp.auth.instructionBoth",
    "messageOtp.auth.instructionSms", "messageOtp.auth.instructionEmail", "messageOtp.auth.ttlTime",
    "messageOtp.auth.resend.button", "messageOtp.auth.resend.timer", "messageOtp.otl.address",
    "messageOtp.otl.instructionBoth", "messageOtp.otl.instructionSms", "messageOtp.otl.instructionEmail",
    "messageOtp.otl.ttlTime", "messageOtp.otl.resend.button", "messageOtp.otl.resend.timer"
] as key>
window.kcContext["x-keycloakify"].messages["${key}"] = decodeHtmlEntities("${msg(key)?js_string}");
</#list>
</script>
