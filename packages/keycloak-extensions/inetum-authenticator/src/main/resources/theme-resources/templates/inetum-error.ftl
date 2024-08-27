<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout ; section>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <span>${msg(error)?no_esc}</span>
                <a href="#" onclick="location.reload(); return false;">Try again</a>
                <p>If the problem persist, please refer in the help desk to identificator 11421 for manual resolution.</p>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
