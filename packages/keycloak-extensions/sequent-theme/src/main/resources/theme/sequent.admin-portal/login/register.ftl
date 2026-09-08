<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Source: https://github.com/keycloak/keycloak/blob/24.0.0/themes/src/main/resources/theme/base/login/register.ftl  -->

<#import "template.ftl" as layout>
<#import "user-profile-commons.ftl" as userProfileCommons>
<#import "register-commons.ftl" as registerCommons>
<#import "field-helper-text.ftl" as fieldHelperText>
<#import "tel-input-widget.ftl" as telInputWidget>
<#import "select-filter-widget.ftl" as selectFilterWidget>
<#import "social-providers.ftl" as socialProviders>
<#assign loginMode = formMode?? && formMode == 'LOGIN'>
<#assign passwordRequired = passwordRequired!false>
<#--  An attribute that explicitly declares showPasswordAfterThis keeps its placement: that
      annotation always wins over credential-field-position, so realms configured before the
      setting existed render exactly as they did.  -->
<#assign passwordAttributeName = 'username'>
<#if realm.registrationEmailAsUsername>
    <#assign passwordAttributeName = 'email'>
</#if>
<#assign passwordProfileAttributes = profile.attributes>
<#if hiddenProfileAttributes??>
    <#assign passwordProfileAttributes = profile.attributes?filter(a -> !hiddenProfileAttributes?seq_contains(a.name))>
</#if>
<#assign defaultPasswordAttributes = passwordProfileAttributes?filter(a -> a.name == passwordAttributeName)>
<#assign explicitPasswordAttributes = passwordProfileAttributes?filter(a -> (a.annotations.showPasswordAfterThis!'false') == 'true')>
<#if explicitPasswordAttributes?has_content>
    <#assign explicitPasswordAttribute = explicitPasswordAttributes?first>
</#if>
<#assign explicitPasswordAnchor = passwordProfileAttributes?filter(a -> a.annotations.showPasswordAfterThis??)?has_content>
<#assign passwordAttribute = {'name': 'password', 'annotations': {}}>
<#if passwordProfileAttributes?has_content>
    <#assign passwordAttribute = passwordProfileAttributes?first>
</#if>
<#if defaultPasswordAttributes?has_content>
    <#assign passwordAttribute = defaultPasswordAttributes?first>
</#if>
<#assign passwordAnchorName = ''>
<#if explicitPasswordAttributes?has_content>
    <#assign passwordAttribute = explicitPasswordAttribute>
    <#assign passwordAnchorName = explicitPasswordAttribute.name>
<#elseif defaultPasswordAttributes?has_content && (passwordAttribute.annotations.showPasswordAfterThis!'true') != 'false'>
    <#assign passwordAnchorName = passwordAttribute.name>
</#if>
<#assign credentialFirst = passwordRequired
    && (realm.attributes['credential-field-position']!'LAST') == 'FIRST'
    && !explicitPasswordAnchor>
<#assign structuredCredentialLogin = loginMode && passwordRequired && ['structured', 'pattern']?seq_contains(realm.attributes['credential-input-policy']!'standard')>
<#assign credentialFieldError = messagesPerField.existsError('username','password')>
<#assign structuredCredentialHasError = structuredCredentialLogin && credentialFieldError>
<#macro credentialFields>
    <div class="${properties.kcFormGroupClass!}">
        <div class="${properties.kcLabelWrapperClass!}">
            <label id="structured-credential-label" for="password" class="${properties.kcLabelClass!}"><#if structuredCredentialLogin>${msg("structuredCredentialLabel")}<#else>${msg("password")}</#if></label> *
        </div>
        <div class="${properties.kcInputWrapperClass!}">
            <#--  You can add a custom passwordHelperTextBefore to either username or email depending on realm.registrationEmailAsUsername settings to add a helpertext -->
            <#if passwordAttribute.annotations.passwordHelperTextBefore??>
                <@fieldHelperText.helperTextBefore id=passwordAttribute.name text=passwordAttribute.annotations.passwordHelperTextBefore/>
            </#if>

            <div class="${properties.kcInputGroup!}"<#if structuredCredentialLogin>
                 data-structured-credential
                 data-credential-pattern="${realm.attributes['credential-input-pattern']!'dddd-dddd-dddd-dddd'}"
                 data-credential-input-placeholder="${realm.attributes['credential-input-placeholder']!'d'}"
                 data-group-status="${msg('structuredCredentialGroupStatus')}"
                 data-paste-error="${msg('structuredCredentialPasteError')}"
                 data-format-error="${msg('structuredCredentialFormatError')}"
                 data-label-id="structured-credential-label"
                 data-hint-id="structured-credential-hint"
                 data-error-id="structured-credential-error"</#if>>
                <input type="password" id="password" class="${properties.kcInputClass!}" name="password"
                       <#if credentialFirst>autofocus</#if>
                       <#if structuredCredentialLogin>autocomplete="current-password"<#else>autocomplete="new-password"</#if>
                       <#if structuredCredentialLogin>inputmode="numeric"</#if>
                       <#if structuredCredentialLogin>aria-describedby="structured-credential-hint structured-credential-error"</#if>
                       <#if structuredCredentialHasError || messagesPerField.existsError('password','password-confirm')>aria-invalid="true"</#if>
                />
                <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="<#if structuredCredentialLogin>${msg('showStructuredCredential')}<#else>${msg('showPassword')}</#if>"
                        aria-controls="password" <#if structuredCredentialLogin>data-structured-credential-toggle<#else>data-password-toggle</#if>
                        data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                        data-label-show="<#if structuredCredentialLogin>${msg('showStructuredCredential')}<#else>${msg('showPassword')}</#if>"
                        data-label-hide="<#if structuredCredentialLogin>${msg('hideStructuredCredential')}<#else>${msg('hidePassword')}</#if>">
                    <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                </button>
            </div>

            <#if structuredCredentialLogin>
                <div id="structured-credential-hint" class="structured-credential__hint">${msg("structuredCredentialHint")}</div>
                <span id="structured-credential-error" data-structured-credential-error class="${properties.kcInputErrorMessageClass!}" role="alert"<#if !structuredCredentialHasError> hidden</#if>>
                    ${msg("structuredCredentialError")}
                </span>
            </#if>

            <#--  You can add a password strength bar if passwordStrengthBar is set to either username or email depending on realm.registrationEmailAsUsername settings to add a strength bar -->
            <#if passwordAttribute.annotations.passwordStrengthBar?? && formMode?? && (formMode!"REGISTRATION") != "LOGIN">
                <div class="pf-c-progress pf-m-sm" id="password-progress">
                    <div class="pf-c-progress__bar" id="password-progress-aria" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0" aria-labelledby="password-progress">
                        <div class="pf-c-progress__indicator" id="password-progress-indicator"></div>
                    </div>
                </div>
            </#if>

            <#if messagesPerField.existsError('password') && !structuredCredentialLogin>
                <span id="input-error-password" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                    ${kcSanitize(messagesPerField.get('password'))?no_esc}
                </span>
            </#if>

            <#--  You can add a custom passwordHelperTextAfter to either username or email depending on realm.registrationEmailAsUsername settings to add a helpertext -->
            <#if passwordAttribute.annotations.passwordHelperTextAfter??>
                <@fieldHelperText.helperTextAfter id=passwordAttribute.name text=passwordAttribute.annotations.passwordHelperTextAfter/>
            </#if>
        </div>
    </div>

    <#if formMode?? && (formMode!"REGISTRATION") != "LOGIN">
        <div class="${properties.kcFormGroupClass!}">
            <div class="${properties.kcLabelWrapperClass!}">
                <label for="password-confirm"
                    class="${properties.kcLabelClass!}">${msg("passwordConfirm")}</label> *
            </div>
            <div class="${properties.kcInputWrapperClass!}">
                <div class="${properties.kcInputGroup!}">
                    <input type="password" id="password-confirm" class="${properties.kcInputClass!}"
                        name="password-confirm"
                        aria-invalid="<#if messagesPerField.existsError('password-confirm')>true</#if>"
                    />
                    <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="${msg('showPassword')}"
                            aria-controls="password-confirm"  data-password-toggle
                            data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                            data-label-show="${msg('showPassword')}" data-label-hide="${msg('hidePassword')}">
                        <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                    </button>
                </div>

                <#if messagesPerField.existsError('password-confirm')>
                    <span id="input-error-password-confirm" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                    ${kcSanitize(messagesPerField.get('password-confirm'))?no_esc}
                </span>
                </#if>
            </div>
        </div>
    </#if>
</#macro>
<@layout.registrationLayout displayMessage=messagesPerField.exists('global') displayRequiredFields=true displaySocialProviders=(formMode?? && formMode = 'LOGIN' && (social.providers)?has_content); section>
    <#if section = "header">
        <#if formMode?? && formMode = 'LOGIN'>
            ${msg('loginTitle',(realm.displayName!''))}
        <#else>
            ${msg('registerTitle')}
        </#if>
    <#elseif section = "form">
        <form id="kc-register-form" class="${properties.kcFormClass!}" action="${url.registrationAction}" method="post">

            <#if credentialFirst>
                <@credentialFields/>
            </#if>
            <@userProfileCommons.userProfileFormFields; callback, attribute>
                <#if callback = "afterField" && passwordRequired && !credentialFirst && attribute.name == passwordAnchorName>
                    <@credentialFields/>
                </#if>
            </@userProfileCommons.userProfileFormFields>

            <@registerCommons.termsAcceptance/>

            <#if recaptchaRequired??>
                <div class="form-group">
                    <div class="${properties.kcInputWrapperClass!}">
                        <div class="g-recaptcha" data-size="compact" data-sitekey="${recaptchaSiteKey}"></div>
                    </div>
                </div>
            </#if>

            <div class="${properties.kcFormGroupClass!}">
                <#if formMode?? && (formMode!"REGISTRATION") != "LOGIN">
                    <div id="kc-form-options" class="${properties.kcFormOptionsClass!}">
                        <div class="${properties.kcFormOptionsWrapperClass!}">
                            <span><a href="${url.loginUrl}">${kcSanitize(msg("backToLogin"))?no_esc}</a></span>
                        </div>
                    </div>
                </#if>

                <div id="kc-form-buttons" class="${properties.kcFormButtonsClass!}">
                    <input
                        id="termsOfServiceText"
                        class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                        type="submit"
                        value="<#if formMode?? && formMode = 'LOGIN'>${msg("doLogIn")}<#else>${msg("doRegister")}</#if>"
                    />
                </div>
            </div>
        </form>
        <#if structuredCredentialLogin>
            <script type="module" src="${url.resourcesPath}/js/structured-credential.js"></script>
        <#else>
            <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>
        </#if>

        <@telInputWidget.assets/>
        <@selectFilterWidget.assets/>

        <#--  Password strength  -->
        <#--  https://github.com/dropbox/zxcvbn  -->
        <script type="text/javascript" src="${url.resourcesPath}/js/zxcvbn.js"></script>
        <script type="text/javascript" src="${url.resourcesPath}/js/keycloak-password-strength.js"></script>
    <#elseif section = "socialProviders" >
        <@socialProviders.render/>
    </#if>
</@layout.registrationLayout>
