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
    <script type="text/javascript" src="${url.resourcesPath}/intl-tel-input-23.3.2/js/intlTelInput.min.js"></script>

    <#--  Timezone country code data  -->
    <script type="text/javascript" src="${url.resourcesPath}/js/timezone-countrycode-data.js"></script>

    <script>
        <#--  Deferred so the widget also upgrades tel inputs rendered after this block:
              register.ftl emits its assets below the form, login.ftl above its field loop.  -->
        document.addEventListener('DOMContentLoaded', function () {
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
            });
        });
    </script>
</#macro>
