<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=true; section>
	<#if section = "header">
		${msg("messageOtp.auth.title", realm.displayName)}
	<#elseif section = "show-username">
		<h1>${msg("messageOtp.auth.codeTitle", realm.displayName)}</h1>
	<#elseif section = "form">
		<form
			id="kc-message-code-login-form"
			class="${properties.kcFormClass!}"
			action="${url.loginAction}"
			method="POST"
		>
			<div class="${properties.kcFormGroupClass!}">
				<div class="${properties.kcLabelWrapperClass!}">
					<label
						for="code"
						class="${properties.kcLabelClass!}"
					>
						${msg("messageOtp.auth.label")}
					</label>
				</div>
				<div class="${properties.kcInputWrapperClass!}">
					<input 
						type="text"
						id="code"
						name="code"
						class="${properties.kcInputClass!}"
						autofocus
					/>
				</div>
			</div>
			<div class="${properties.kcFormGroupClass!} ${properties.kcFormSettingClass!}">
				<div
					id="kc-form-buttons"
					class="${properties.kcFormButtonsClass!}"
				>
					<input
						class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
						type="submit"
						value="${msg("doSubmit")}"
					/>
				</div>
			</div>
		</form>
	<#elseif section = "info" >
		${msg("messageOtp.auth.instructionBoth")}
	</#if>
</@layout.registrationLayout>
