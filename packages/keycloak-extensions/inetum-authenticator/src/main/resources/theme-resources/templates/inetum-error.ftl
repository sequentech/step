<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout ; section>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper" class="inetum-error-form">
                <span class="error-message">${msg(error)?no_esc}</span>
                <span class="error-message">code_id: ${code_id}</span>
                <a class="retry-link" href="#" onclick="location.reload(); return false;">${msg("linkTryAgain")?no_esc}</a>
                <p>If the problem persist, please refer in the help desk to identificator 11421 for manual resolution.</p>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
