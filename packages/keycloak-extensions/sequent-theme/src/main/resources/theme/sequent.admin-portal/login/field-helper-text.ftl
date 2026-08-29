<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Renders a User Profile attribute's inputHelperTextBefore/After annotation the same way
      wherever that attribute is collected: register.ftl (via user-profile-commons.ftl) and
      login.ftl's matchAttributes loop (MultiAttributePasswordAuthenticator). Lives in
      sequent.admin-portal/login so sequent.voting-portal inherits it without its own copy.  -->

<#macro helperTextBefore id text>
    <#if text?has_content>
        <div class="${properties.kcInputHelperTextBeforeClass!}" id="form-help-text-before-${id}" aria-live="polite">${kcSanitize(advancedMsg(text))?no_esc}</div>
    </#if>
</#macro>

<#macro helperTextAfter id text>
    <#if text?has_content>
        <div class="${properties.kcInputHelperTextAfterClass!}" id="form-help-text-after-${id}" aria-live="polite">${kcSanitize(advancedMsg(text))?no_esc}</div>
    </#if>
</#macro>
