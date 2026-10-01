<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<#import "match-attributes.ftl" as matchAttributeFields>
<#import "social-providers.ftl" as socialProviders>
<#--  SERVER_ONLY adds novalidate, so the browser stops adjudicating field formats and the
      authenticator is the only judge of a login attempt. The constraint attributes stay in the
      DOM: required still maps to aria-required, maxlength still caps typing, and max still bounds
      the date picker - novalidate suppresses only the interactive pass on submit.  -->
<#assign loginValidationServerOnly = (realm.attributes['login-validation-policy']!'BROWSER') == 'SERVER_ONLY'>
<#assign credentialFirst = (realm.attributes['credential-field-position']!'LAST') == 'FIRST'
    && matchAttributes?? && matchAttributes?has_content>
<#assign credentialFieldError = messagesPerField.existsError('username','password')>
<#assign honorRequired = honorUserProfileRequired??>
<#--  Required markers only earn their place when some field is optional: the credential is
      always mandatory, so marking everything would tell the voter nothing. An attribute with no
      User Profile declaration stays mandatory in the authenticator (see
      MultiAttributePasswordAuthenticator#optionalAttributes), so it is never the optional one.  -->
<#assign showRequiredMarkers = matchAttributeFields.showRequiredMarkers(
    matchAttributes![], profile!{}, honorRequired)>
<#--  The credential block is invoked in one of two positions, chosen by the realm's
      credential-field-position attribute. Kept as a local macro rather than a shared one: the two
      portals' credential markup differs, and only the voting portal supports the structured PIN.  -->
<#macro credentialField autofocus=false>
                    <div class="${properties.kcFormGroupClass!}">
                        <label for="password" class="${properties.kcLabelClass!}">${msg("password")}</label><#if showRequiredMarkers> *</#if>

                        <div class="${properties.kcInputGroup!}">
                            <input id="password"<#if autofocus> autofocus</#if> class="${properties.kcInputClass!}" name="password" type="password"
                                    autocomplete="off"
                                   aria-invalid="<#if messagesPerField.existsError('username','password')>true</#if>"
                            />
                            <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="${msg("showPassword")}"
                                    aria-controls="password" data-password-toggle
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
</#macro>

<@layout.registrationLayout displayMessage=!credentialFieldError displayInfo=realm.password && realm.registrationAllowed && !registrationDisabled?? displayRequiredFields=showRequiredMarkers displaySocialProviders=social.providers?has_content; section>
    <#if section = "header">
        ${msg("loginAccountTitle")}
    <#elseif section = "form">
        <div id="kc-form">
          <div id="kc-form-wrapper">
            <#if realm.password>
                <form id="kc-form-login" onsubmit="login.disabled = true; return true;"<#if loginValidationServerOnly> novalidate</#if> action="${url.loginAction}" method="post">
                    <#if !usernameHidden??>
                        <#if matchAttributes?? && matchAttributes?has_content>
                            <@matchAttributeFields.render
                                matchAttributes=matchAttributes profile=profile
                                honorRequired=honorRequired showRequiredMarkers=showRequiredMarkers
                                credentialFirst=credentialFirst fieldError=credentialFieldError>
                                <#if credentialFirst><@credentialField autofocus=true/></#if>
                            </@matchAttributeFields.render>

                            <#if credentialFieldError>
                                <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                        ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                </span>
                            </#if>
                        <#else>
                            <div class="${properties.kcFormGroupClass!}">
                                <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>

                                <input id="username" class="${properties.kcInputClass!}" name="username" type="text" autofocus autocomplete="off"
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

                    <#if !credentialFirst><@credentialField/></#if>

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
        <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>
    <#elseif section = "info" >
        <#if realm.password && realm.registrationAllowed && !registrationDisabled??>
            <div id="kc-registration-container">
                <div id="kc-registration">
                    <span>${msg("noAccount")} <a href="${url.registrationUrl}">${msg("doRegister")}</a></span>
                </div>
            </div>
        </#if>
    <#elseif section = "socialProviders" >
        <@socialProviders.render/>
    </#if>

</@layout.registrationLayout>
