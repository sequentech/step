<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
    <#if section = "header">
        <title>${msg("emailOtp.auth.enterEmailTitle")}</title>
        <h2>${msg("emailOtp.auth.enterEmailTitle")}</h2>
    <#elseif section = "form">
        <#if error??>
            <div class="error">${error}</div>
        </#if>
        <form method="post">
            <label for="email">${msg("emailOtp.auth.enterEmailLabel")}</label>
            <input type="email" id="email" name="email" required autofocus />
            <div class="help-message" style="margin: 8px 0 16px 0; color: #555;">
                ${msg("emailOtp.auth.enterEmailHelp")}
            </div>
            <button type="submit">${msg("emailOtp.auth.sendCodeButton")}</button>
        </form>
    </#if>
</@layout.registrationLayout>
