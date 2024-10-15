<#--
    SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
    <#import "template.ftl" as layout>
        <@layout.registrationLayout displayMessage=false; section>
            <#if section="form">
                <h3>${msg("ConfirmationTitle")}</h3>
                <#list storedAttributes as key, value>
                    <div class="${properties.kcFormGroupClass!}">
                        <label for="${key}" class="${properties.kcLabelClass!}">
                            ${msg(key)}
                        </label>
                        <input tabindex="1" id="${key}" class="${properties.kcInputClass!}" name="${key}" type="text" autofocus autocomplete="off"
                            value="${value}" disabled />
                    </div>
                </#list>
                <form action="${actionUrl}" method="post">
                    <div id="kc-form-buttons" class="${properties.kcFormGroupClass!}">
                        <button name="action" value="confirm" type="submit" class="g-recaptcha ${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}">Confirm</button>
                        <button onclick="location.reload(); return false;" class="g-recaptcha ${properties.kcFormPasswordVisibilityButtonClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}">Retry</button>
                    </div>
                </form>
            </#if>
        </@layout.registrationLayout>