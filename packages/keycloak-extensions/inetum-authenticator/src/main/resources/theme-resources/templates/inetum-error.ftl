<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout ; section>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper" class="inetum-error-form">
                <p class="error-message">${msg(error)?no_esc}</p>
                <p class="error-message">code_id: ${code_id}</p>
                <#if error != "maxRetriesError">
                    <a class="retry-link" href="#" onclick="location.reload(); return false;">${msg("linkTryAgain")?no_esc}</a>
                </#if>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
