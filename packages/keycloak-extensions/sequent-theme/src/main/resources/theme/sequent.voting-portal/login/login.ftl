<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<#import "match-attributes.ftl" as matchAttributeFields>
<#import "social-providers.ftl" as socialProviders>
<#assign structuredCredential = ['structured', 'pattern']?seq_contains(realm.attributes['credential-input-policy']!'standard')>
<#--  SERVER_ONLY adds novalidate, so the browser stops adjudicating field formats and the
      authenticator is the only judge of a login attempt. The constraint attributes stay in the
      DOM: required still maps to aria-required, maxlength still caps typing, and max still bounds
      the date picker - novalidate suppresses only the interactive pass on submit.  -->
<#assign loginValidationServerOnly = (realm.attributes['login-validation-policy']!'BROWSER') == 'SERVER_ONLY'>
<#assign credentialFirst = (realm.attributes['credential-field-position']!'LAST') == 'FIRST'
    && matchAttributes?? && matchAttributes?has_content>
<#assign credentialFieldError = messagesPerField.existsError('username','password')>
<#assign structuredCredentialHasError = structuredCredential && credentialFieldError>
<#assign honorRequired = honorUserProfileRequired??>
<#--  Required markers only earn their place when some field is optional: the credential is
      always mandatory, so marking everything would tell the voter nothing. An attribute with no
      User Profile declaration stays mandatory in the authenticator (see
      MultiAttributePasswordAuthenticator#optionalAttributes), so it is never the optional one.  -->
<#assign showRequiredMarkers = matchAttributeFields.showRequiredMarkers(
    matchAttributes![], profile!{}, honorRequired)>
<#--  The credential block is invoked in one of two positions, chosen by the realm's
      credential-field-position attribute. Kept as a local macro rather than a shared one: the two
      portals' credential markup differs, and only this one supports the structured PIN.  -->
<#macro credentialField autofocus=false>
                    <div class="${properties.kcFormGroupClass!}">
                        <label id="structured-credential-label" for="password" class="${properties.kcLabelClass!}"><#if structuredCredential>${msg("structuredCredentialLabel")}<#else>${msg("password")}</#if></label><#if showRequiredMarkers> *</#if>

                        <div class="${properties.kcInputGroup!}"<#if structuredCredential>
                             data-structured-credential
                             data-credential-pattern="${realm.attributes['credential-input-pattern']!'dddd-dddd-dddd-dddd'}"
                             data-credential-input-placeholder="${realm.attributes['credential-input-placeholder']!'d'}"
                             data-group-status="${msg('structuredCredentialGroupStatus')}"
                             data-paste-error="${msg('structuredCredentialPasteError')}"
                             data-format-error="${msg('structuredCredentialFormatError')}"
                             data-label-id="structured-credential-label"
                             data-hint-id="structured-credential-hint"
                             data-error-id="structured-credential-error"</#if>>
                            <input id="password"<#if autofocus> autofocus</#if> class="${properties.kcInputClass!}" name="password" type="password"
                                   autocomplete="<#if structuredCredential>current-password<#else>off</#if>"
                                   <#if structuredCredential>inputmode="numeric"</#if>
                                   <#if structuredCredential>aria-describedby="structured-credential-hint structured-credential-error"</#if>
                                   <#if structuredCredentialHasError || credentialFieldError>aria-invalid="true"</#if>
                            />
                            <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="<#if structuredCredential>${msg('showStructuredCredential')}<#else>${msg('showPassword')}</#if>"
                                    aria-controls="password" <#if structuredCredential>data-structured-credential-toggle<#else>data-password-toggle</#if>
                                    data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                                    data-label-show="<#if structuredCredential>${msg('showStructuredCredential')}<#else>${msg('showPassword')}</#if>"
                                    data-label-hide="<#if structuredCredential>${msg('hideStructuredCredential')}<#else>${msg('hidePassword')}</#if>">
                                <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                            </button>
                        </div>

                        <#if structuredCredential>
                            <div id="structured-credential-hint" class="structured-credential__hint">${msg("structuredCredentialHint")}</div>
                            <span id="structured-credential-error" data-structured-credential-error class="${properties.kcInputErrorMessageClass!}" role="alert"<#if !structuredCredentialHasError> hidden</#if>>
                                <#if structuredCredentialHasError && matchAttributes?? && matchAttributes?has_content>
                                    ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                <#else>
                                    ${msg("structuredCredentialError")}
                                </#if>
                            </span>
                        <#elseif usernameHidden?? && credentialFieldError>
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
        <#--  The login page is not rendered from the user profile, so the username policy is a
              realm attribute rather than the loginHintPrefillPolicy attribute annotation used by
              the registration forms. A remembered username stays editable so the voter can still
              sign in as somebody else.  -->
        <#assign usernamePrefilled = (login.username!'')?has_content && !login.rememberMe??>
        <#assign usernameReadOnly = usernamePrefilled
            && (realm.attributes['loginHintUsernamePolicy']!'EDITABLE') == 'READ_ONLY'>
        <div id="kc-form">
          <div id="kc-form-wrapper">
            <#if realm.password>
                <form id="kc-form-login" <#if !structuredCredential>onsubmit="login.disabled = true; return true;"</#if><#if loginValidationServerOnly> novalidate</#if> action="${url.loginAction}" method="post">
                    <#if !usernameHidden??>
                        <#if matchAttributes?? && matchAttributes?has_content>
                            <@matchAttributeFields.render
                                matchAttributes=matchAttributes profile=profile
                                honorRequired=honorRequired showRequiredMarkers=showRequiredMarkers
                                credentialFirst=credentialFirst fieldError=credentialFieldError>
                                <#if credentialFirst><@credentialField autofocus=true/></#if>
                            </@matchAttributeFields.render>

                            <#if credentialFieldError && !structuredCredential>
                                <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                        ${kcSanitize(messagesPerField.getFirstError('username','password'))?no_esc}
                                </span>
                            </#if>
                        <#else>
                            <div class="${properties.kcFormGroupClass!}">
                                <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>

                                <#-- readonly rather than disabled so the locked value is still submitted -->
                                <input id="username" class="${properties.kcInputClass!}" name="username" value="${(login.username!'')}" type="text" autofocus autocomplete="<#if structuredCredential>username<#else>off</#if>"
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
        <@socialProviders.render/>
    </#if>

</@layout.registrationLayout>
