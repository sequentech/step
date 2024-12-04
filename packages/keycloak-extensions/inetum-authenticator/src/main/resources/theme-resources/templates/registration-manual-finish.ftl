<#--
    SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header" || section = "show-username">
        ${msg("registerFinishManualTitle")?no_esc}
    <#elseif section = "form">
        <#if rejectReason??>
            <p>${msg(rejectReason)}</p>
        </#if>
        <p>${msg("registerFinishManualMessage")?no_esc}</p>
        <p id="instruction1" class="instruction">
            ${msg("pageExpiredMsg2")} <a id="loginContinueLink" href="${url.loginRestartFlowUrl}">${msg("doClickHere")}</a> .
        </p>
    </#if>
</@layout.registrationLayout>