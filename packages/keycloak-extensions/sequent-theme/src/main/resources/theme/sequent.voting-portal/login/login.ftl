<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<#assign segmentedCredential = (realm.attributes['credential-input-policy']!'standard') == 'segmented-numeric'>
<#assign credentialFieldError = messagesPerField.existsError('username','password')>
<#assign segmentedCredentialHasError = segmentedCredential && credentialFieldError>
<@layout.registrationLayout displayMessage=!credentialFieldError displayInfo=realm.password && realm.registrationAllowed && !registrationDisabled?? displaySocialProviders=social.providers?has_content; section>
    <#if section = "header">
        ${msg("loginAccountTitle")}
    <#elseif section = "form">
        <div id="kc-form">
          <div id="kc-form-wrapper">
            <#if realm.password>
                <form id="kc-form-login" <#if !segmentedCredential>onsubmit="login.disabled = true; return true;"</#if> action="${url.loginAction}" method="post">
                    <#if !usernameHidden??>
                        <div class="${properties.kcFormGroupClass!}">
                            <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>

                            <input tabindex="1" id="username" class="${properties.kcInputClass!}" name="username" type="text" autofocus autocomplete="off"
                                   aria-invalid="<#if segmentedCredentialHasError || credentialFieldError>true</#if>"
                            />

                            <#if credentialFieldError && !segmentedCredential>
                                <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                        ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                </span>
                            </#if>

                        </div>
                    </#if>

                    <div class="${properties.kcFormGroupClass!}">
                        <label id="segmented-credential-label" for="password" class="${properties.kcLabelClass!}"><#if segmentedCredential>${msg("segmentedCredentialLabel")}<#else>${msg("password")}</#if></label>

                        <div class="${properties.kcInputGroup!}"<#if segmentedCredential>
                             data-segmented-credential
                             data-segment-layout="${(realm.attributes['credential-segment-layout']!'4-4-4-4')?html}"
                             data-group-label="${msg('segmentedCredentialGroupLabel')?html}"
                             data-label-id="segmented-credential-label"
                             data-hint-id="segmented-credential-hint"
                             data-error-id="segmented-credential-error"</#if>>
                            <input tabindex="3" id="password" class="${properties.kcInputClass!}" name="password" type="password"
                                   autocomplete="off"
                                   <#if segmentedCredential>aria-describedby="segmented-credential-hint segmented-credential-error"</#if>
                                   aria-invalid="<#if segmentedCredentialHasError || credentialFieldError>true</#if>"
                            />
                            <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="<#if segmentedCredential>${msg('showSegmentedCredential')}<#else>${msg('showPassword')}</#if>"
                                    aria-controls="password" <#if segmentedCredential>data-segmented-credential-toggle<#else>data-password-toggle</#if> tabindex="4"
                                    data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                                    data-label-show="<#if segmentedCredential>${msg('showSegmentedCredential')}<#else>${msg('showPassword')}</#if>"
                                    data-label-hide="<#if segmentedCredential>${msg('hideSegmentedCredential')}<#else>${msg('hidePassword')}</#if>">
                                <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                            </button>
                        </div>

                        <#if segmentedCredential>
                            <div id="segmented-credential-hint" class="segmented-credential__hint">${msg("segmentedCredentialHint")}</div>
                            <span id="segmented-credential-error" data-segmented-credential-error class="${properties.kcInputErrorMessageClass!}" role="alert"<#if !segmentedCredentialHasError> hidden</#if>>
                                ${msg("segmentedCredentialError")}
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
        <#if segmentedCredential>
            <script type="module" src="${url.resourcesPath}/js/segmented-credential.js"></script>
        <#else>
            <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>
        </#if>
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
