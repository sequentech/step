<#--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#--
Reusable partial for rendering a phone input with intl-tel-input in Keycloak forms.
Usage: <#include "intl-tel-input.ftl"> and call renderIntlTelInput(id, name, value)
-->
<#macro renderIntlTelInput id name value="">
    <input
        id="${id}"
        name="${name}"
        type="tel"
        class="${properties.kcInputClass!} intl-tel-input-field"
        value="${value}"
        required
        autofocus
    />
</#macro>

<link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/intlTelInput.css">
<link rel="stylesheet" href="${url.resourcesPath}/intl-tel-input-23.3.2/css/customized.css">
<script type="text/javascript" src="${url.resourcesPath}/intl-tel-input-23.3.2/js/intlTelInput.min.js"></script>
<script type="text/javascript" src="${url.resourcesPath}/js/timezone-countrycode-data.js"></script>
<script type="text/javascript" src="${url.resourcesPath}/js/jquery-3.7.1.slim.min.js"></script>
<script>
    document.addEventListener('DOMContentLoaded', function() {
        const telInputs = document.querySelectorAll('.intl-tel-input-field');
        telInputs.forEach(function(input) {
            window.intlTelInput(input, {
                utilsScript: "${url.resourcesPath}/intl-tel-input-23.3.2/js/utils.js",
                initialCountry: "auto",
                separateDialCode: true,
                customPlaceholder: function(selectedCountryPlaceholder, selectedCountryData) {
                    return selectedCountryPlaceholder.replace(/\d/g, '0');
                },
                hiddenInput: () => ({ phone: input.id, country: "country_code" }),
                geoIpLookup: function(success, failure) {
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
