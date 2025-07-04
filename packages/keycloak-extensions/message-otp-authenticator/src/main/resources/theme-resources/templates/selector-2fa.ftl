<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout; section>
    <#if section = "header">
		${msg("selector2FATitle")}
    <#elseif section = "form">
		<p>${msg("selector2FAText")}</p>
		<style>
			.kc-2fa-selector-desc {
				margin-top: 10px;
				text-align: center;
			}
		</style>
		<form
			id="configCredentials"
			class="${properties.kcFormClass!}"
			action="${url.loginAction}"
			method="post"
		>
			<#-- requiredActions is a List<String> of required action ids -->
			<#list requiredActions as requiredActionId>
				<div class="${properties.kcFormGroupClass!}">
					<div
						id="kc-form-buttons"
						class="${properties.kcFormButtonsClass!}"
					>
						<button
							class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
							name="requiredActionName"
							type="submit"
							value="${requiredActionId}"
						>
							${msg("requiredAction." +  requiredActionId + ".label")}
						</button>
						<p class="kc-2fa-selector-desc">
							${msg("requiredAction." +  requiredActionId + ".description")}
						</p>
					</div>
				</div>
			</#list>
		</form>
    </#if>
</@layout.registrationLayout>
