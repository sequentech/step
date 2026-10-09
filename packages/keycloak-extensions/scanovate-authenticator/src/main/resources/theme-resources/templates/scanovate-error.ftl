<#--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "head">
        <style>
            .scanovate-container {
                display: flex;
                flex-direction: column;
                align-items: center;
                text-align: center;
            }
            .scanovate-icon {
                margin-bottom: 16px;
            }
            .scanovate-description {
                color: var(--pf-global--palette--black-800, #191D23);
                font-size: 14px;
                line-height: 1.5;
                margin-bottom: 24px;
                max-width: 400px;
            }
            .scanovate-reference {
                font-size: 12px;
                color: #72767b;
                margin-top: 16px;
            }
        </style>
    <#elseif section = "header">
        ${msg("scanovateErrorTitle")}
    <#elseif section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper" class="identity-verification-error-form">
                <div class="scanovate-container">
                    <div class="scanovate-icon">
                        <svg width="48" height="48" viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <circle cx="24" cy="24" r="24" fill="#D32F2F" opacity="0.12"/>
                            <circle cx="24" cy="24" r="18" fill="#D32F2F"/>
                            <path d="M24 19V25" stroke="white" stroke-width="2.5" stroke-linecap="round"/>
                            <circle cx="24" cy="29" r="1.25" fill="white"/>
                        </svg>
                    </div>
                    <p class="scanovate-description">${kcSanitize(msg(error))?no_esc}</p>
                    <#if canRetry>
                        <form action="${url.loginAction}" method="post" style="width: 100%;">
                            <input type="hidden" name="action" value="retry" />
                            <button type="submit"
                                class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!}">
                                ${msg("linkTryAgain")}
                            </button>
                        </form>
                    </#if>
                    <p class="scanovate-reference">${msg("scanovateErrorReferenceLabel")}: ${code_id}</p>
                </div>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
