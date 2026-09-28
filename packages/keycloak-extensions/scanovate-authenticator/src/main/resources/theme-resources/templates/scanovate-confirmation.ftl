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
                color: #72767b;
                font-size: 14px;
                line-height: 1.5;
                margin-bottom: 24px;
                max-width: 400px;
            }
            .scanovate-data-card {
                background: var(--pf-global--BackgroundColor--100, #F7F9FE);
                border-radius: 8px;
                width: 100%;
                margin-bottom: 24px;
                text-align: left;
            }
            .scanovate-data-row {
                padding: 12px 16px;
            }
            .scanovate-data-row + .scanovate-data-row {
                border-top: 1px solid rgba(0, 0, 0, 0.08);
            }
            .scanovate-data-label {
                font-size: 11px;
                text-transform: uppercase;
                color: #72767b;
                letter-spacing: 0.05em;
                margin-bottom: 2px;
            }
            .scanovate-data-value {
                font-size: 16px;
                font-weight: 500;
                color: var(--pf-global--palette--black-800, #191D23);
            }
            .scanovate-btn-secondary {
                background: none;
                border: none;
                color: #72767b;
                font-size: 14px;
                cursor: pointer;
                padding: 8px 16px;
                margin-top: 8px;
            }
            .scanovate-btn-secondary:hover {
                text-decoration: underline;
            }
        </style>
    <#elseif section = "header">
        ${msg("scanovateConfirmationTitle")}
    <#elseif section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <div class="scanovate-container">
                    <div class="scanovate-icon">
                        <svg width="48" height="48" viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
                            <circle cx="24" cy="24" r="24" fill="#4CAF50" opacity="0.12"/>
                            <circle cx="24" cy="24" r="18" fill="#4CAF50"/>
                            <path d="M20 24.5L22.5 27L28 21" stroke="white" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </div>
                    <p class="scanovate-description">${msg("scanovateConfirmationDescription")}</p>
                </div>
                <div class="scanovate-data-card">
                    <#list storedAttributes as attribute>
                        <div class="scanovate-data-row">
                            <div class="scanovate-data-label">${msg(attribute.key())}</div>
                            <div class="scanovate-data-value">${attribute.value()}</div>
                        </div>
                    </#list>
                </div>
                <form id="scanovate-confirmation" action="${url.loginAction}" method="post">
                    <div class="${properties.kcFormGroupClass!}">
                        <button type="submit" name="action" value="confirm"
                            class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!}">
                            ${msg("ButtonContinue")}
                        </button>
                        <div style="text-align: center;">
                            <button type="submit" name="action" value="retry" class="scanovate-btn-secondary">
                                ${msg("ButtonRepeat")}
                            </button>
                        </div>
                    </div>
                </form>
                <script>
                    document.getElementById("scanovate-confirmation").addEventListener("submit", (event) => {
                        event.target.querySelectorAll("button").forEach((button) => {
                            if (event.submitter === button) {
                                button.textContent = "${msg('ButtonSubmitting')?js_string}";
                            }
                            setTimeout(() => (button.disabled = true));
                        });
                    });
                </script>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
