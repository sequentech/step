<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true displayMessage=!messagesPerField.existsError('username'); section>
    <#if section = "header">
        ${msg("emailForgotTitle")}
    <#elseif section = "form">
        <form
            id="kc-reset-password-form"
            class="${properties.kcFormClass!}"
            action="${url.loginAction}"
            method="post"
        >
            <div class="${properties.kcFormGroupClass!}">
                <div class="${properties.kcLabelWrapperClass!}">
                    <label for="username" class="${properties.kcLabelClass!}">
                        ${msg("username")}
                    </label>
                </div>
                <div class="${properties.kcInputWrapperClass!}">
                    <input
                        type="text"
                        id="username"
                        name="username"
                        class="${properties.kcInputClass!}"
                        autofocus
                        value="${(auth.attemptedUsername!'')}"
                        aria-invalid="<#if messagesPerField.existsError('username')>true</#if>"
                    />
                    <#if messagesPerField.existsError('username')>
                        <span 
                            id="input-error-username"
                            class="${properties.kcInputErrorMessageClass!}"
                            aria-live="polite">
                                ${kcSanitize(messagesPerField.get('username'))?no_esc}
                        </span>
                    </#if>
                </div>
            </div>
            <div class="${properties.kcFormGroupClass!}">
                <div class="${properties.kcLabelWrapperClass!}">
                    <label for="email" class="${properties.kcLabelClass!}">
                        ${msg("email")}
                    </label>
                </div>
                <div class="${properties.kcInputWrapperClass!}">
                    <input
                        type="text"
                        id="email"
                        name="email"
                        class="${properties.kcInputClass!}"
                        autofocus
                        value="${(auth.attemptedemail!'')}"
                        aria-invalid="<#if messagesPerField.existsError('email')>true</#if>"
                    />
                    <#if messagesPerField.existsError('email')>
                        <span 
                            id="input-error-email"
                            class="${properties.kcInputErrorMessageClass!}"
                            aria-live="polite">
                                ${kcSanitize(messagesPerField.get('email'))?no_esc}
                        </span>
                    </#if>
                </div>
            </div>
            <div
                class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}"
            >
                <div
                    id="kc-form-options"
                    class="${properties.kcFormOptionsClass!}"
                >
                    <div class="${properties.kcFormOptionsWrapperClass!}">
                        <span>
                            <a href="${url.loginUrl}">
                                ${kcSanitize(msg("backToLogin"))?no_esc}
                            </a>
                        </span>
                    </div>
                </div>

                <#if recaptchaEnabled??>
                    <input
                        type="hidden"
                        id="g-recaptcha-response"
                        name="g-recaptcha-response" />
                    <script>
                        var onRecaptchaLoaded = function()
                        {
                            grecaptcha
                                .execute(
                                    '${recaptchaSiteKey}',
                                    { action:'${recaptchaActionName}' }
                                )
                                .then(function(token) {
                                    document.getElementById(
                                        'g-recaptcha-response'
                                    ).value = token;
                                });
                        };
                    </script>
                </#if>

                <div 
                    id="kc-form-buttons"
                    class="${properties.kcFormButtonsClass!}"
                >
                    <input
                        class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                        type="submit"
                        value="${msg("doSubmit")}"
                    />
                </div>
            </div>
        </form>
    <#elseif section = "info" >
        <#if realm.duplicateEmailsAllowed>
            ${msg("emailInstructionUsername")}
        <#else>
            ${msg("emailInstruction")}
        </#if>
    </#if>
</@layout.registrationLayout>
