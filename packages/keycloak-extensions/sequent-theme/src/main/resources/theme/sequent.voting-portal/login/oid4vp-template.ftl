<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#-- Override of the OID4VP extension's global oid4vp-template.ftl (from
     theme-resources/templates/). Delegates to template.ftl so the Sequent
     header (logo, version, hash, language selector) appears on the wallet
     login page, matching all other login pages in this theme. -->

<#import "template.ftl" as adminTemplate>

<#macro registrationLayout bodyClass="" displayInfo=false displayMessage=true displayRequiredFields=false>
<@adminTemplate.registrationLayout
    displayInfo=displayInfo
    displayMessage=displayMessage
    displayRequiredFields=displayRequiredFields
    displaySocialProviders=false; section>
    <#if section = "head">
        <style>
            /* The OID4VP flow does not support locale switching — the extension
               appends the locale to a URL that has no handler, causing an error.
               Show the current language label but suppress the dropdown. */
            #kc-current-locale-link { pointer-events: none; cursor: default; }
            #kc-current-locale-link img:last-child { display: none; }
            #language-switch1 { display: none !important; }
        </style>
    <#else>
        <#nested section>
    </#if>
</@adminTemplate.registrationLayout>
</#macro>
