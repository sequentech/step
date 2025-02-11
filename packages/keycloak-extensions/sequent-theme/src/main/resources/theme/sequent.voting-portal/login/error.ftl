<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header">
        ${kcSanitize(msg("errorTitle"))?no_esc}
    <#elseif section = "form">
        <div id="kc-error-message">
            <p class="instruction">${kcSanitize(message.summary)?no_esc}</p>
            <#if skipLink??>
            <#elseif client?? && client.baseUrl?has_content>
                <p><a id="backToApplication" href="${client.baseUrl}">
                    ${kcSanitize(msg("backToApplication"))?no_esc}
                </a></p>
            <#else>
                <div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
                    <div id="kc-form-options" class="${properties.kcFormOptionsClass!}">
                        <div class="${properties.kcFormOptionsWrapperClass!}">
                            <span>
                                <a href="${url.loginUrl}?client_id=voting-portal">
                                    ${kcSanitize(msg("backToLogin"))?no_esc}
                                </a>
                            </span>
                        </div>
                    </div>
                </div>
            </#if>
        </div>
    </#if>
</@layout.registrationLayout>
