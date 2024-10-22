<#--
    SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
    <#import "template.ftl" as layout>
        <@layout.registrationLayout displayMessage=false; section>
            <#if section="form">
                <h3>${msg("ConfirmationTitle")}</h3>
                <#list storedAttributes as attribute>
                    <div class="${properties.kcFormGroupClass!}">
                        <label for="${attribute.key}" class="${properties.kcLabelClass!}">
                            ${msg(key)}
                        </label>
                        <input tabindex="1" id="${attribute.key}" class="${properties.kcInputClass!}" name="${attribute.key}" type="${attribute.type}" autofocus autocomplete="off"
                            value="${attribute.value}" disabled />
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