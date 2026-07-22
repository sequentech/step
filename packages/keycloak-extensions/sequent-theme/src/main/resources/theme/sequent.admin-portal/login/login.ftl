<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=!messagesPerField.existsError('username','password') displayInfo=realm.password && realm.registrationAllowed && !registrationDisabled?? displaySocialProviders=social.providers?has_content; section>
    <#if section = "header">
        ${msg("loginAccountTitle")}
    <#elseif section = "form">
        <div id="kc-form">
          <div id="kc-form-wrapper">
            <#if realm.password>
                <form id="kc-form-login" onsubmit="login.disabled = true; return true;" action="${url.loginAction}" method="post">
                    <#-- Number of fields rendered ahead of the password field -->
                    <#assign fieldCount = (matchAttributes?? && matchAttributes?has_content)?then(matchAttributes?size, 1)>
                    <#if !usernameHidden??>
                        <#if matchAttributes?? && matchAttributes?has_content>
                            <#if matchAttributes?filter(f -> f.type == "tel")?has_content>
                                <#include "intl-tel-input.ftl">
                            </#if>
                            <#list matchAttributes as field>
                                <div class="${properties.kcFormGroupClass!}">
                                    <label for="${field.name}" class="${properties.kcLabelClass!}">${msg(field.name)}</label>

                                    <#if field.type == "tel">
                                        <@renderIntlTelInput id=field.name name=field.name autofocus=(field?index == 0)/>
                                    <#else>
                                        <input tabindex="${field?index + 1}" id="${field.name}" class="${properties.kcInputClass!}" name="${field.name}" type="${field.type!'text'}"
                                               <#if field?index == 0>autofocus</#if> autocomplete="off"
                                               aria-invalid="<#if messagesPerField.existsError('username','password')>true</#if>"
                                        />
                                    </#if>
                                </div>
                            </#list>

                            <#if messagesPerField.existsError('username','password')>
                                <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                        ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                </span>
                            </#if>
                        <#else>
                            <div class="${properties.kcFormGroupClass!}">
                                <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>

                                <input tabindex="1" id="username" class="${properties.kcInputClass!}" name="username" type="text" autofocus autocomplete="off"
                                       aria-invalid="<#if messagesPerField.existsError('username','password')>true</#if>"
                                />

                                <#if messagesPerField.existsError('username','password')>
                                    <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                            ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                    </span>
                                </#if>

                            </div>
                        </#if>
                    </#if>

                    <div class="${properties.kcFormGroupClass!}">
                        <label for="password" class="${properties.kcLabelClass!}">${msg("password")}</label>

                        <div class="${properties.kcInputGroup!}">
                            <input tabindex="${fieldCount + 2}" id="password" class="${properties.kcInputClass!}" name="password" type="password"
                                    autocomplete="off"
                                   aria-invalid="<#if messagesPerField.existsError('username','password')>true</#if>"
                            />
                            <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="${msg("showPassword")}"
                                    aria-controls="password" data-password-toggle tabindex="${fieldCount + 3}"
                                    data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                                    data-label-show="${msg('showPassword')}" data-label-hide="${msg('hidePassword')}">
                                <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                            </button>
                        </div>

                        <#if usernameHidden?? && messagesPerField.existsError('username','password')>
                            <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                    ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                            </span>
                        </#if>

                    </div>

                    <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
                        <div id="kc-form-options">
                            <#if realm.rememberMe && !usernameHidden??>
                                <div class="checkbox">
                                    <label>
                                        <#if login.rememberMe??>
                                            <input tabindex="3" id="rememberMe" name="rememberMe" type="checkbox" checked> ${msg("rememberMe")}
                                        <#else>
                                            <input tabindex="3" id="rememberMe" name="rememberMe" type="checkbox"> ${msg("rememberMe")}
                                        </#if>
                                    </label>
                                </div>
                            </#if>
                            </div>
                            <div class="${properties.kcFormOptionsWrapperClass!}">
                                <#if realm.resetPasswordAllowed>
                                    <span><a tabindex="5" href="${url.loginResetCredentialsUrl}">${msg("doForgotPassword")}</a></span>
                                </#if>
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

                      <div id="kc-form-buttons" class="${properties.kcFormGroupClass!}">
                          <input type="hidden" id="id-hidden-input" name="credentialId" <#if auth.selectedCredential?has_content>value="${auth.selectedCredential}"</#if>/>
                          <input
                            tabindex="4" 
                            class="g-recaptcha ${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                            name="login"
                            id="kc-login"
                            value="${msg("doLogIn")}"
                            type="submit"
                        />
                      </div>
                </form>
            </#if>
            </div>
        </div>
        <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>
    <#elseif section = "info" >
        <#if realm.password && realm.registrationAllowed && !registrationDisabled??>
            <div id="kc-registration-container">
                <div id="kc-registration">
                    <span>${msg("noAccount")} <a tabindex="6"
                                                 href="${url.registrationUrl}">${msg("doRegister")}</a></span>
                </div>
            </div>
        </#if>
    <#elseif section = "socialProviders" >
        <#if realm.password && social.providers??>
            <div id="kc-social-providers" class="${properties.kcFormSocialAccountSectionClass!}">
                <hr/>
                <h4>${msg("identity-provider-login-label")}</h4>

                <ul class="${properties.kcFormSocialAccountListClass!} <#if social.providers?size gt 3>${properties.kcFormSocialAccountListGridClass!}</#if>">
                    <#list social.providers as p>
                        <li>
                            <a id="social-${p.alias}" class="${properties.kcFormSocialAccountListButtonClass!} <#if social.providers?size gt 3>${properties.kcFormSocialAccountGridItem!}</#if>"
                                    type="button" href="${p.loginUrl}">
                                <#if p.iconClasses?has_content>
                                    <i class="${properties.kcCommonLogoIdP!} ${p.iconClasses!}" aria-hidden="true"></i>
                                    <span class="${properties.kcFormSocialAccountNameClass!} kc-social-icon-text">${p.displayName!}</span>
                                <#else>
                                    <span class="${properties.kcFormSocialAccountNameClass!}">${p.displayName!}</span>
                                </#if>
                            </a>
                        </li>
                    </#list>
                </ul>
            </div>
        </#if>
    </#if>

</@layout.registrationLayout>
