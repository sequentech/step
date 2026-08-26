<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<#assign structuredCredential = (realm.attributes['credential-input-policy']!'standard') == 'structured'>
<#assign credentialFieldError = messagesPerField.existsError('username','password')>
<#assign structuredCredentialHasError = structuredCredential && credentialFieldError>
<@layout.registrationLayout displayMessage=!credentialFieldError displayInfo=realm.password && realm.registrationAllowed && !registrationDisabled?? displaySocialProviders=social.providers?has_content; section>
    <#if section = "header">
        ${msg("loginAccountTitle")}
    <#elseif section = "form">
        <#--  The login page is not rendered from the user profile, so the username policy is a
              realm attribute rather than the loginHintPrefillPolicy attribute annotation used by
              the registration forms. A remembered username stays editable so the voter can still
              sign in as somebody else.  -->
        <#assign usernamePrefilled = (login.username!'')?has_content && !login.rememberMe??>
        <#assign usernameReadOnly = usernamePrefilled
            && (realm.attributes['loginHintUsernamePolicy']!'EDITABLE') == 'READ_ONLY'>
        <#-- Whether Tab order should visit the credential field before the identity/matchAttributes
              fields. The DOM position of both blocks below is unchanged for every realm — a realm
              opting into this only gets explicit tabindex values on the two fields that need
              reordering (and nothing else), so Tab order can match a realm's own visual CSS
              (e.g. a loginCustomCss override) without moving markup that other realms, and any of
              their own CSS/JS assumptions about element order, still depend on unchanged. -->
        <#assign credentialFirst = (realm.attributes['login-field-order']!'identity-first') == 'credential-first'>
        <div id="kc-form">
          <div id="kc-form-wrapper">
            <#if realm.password>
                <form id="kc-form-login" <#if !structuredCredential>onsubmit="login.disabled = true; return true;"</#if> action="${url.loginAction}" method="post">
                    <#if !usernameHidden??>
                        <#if matchAttributes?? && matchAttributes?has_content>
                            <#if matchAttributes?filter(f -> f.type == "tel")?has_content>
                                <#include "intl-tel-input.ftl">
                            </#if>
                            <#list matchAttributes as field>
                                <div class="${properties.kcFormGroupClass!}">
                                    <label for="${field.name}" class="${properties.kcLabelClass!}">${msg(field.name)}</label>

                                    <#if field.type == "tel">
                                        <@renderIntlTelInput id=field.name name=field.name autofocus=(!credentialFirst && field?index == 0)/>
                                    <#else>
                                        <input <#if credentialFirst>tabindex="${field?index + 3}"</#if> id="${field.name}" class="${properties.kcInputClass!}" name="${field.name}" type="${field.type!'text'}"
                                               <#-- see user-profile-commons.ftl: date inputs accept 5+ digit years -->
                                               <#if (field.type!'') == 'date'>max="${field.max!'9999-12-31'}"</#if>
                                               <#if !credentialFirst && field?index == 0>autofocus</#if> autocomplete="off"
                                               <#if credentialFieldError>aria-invalid="true"</#if>
                                        />
                                    </#if>
                                </div>
                            </#list>

                            <#if credentialFieldError && !structuredCredential>
                                <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                        ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                </span>
                            </#if>
                        <#else>
                            <div class="${properties.kcFormGroupClass!}">
                                <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>

                                <#-- readonly rather than disabled so the locked value is still submitted -->
                                <input <#if credentialFirst>tabindex="3"</#if> id="username" class="${properties.kcInputClass!}" name="username" value="${(login.username!'')}" type="text" <#if !credentialFirst>autofocus</#if> autocomplete="<#if structuredCredential>username<#else>off</#if>"
                                       <#if usernameReadOnly>readonly</#if>
                                       <#if credentialFieldError>aria-invalid="true"</#if>
                                />

                                <#if credentialFieldError && !structuredCredential>
                                    <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                            ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                    </span>
                                </#if>

                            </div>
                        </#if>
                    </#if>

                    <div class="${properties.kcFormGroupClass!}">
                        <label id="structured-credential-label" for="password" class="${properties.kcLabelClass!}"><#if structuredCredential>${msg("structuredCredentialLabel")}<#else>${msg("password")}</#if></label>

                        <div class="${properties.kcInputGroup!}"<#if structuredCredential>
                             data-structured-credential
                             data-credential-pattern="${realm.attributes['credential-input-pattern']!'dddd-dddd-dddd-dddd'}"
                             data-group-status="${msg('structuredCredentialGroupStatus')}"
                             data-paste-error="${msg('structuredCredentialPasteError')}"
                             data-format-error="${msg('structuredCredentialFormatError')}"
                             data-label-id="structured-credential-label"
                             data-hint-id="structured-credential-hint"
                             data-error-id="structured-credential-error"</#if>>
                            <input <#if credentialFirst>tabindex="1"</#if> id="password" class="${properties.kcInputClass!}" name="password" type="password"
                                   <#if credentialFirst>autofocus</#if>
                                   autocomplete="<#if structuredCredential>current-password<#else>off</#if>"
                                   <#if structuredCredential>inputmode="numeric"</#if>
                                   <#if structuredCredential>aria-describedby="structured-credential-hint structured-credential-error"</#if>
                                   <#if structuredCredentialHasError || credentialFieldError>aria-invalid="true"</#if>
                            />
                            <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="<#if structuredCredential>${msg('showStructuredCredential')}<#else>${msg('showPassword')}</#if>"
                                    aria-controls="password" <#if structuredCredential>data-structured-credential-toggle<#else>data-password-toggle</#if> <#if credentialFirst>tabindex="2"</#if>
                                    data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                                    data-label-show="<#if structuredCredential>${msg('showStructuredCredential')}<#else>${msg('showPassword')}</#if>"
                                    data-label-hide="<#if structuredCredential>${msg('hideStructuredCredential')}<#else>${msg('hidePassword')}</#if>">
                                <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                            </button>
                        </div>

                        <#if structuredCredential>
                            <div id="structured-credential-hint" class="structured-credential__hint">${msg("structuredCredentialHint")}</div>
                            <span id="structured-credential-error" data-structured-credential-error class="${properties.kcInputErrorMessageClass!}" role="alert"<#if !structuredCredentialHasError> hidden</#if>>
                                ${msg("structuredCredentialError")}
                            </span>
                        <#elseif usernameHidden?? && credentialFieldError>
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
                                            <input id="rememberMe" name="rememberMe" type="checkbox" checked> ${msg("rememberMe")}
                                        <#else>
                                            <input id="rememberMe" name="rememberMe" type="checkbox"> ${msg("rememberMe")}
                                        </#if>
                                    </label>
                                </div>
                            </#if>
                            </div>
                            <div class="${properties.kcFormOptionsWrapperClass!}">
                                <#if realm.resetPasswordAllowed>
                                    <span><a href="${url.loginResetCredentialsUrl}">${msg("doForgotPassword")}</a></span>
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
        <#if structuredCredential>
            <script type="module" src="${url.resourcesPath}/js/structured-credential.js"></script>
        <#else>
            <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>
        </#if>
    <#elseif section = "info" >
        <#if realm.password && realm.registrationAllowed && !registrationDisabled??>
            <div id="kc-registration-container">
                <div id="kc-registration">
                    <span>${msg("noAccount")} <a href="${url.registrationUrl}">${msg("doRegister")}</a></span>
                </div>
            </div>
        </#if>
    <#elseif section = "socialProviders" >
        <#assign visibleProviders = social.providers?filter(p -> p.alias != 'digital-certificates' || (realm.attributes['voter-certificate-policy']!'disabled') == 'enabled')>
        <#if realm.password && visibleProviders?has_content>
            <hr/>
            <h4 style="text-align: center;">${msg("identity-provider-login-label")}</h4>
            <div id="kc-social-providers" class="${properties.kcFormSocialAccountSectionClass!}">
                <#list visibleProviders as p>
                    <ul class="${properties.kcFormSocialAccountListClass!} <#if visibleProviders?size gt 3>${properties.kcFormSocialAccountListGridClass!}</#if>">
                        <li>
                            <a id="social-${p.alias}" class="${properties.kcFormSocialAccountListButtonClass!} <#if visibleProviders?size gt 3>${properties.kcFormSocialAccountGridItem!}</#if>"
                                    type="button" href="${p.loginUrl}">
                                <#if p.iconClasses?has_content>
                                    <i class="${properties.kcCommonLogoIdP!} ${p.iconClasses!}" aria-hidden="true"></i>
                                    <span class="${properties.kcFormSocialAccountNameClass!} kc-social-icon-text"><#if p.alias == 'digital-certificates'>${msg("digitalCertificateButton")}<#else>${msg(p.displayName)!}</#if></span>
                                <#else>
                                    <span class="${properties.kcFormSocialAccountNameClass!}"><#if p.alias == 'digital-certificates'>${msg("digitalCertificateButton")}<#else>${msg(p.displayName)!}</#if></span>
                                </#if>
                            </a>
                        </li>
                    </ul>
                </#list>
            </div>
        </#if>
    </#if>

</@layout.registrationLayout>
