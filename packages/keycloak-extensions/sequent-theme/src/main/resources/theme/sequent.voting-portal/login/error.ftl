<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header">
    <#elseif section = "form">
        <div id="kc-error-message">
            <h1 class="kc-error-title">${msg("errorTitle")}</h1>
            <div class="kc-error-alert">
                <div class="kc-error-alert-icon-wrap">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="kc-error-alert-icon" aria-hidden="true">
                        <path d="M11 15h2v2h-2zm0-8h2v6h-2zm.99-5C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z"/>
                    </svg>
                </div>
                <span class="kc-error-alert-text">${kcSanitize(message.summary)?no_esc}</span>
            </div>
            <#if skipLink??>
            <#elseif client?? && client.baseUrl?has_content>
                <a id="backToLogin" class="kc-error-back-btn" href="${client.baseUrl}">
                    ${msg("backToLogin")}
                </a>
            <#else>
                <a id="backToLogin" class="kc-error-back-btn" href="${url.loginUrl}">
                    ${msg("backToLogin")}
                </a>
            </#if>
        </div>
    </#if>
</@layout.registrationLayout>
