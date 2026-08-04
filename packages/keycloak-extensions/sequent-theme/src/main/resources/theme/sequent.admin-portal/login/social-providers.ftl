<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Renders the "socialProviders" section, filtering out the digital-certificates IdP unless the
      realm has opted in via voter-certificate-policy=enabled. Shared by register.ftl and both
      portals' login.ftl.  -->

<#macro render>
    <#assign visibleProviders = social.providers?filter(p -> p.alias != 'digital-certificates' || (realm.attributes['voter-certificate-policy']!'disabled') == 'enabled')>
    <#if realm.password && visibleProviders?has_content>
        <hr/>
        <h4 style="text-align: center;">${msg("identity-provider-login-label")}</h4>
        <div id="kc-social-providers" class="${properties.kcFormSocialAccountSectionClass!}">
            <#list visibleProviders as p>
                <ul class="${properties.kcFormSocialAccountListClass!} <#if visibleProviders?size gt 3>${properties.kcFormSocialAccountListGridClass!}</#if>">
                    <li>
                        <a id="social-${p.alias}" class="${properties.kcFormSocialAccountListButtonClass!} <#if visibleProviders?size gt 3>${properties.kcFormSocialAccountGridItem!}</#if>"
                                type="button" href="${p.loginUrl}">
                            <#if p.iconClasses?has_content>
                                <i class="${properties.kcCommonLogoIdP!} ${p.iconClasses!}" aria-hidden="true"></i>
                                <span class="${properties.kcFormSocialAccountNameClass!} kc-social-icon-text"><#if p.alias == 'digital-certificates'>${msg("digitalCertificateButton")}<#else>${msg(p.displayName)!}</#if></span>
                            <#else>
                                <span class="${properties.kcFormSocialAccountNameClass!}"><#if p.alias == 'digital-certificates'>${msg("digitalCertificateButton")}<#else>${msg(p.displayName)!}</#if></span>
                            </#if>
                        </a>
                    </li>
                </ul>
            </#list>
        </div>
    </#if>
</#macro>
