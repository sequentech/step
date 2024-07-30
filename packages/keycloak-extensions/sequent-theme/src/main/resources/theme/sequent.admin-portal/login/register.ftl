<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Source: https://github.com/keycloak/keycloak/blob/24.0.0/themes/src/main/resources/theme/base/login/register.ftl  -->

<#import "template.ftl" as layout>
<#import "user-profile-commons.ftl" as userProfileCommons>
<#import "register-commons.ftl" as registerCommons>
<@layout.registrationLayout displayMessage=messagesPerField.exists('global') displayRequiredFields=true; section>
    <#if section = "header">
        ${msg("registerTitle")}
    <#elseif section = "form">
        <form id="kc-register-form" class="${properties.kcFormClass!}" action="${url.registrationAction}" method="post">

            <@userProfileCommons.userProfileFormFields; callback, attribute>
                <#if callback = "afterField">
                <#-- render password fields just under the username or email (if used as username) -->
                    <#if passwordRequired?? && (attribute.name == 'username' || (attribute.name == 'email' && realm.registrationEmailAsUsername))>
                        <div class="${properties.kcFormGroupClass!}">
                            <div class="${properties.kcLabelWrapperClass!}">
                                <label for="password" class="${properties.kcLabelClass!}">${msg("password")}</label> *
                            </div>
                            <div class="${properties.kcInputWrapperClass!}">
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

                                <#if messagesPerField.existsError('password')>
                                    <span id="input-error-password" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
		                                ${kcSanitize(messagesPerField.get('password'))?no_esc}
		                            </span>
                                </#if>
                            </div>
                        </div>

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
                <div id="kc-form-options" class="${properties.kcFormOptionsClass!}">
                    <div class="${properties.kcFormOptionsWrapperClass!}">
                        <span><a href="${url.loginUrl}">${kcSanitize(msg("backToLogin"))?no_esc}</a></span>
                    </div>
                </div>

                <div id="kc-form-buttons" class="${properties.kcFormButtonsClass!}">
                    <input class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}" type="submit" value="${msg("doRegister")}"/>
                </div>
            </div>
        </form>
        <script type="module" src="${url.resourcesPath}/js/passwordVisibility.js"></script>

        <#--  Adding intel-tel-input  -->
        <#--  https://github.com/jackocnr/intl-tel-input/tree/master  -->

        <link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/intlTelInput.css">
        <script type="text/javascript" src="${url.resourcesPath}/intl-tel-input-23.3.2/js/intlTelInput.min.js"></script>

        <#--  Timezone country code data  -->

        <script type="text/javascript" src="${url.resourcesPath}/js/timezone-countrycode-data.js"></script>

        <#-- jQuery -->
        <script type="text/javascript" src="${url.resourcesPath}/js/jquery-3.7.1.slim.min.js"></script>

        <script>
            // Get all inputs that use type tel
            const listTelInputs = document.querySelectorAll("input[type='tel']");
            listTelInputs.forEach(function (input) {
                // Use intel-tel-input
                window.intlTelInput(input, {
                    utilsScript: "${url.resourcesPath}/intl-tel-input-23.3.2/js/utils.js",
                    initialCountry: "auto",
                    separateDialCode: true,
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

        <#--  Disable field function. Turns inputs into read only. Add a disableAttribute annotation to a select or multiselect user profile attribute. -->
        <script>
            function readOnlyElementById(e, idToSetReadOnly) {
                e = e || window.event;
                var target = e.target || e.srcElement;

                document.getElementById(idToSetReadOnly).readOnly = !target.checked;
            }
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
    </#if>
</@layout.registrationLayout>
