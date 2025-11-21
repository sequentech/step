<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Source: https://github.com/keycloak/keycloak/blob/24.0.0/themes/src/main/resources/theme/base/login/register.ftl  -->

<#import "template.ftl" as layout>
<#import "user-profile-commons.ftl" as userProfileCommons>
<#import "register-commons.ftl" as registerCommons>
<#include "intl-tel-input.ftl">
<@layout.registrationLayout displayMessage=messagesPerField.exists('global') displayRequiredFields=true; section>
    <#if section = "header">
        <#if formMode?? && formMode = 'LOGIN'>
            ${msg('loginTitle',(realm.displayName!''))}
        <#else>
            ${msg('registerTitle')}
        </#if>
    <#elseif section = "form">
        <form id="kc-register-form" class="${properties.kcFormClass!}" action="${url.registrationAction}" method="post">

            <@userProfileCommons.userProfileFormFields; callback, attribute>
                <#if callback = "afterField">
                    <#if attribute.name == 'mobile'>
                        <div class="${properties.kcFormGroupClass!}">
                            <div class="${properties.kcLabelWrapperClass!}">
                                <label for="mobile" class="${properties.kcLabelClass!}">${msg("mobileOtp.auth.enterMobileLabel")}</label>
                            </div>
                            <div class="${properties.kcInputWrapperClass!}">
                                <@renderIntlTelInput id="mobile" name="mobile" value=attribute.value />
                            </div>
                        </div>
                    <#else>
                        <#-- render password fields just under the username or email (if used as username) -->
                        <#if passwordRequired && (attribute.name == 'username' || (attribute.name == 'email' && realm.registrationEmailAsUsername)) && (attribute.annotations.showPasswordAfterThis!'true') != 'false' || (attribute.annotations.showPasswordAfterThis!'false') == 'true'>
                            <div class="${properties.kcFormGroupClass!}">
                                <div class="${properties.kcLabelWrapperClass!}">
                                    <label for="password" class="${properties.kcLabelClass!}">${msg("password")}</label> *
                                </div>
                                <div class="${properties.kcInputWrapperClass!}">
                                    <#--  You can add a custom passwordHelperTextBefore to either username or email depending on realm.registrationEmailAsUsername settings to add a helpertext -->
                                    <#if attribute.annotations.passwordHelperTextBefore??>
                                        <div class="${properties.kcInputHelperTextBeforeClass!}" id="form-help-text-before-${attribute.name}" aria-live="polite">${kcSanitize(advancedMsg(attribute.annotations.passwordHelperTextBefore))?no_esc}</div>
                                    </#if>

                                    <div class="${properties.kcInputGroup!}">
                                        <input type="password" id="password" class="${properties.kcInputClass!}" name="password"
                                               autocomplete="new-password"
                                               aria-invalid="<#if messagesPerField.existsError('password','password-confirm')>true</#if>"
                                        />
                                        <button class="${properties.kcFormPasswordVisibilityButtonClass!}" type="button" aria-label="${msg('showPassword')}"
                                                aria-controls="password"  data-password-toggle
                                                data-icon-show="${properties.kcFormPasswordVisibilityIconShow!}" data-icon-hide="${properties.kcFormPasswordVisibilityIconHide!}"
                                                data-label-show="${msg('showPassword')}" data-label-hide="${msg('hidePassword')}">
                                            <i class="${properties.kcFormPasswordVisibilityIconShow!}" aria-hidden="true"></i>
                                        </button>
                                    </div>

                                    <#--  You can add a password strength bar if passwordStrengthBar is set to either username or email depending on realm.registrationEmailAsUsername settings to add a strength bar -->
                                    <#if attribute.annotations.passwordStrengthBar?? && formMode?? && (formMode!"REGISTRATION") != "LOGIN">
                                        <div class="pf-c-progress pf-m-sm" id="password-progress">
                                            <div class="pf-c-progress__bar" id="password-progress-aria" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0" aria-labelledby="password-progress">
                                                <div class="pf-c-progress__indicator" id="password-progress-indicator"></div>
                                            </div>
                                        </div>
                                    </#if>

                                    <#if messagesPerField.existsError('password')>
                                        <span id="input-error-password" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
		                                ${kcSanitize(messagesPerField.get('password'))?no_esc}
		                            </span>
                                    </#if>

                                    <#--  You can add a custom passwordHelperTextAfter to either username or email depending on realm.registrationEmailAsUsername settings to add a helpertext -->
                                    <#if attribute.annotations.passwordHelperTextAfter??>
                                        <div class="${properties.kcInputHelperTextAfterClass!}" id="form-help-text-after-${attribute.name}" aria-live="polite">${kcSanitize(advancedMsg(attribute.annotations.passwordHelperTextAfter))?no_esc}</div>
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
                        </#if>
                    </#if>
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
        <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>

        <#--  Adding intel-tel-input  -->
        <#--  https://github.com/jackocnr/intl-tel-input/tree/master  -->

        <link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/intlTelInput.css">
        <link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/customized.css">
        <script type="text/javascript" src="${url.resourcesPath}/intl-tel-input-23.3.2/js/intlTelInput.min.js"></script>

        <#--  Timezone country code data  -->
        <script type="text/javascript" src="${url.resourcesPath}/js/timezone-countrycode-data.js"></script>

        <#-- jQuery -->
        <script type="text/javascript" src="${url.resourcesPath}/js/jquery-3.7.1.slim.min.js"></script>

        <script>
            // Get all inputs that use type tel
            const listTelInputs = document.querySelectorAll("input[type='tel']");
            listTelInputs.forEach(function (input) {
                // Change id and name to use the correctly formatted phone number in the form
                let id = input.id;
                input.id = id + "-input";
                input.name = id + "-input";

                // Use intel-tel-input
                window.intlTelInput(input, {
                    utilsScript: "${url.resourcesPath}/intl-tel-input-23.3.2/js/utils.js",
                    initialCountry: "auto",
                    separateDialCode: true,
					customPlaceholder: function(selectedCountryPlaceholder, selectedCountryData) {
						return selectedCountryPlaceholder.replace(/\d/g, '0');
					},
                    hiddenInput: () => ({ phone: id, country: "country_code" }),
                    geoIpLookup: function(success, failure) {
                        const userTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;

                        let timezoneCountrycodeData = JSON.parse(data);
                        let countryCode = timezoneCountrycodeData[userTimeZone].toString();

                        if (countryCode) {
                            return success(countryCode);
                        }
                        return failure();
                    },
                });
            });
        </script>

        <#-- Filter for select inputs -->
        <script>
            function filterSelectAttribute(e, elementId) {
                e = e || window.event;
                var selectElement = e.target;
                var value = selectElement.value;

                let first = null;
                $('#' + elementId + ' option').hide();
                $('#' + elementId).find('option').filter(function() {
                    var optionValue = $(this)[0].value;
                    let found = optionValue.indexOf(value) != -1;
                    if (found && first === null) {
                        first = optionValue;
                    }
                    return found;
                }).show();
                
                // Set default value
                $('#' + elementId).val(first);
            }
        </script>

        <#--  Password strength  -->
        <#--  https://github.com/dropbox/zxcvbn  -->
        <script type="text/javascript" src="${url.resourcesPath}/js/zxcvbn.js"></script>
        <script type="text/javascript" src="${url.resourcesPath}/js/keycloak-password-strength.js"></script>
    </#if>
</@layout.registrationLayout>
