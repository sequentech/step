<#--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header">
        ${msg("scanovateCaptureTitle")}
    <#elseif section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <p style="text-align: center;">${msg("scanovateCaptureThemeRequired")}</p>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
