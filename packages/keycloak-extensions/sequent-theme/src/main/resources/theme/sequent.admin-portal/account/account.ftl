<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.accountLayout section>
    <#if section == "header">
        ${msg("accountTitle")}
    <#elseif section == "content">
        <div id="kc-account-wrapper">
            <div id="kc-account-tabs">
                <ul class="kc-tabs">
                    <li class="kc-tab"><a href="${url.accountUrl}">${msg("personalInfo")}</a></li>
                    <li class="kc-tab"><a href="${url.accountSecurityUrl}">${msg("accountSecurity")}</a></li>
                </ul>
            </div>
            <div id="kc-account-content">
                <#if accountSection == "personalInfo">
                    <#-- Personal Info Section -->
                    <form id="kc-account-form" action="${url.accountUpdateAction}" method="post">
                        <div class="${properties.kcFormGroupClass!}">
                            <label for="username" class="${properties.kcLabelClass!}">${msg("username")}</label>
                            <input id="username" class="${properties.kcInputClass!}" name="username" value="${account.username}" type="text" readonly />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <label for="email" class="${properties.kcLabelClass!}">${msg("email")}</label>
                            <input id="email" class="${properties.kcInputClass!}" name="email" value="${account.email}" type="email" />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <label for="firstName" class="${properties.kcLabelClass!}">${msg("firstName")}</label>
                            <input id="firstName" class="${properties.kcInputClass!}" name="firstName" value="${account.firstName}" type="text" />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <label for="lastName" class="${properties.kcLabelClass!}">${msg("lastName")}</label>
                            <input id="lastName" class="${properties.kcInputClass!}" name="lastName" value="${account.lastName}" type="text" />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <button type="submit" class="${properties.kcButtonClass!}">${msg("save")}</button>
                        </div>
                    </form>
                <#elseif accountSection == "accountSecurity">
                    <#-- Account Security Section -->
                    <form id="kc-security-form" action="${url.accountSecurityAction}" method="post">
                        <div class="${properties.kcFormGroupClass!}">
                            <label for="currentPassword" class="${properties.kcLabelClass!}">${msg("currentPassword")}</label>
                            <input id="currentPassword" class="${properties.kcInputClass!}" name="currentPassword" type="password" />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <label for="newPassword" class="${properties.kcLabelClass!}">${msg("newPassword")}</label>
                            <input id="newPassword" class="${properties.kcInputClass!}" name="newPassword" type="password" />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <label for="confirmPassword" class="${properties.kcLabelClass!}">${msg("confirmPassword")}</label>
                            <input id="confirmPassword" class="${properties.kcInputClass!}" name="confirmPassword" type="password" />
                        </div>

                        <div class="${properties.kcFormGroupClass!}">
                            <button type="submit" class="${properties.kcButtonClass!}">${msg("save")}</button>
                        </div>
                    </form>
                </#if>
            </div>
        </div>
    <#elseif section == "info">
        <div id="kc-info-message">
            <p>${msg("accountInfoMessage")}</p>
        </div>
    </#if>
</@layout.accountLayout>
