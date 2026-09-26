<#--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout ; section>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper" class="identity-verification-error-form">
                <p class="error-message">${kcSanitize(msg(error))?no_esc}</p>
                <p class="error-message">code_id: ${code_id}</p>
                <#if canRetry>
                    <form action="${url.loginAction}" method="post">
                        <input type="hidden" name="action" value="retry" />
                        <button type="submit" class="retry-link ${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!}">
                            ${msg("linkTryAgain")}
                        </button>
                    </form>
                </#if>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
