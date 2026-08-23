<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Upgrades every plain `input[type='tel']` on the page (however it was rendered - a User
      Profile attribute with an `html5-tel` inputType, via user-profile-commons.ftl, is the
      common case) into an intl-tel-input widget. Shared by register.ftl and login.ftl so both
      get the same phone-number UI for any tel-typed attribute, not just a hardcoded field name.
      https://github.com/jackocnr/intl-tel-input/tree/master  -->

<#macro assets>
    <link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/intlTelInput.css">
    <link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/customized.css">
    <script type="text/javascript" src="${url.resourcesPath}/intl-tel-input-23.3.2/js/intlTelInputWithUtils.min.js"></script>

    <#--  Timezone country code data  -->
    <script type="text/javascript" src="${url.resourcesPath}/js/timezone-countrycode-data.js"></script>

    <script>
        <#--  Deferred so the widget also upgrades tel inputs rendered after this block:
              register.ftl emits its assets below the form, login.ftl above its field loop.  -->
        document.addEventListener('DOMContentLoaded', function () {
            // Get all inputs that use type tel
            const listTelInputs = document.querySelectorAll("input[type='tel']");
            listTelInputs.forEach(function (input) {
                // A profile pattern describes the stored, normalized value. With a separate dial
                // code the visible control holds only the national part, so native validation on
                // that value would reject valid E.164 numbers.
                const configuredPattern = input.getAttribute("pattern");
                if (configuredPattern !== null) {
                    input.removeAttribute("pattern");
                }

                // Keep the id so the visible input remains associated with its label. Rename only
                // the raw input; intl-tel-input submits the normalized value under the original id.
                let id = input.id;
                input.name = id + "-input";

                // Use intel-tel-input
                const phoneInput = window.intlTelInput(input, {
                    initialCountry: "auto",
                    separateDialCode: true,
                    customPlaceholder: function(selectedCountryPlaceholder, selectedCountryData) {
                        return selectedCountryPlaceholder.replace(/\d/g, '0');
                    },
                    hiddenInput: () => ({ phone: id, country: "country_code" }),
                    geoIpLookup: function(success, failure) {
                        <#--  A timezone with no entry in the map, or a map that failed to load,
                              must fall through to failure() rather than throw - otherwise the
                              widget is left without an initial country.  -->
                        try {
                            const userTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
                            let timezoneCountrycodeData = typeof data !== 'undefined' ? JSON.parse(data) : {};
                            let countryCode = timezoneCountrycodeData[userTimeZone]?.toString();

                            if (countryCode) {
                                return success(countryCode);
                            }
                        } catch (e) {}
                        return failure();
                    },
                });

                if (configuredPattern !== null) {
                    const validateNormalizedPattern = function () {
                        input.setCustomValidity("");
                        if (!input.value) {
                            return;
                        }

                        // Let the browser interpret the configured HTML pattern and provide its
                        // localized validation message, but test the normalized E.164 value.
                        const patternProbe = document.createElement("input");
                        patternProbe.type = "text";
                        patternProbe.required = true;
                        patternProbe.pattern = configuredPattern;
                        patternProbe.value = phoneInput.getNumber();
                        if (!patternProbe.validity.valid) {
                            input.setCustomValidity(patternProbe.validationMessage);
                        }
                    };
                    input.addEventListener("input", validateNormalizedPattern);
                    input.addEventListener("countrychange", validateNormalizedPattern);
                    validateNormalizedPattern();
                }
            });
        });
    </script>
</#macro>
