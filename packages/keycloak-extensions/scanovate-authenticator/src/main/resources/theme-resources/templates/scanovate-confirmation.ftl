<#--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=false; section>
    <#if section = "header">
        ${msg("ConfirmationTitle")}
    <#elseif section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <p>${msg("ConfirmationDescription")}</p>
                <#list storedAttributes as attribute>
                    <div class="${properties.kcFormGroupClass!}">
                        <label for="${attribute.key()}" class="${properties.kcLabelClass!}">${msg(attribute.key())}</label>
                        <input id="${attribute.key()}" class="${properties.kcInputClass!}" type="${attribute.type()}"
                            value="${attribute.value()}" disabled />
                    </div>
                </#list>
                <form id="scanovate-confirmation" action="${url.loginAction}" method="post">
                    <div class="${properties.kcFormGroupClass!}">
                        <button type="submit" name="action" value="confirm"
                            class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!}">
                            ${msg("ButtonContinue")}
                        </button>
                        <button type="submit" name="action" value="retry"
                            class="${properties.kcButtonClass!} ${properties.kcButtonDefaultClass!} ${properties.kcButtonBlockClass!}">
                            ${msg("ButtonRepeat")}
                        </button>
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
