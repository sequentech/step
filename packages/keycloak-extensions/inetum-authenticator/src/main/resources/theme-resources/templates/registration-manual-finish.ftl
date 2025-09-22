<#--
    SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header" || section = "show-username">
        ${msg("registerFinishManualTitle")?no_esc}
    <#elseif section = "form">
        <div class="last-page-text">
            <#if rejectReason??>
                <p>${msg(rejectReason)}</p>
            </#if>
            <#if mismatchedFields??>
                <p>${msg("rejectReasonListItems")?no_esc}</p>
                <ul>
                <#list mismatchedFields?keys as key>
                    <#if mismatchedFields[key]??>
                        <li>${key}: <strong>${mismatchedFields[key]}</strong></li>
                    <#else>
                        <li>${key}: <i>${msg("empty")}</i></li>
                    </#if>
                </#list>
                </ul>
            </#if>
            <p>${msg("registerFinishManualMessage")?no_esc}</p>
            <p id="instruction1" class="instruction">
                ${msg("pageExpiredMsg2")} <a id="loginContinueLink" href="${url.loginRestartFlowUrl}">${msg("doClickHere")}</a> .
            </p>
        </div>
    </#if>
</@layout.registrationLayout>