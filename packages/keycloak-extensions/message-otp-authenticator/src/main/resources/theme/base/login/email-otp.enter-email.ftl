<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
    <#if section = "header">
        <title>${msg("emailOtp.auth.enterEmailTitle")}</title>
        <h2>${msg("emailOtp.auth.enterEmailTitle")}</h2>
    <#elseif section = "form">
        <#if error??>
            <div class="${properties.kcInputErrorMessageClass!}" aria-live="polite">${error}</div>
        </#if>
        <form id="kc-email-otp-form" class="${properties.kcFormClass!}" method="post" action="${url.loginAction}">
            <div class="${properties.kcFormGroupClass!}">
                <div class="${properties.kcLabelWrapperClass!}">
                    <label for="email" class="${properties.kcLabelClass!}">${msg("emailOtp.auth.enterEmailLabel")}</label> *
                </div>
                <div class="${properties.kcInputWrapperClass!}">
                    <input type="email" id="email" name="email" class="${properties.kcInputClass!}" required autofocus />
                    <div class="help-message ${properties.kcInputHelperTextAfterClass!}" style="margin: 8px 0 16px 0; color: #555;">
                        ${msg("emailOtp.auth.enterEmailHelp")}
                    </div>
                </div>
            </div>
            <div class="${properties.kcFormGroupClass!}">
                <div id="kc-form-buttons" class="${properties.kcFormButtonsClass!}">
                    <button class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}" type="submit">
                        ${msg("emailOtp.auth.sendCodeButton")}
                    </button>
                </div>
            </div>
        </form>
    </#if>
</@layout.registrationLayout>
