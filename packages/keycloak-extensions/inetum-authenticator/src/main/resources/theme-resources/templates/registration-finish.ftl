<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header">
        <#if messageHeader??>
            ${kcSanitize(msg("${messageHeader}"))?no_esc}
        <#else>
            ${msg("registerTitle")}
        </#if>
    <#elseif section = "form">
        <p id="instruction1" class="instruction">
            ${msg("pageExpiredMsg2")} <a id="loginContinueLink" href="${url.loginRestartFlowUrl}">${msg("doClickHere")}</a> .
        </p>
    </#if>
</@layout.registrationLayout>