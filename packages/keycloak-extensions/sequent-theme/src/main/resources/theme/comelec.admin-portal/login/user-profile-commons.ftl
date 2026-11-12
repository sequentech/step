<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#--  Source: https://github.com/keycloak/keycloak/blob/24.0.0/themes/src/main/resources/theme/base/login/user-profile-commons.ftl  -->

<#macro userProfileFormFields>
	<#assign currentGroup="">
	<#assign readonlyElements = []>
	<#assign disabledElements = []>
	
	<#list profile.attributes as attribute>
		<#--  Check for default custom attribute and assign it the first time the form is opened  -->
		<#if attribute.values?has_content>
			<#assign values = attribute.values>
		<#elseif attribute.annotations.default?has_content>
			<#assign values = attribute.annotations.default>
		<#else>
			<#assign values = []>
		</#if>

		<#assign group = (attribute.group)!"">
		<#if group != currentGroup>
			<#assign currentGroup=group>
			<#if currentGroup != "">
				<div class="${properties.kcFormGroupClass!}"
				<#list group.html5DataAnnotations as key, value>
					data-${key}="${value}"
				</#list>
				>
	
					<#assign groupDisplayHeader=group.displayHeader!"">
					<#if groupDisplayHeader != "">
						<#assign groupHeaderText=advancedMsg(groupDisplayHeader)!group>
					<#else>
						<#assign groupHeaderText=group.name!"">
					</#if>
					<div class="${properties.kcContentWrapperClass!}">
						<label id="header-${attribute.group.name}" class="${kcFormGroupHeader!}">${groupHeaderText}</label>
					</div>
	
					<#assign groupDisplayDescription=group.displayDescription!"">
					<#if groupDisplayDescription != "">
						<#assign groupDescriptionText=advancedMsg(groupDisplayDescription)!"">
						<div class="${properties.kcLabelWrapperClass!}">
							<label id="description-${group.name}" class="${properties.kcLabelClass!}">${groupDescriptionText}</label>
						</div>
					</#if>
				</div>
			</#if>
		</#if>

		<#nested "beforeField" attribute>
		<#if attribute.annotations.hidden?? && attribute.annotations.hidden?matches("true")>
		<#else>
			<@inputFieldWithLabel attribute=attribute name=attribute.name values=values/>
			<#if attribute.annotations.confirm??>
				<@inputFieldWithLabel attribute=attribute name=attribute.name+'-confirm' values=values/>
			</#if>
		</#if>
		<#nested "afterField" attribute>
	</#list>

	<script>
		// Disable field function. Turns inputs into read only. Add a disableAttribute annotation to a select or multiselect user profile attribute.
		function readOnlyElementById(e, idToSetReadOnly) {
			e = e || window.event;
			var target = e.target || e.srcElement;

			setAllReadOnly(idToSetReadOnly, target.checked);
		}

		// Disable field function using disabled. Add a disableElement annotation to a select or multiselect user profile attribute.
		function disableElementById(e, idToDisable) {
			e = e || window.event;
			var target = e.target || e.srcElement;

			setAllDisabled(idToDisable, target.checked);
		}

		function setReadOnly(id, value) {
			let element = document.getElementById(id);

			if (element) {
				element.readOnly = !value;
				element.required = value;
				if (!value) {
					element.value = '';
				}
			}
		}

		function setAllReadOnly(id, value) {
			setReadOnly(id, value);
            // In case of using hidden inputs for int-tel input
			setReadOnly(id + "-input", value);
			// In case of having confirm inputs
			setReadOnly(id + "-confirm", value);
		}

		function setAllDisabled(id, checked) {
			let element = document.getElementById(id);

			if (element) {
				element.disabled = !checked;
			}
		}
	document.addEventListener('DOMContentLoaded', function() {
		<#list readonlyElements as element>
			setAllReadOnly("${element.id}", ${element.checked});
		</#list>
		<#list disabledElements as element>
			setAllDisabled("${element.id}", ${element.checked});
		</#list>
	}, false);

	</script>

	<#list profile.html5DataAnnotations?keys as key>
		<script type="module" src="${url.resourcesPath}/js/${key}.js"></script>
	</#list>
</#macro>

<#macro inputFieldWithLabel attribute name values>
	<div class="${properties.kcFormGroupClass!}">
		<div class="${properties.kcLabelWrapperClass!}">
			<label for="${name}" class="${properties.kcLabelClass!}">
				<#if name?ends_with("-confirm")>
					${advancedMsg(attribute.annotations.confirm!'')}
				<#else>
					${advancedMsg(attribute.displayName!'')}
				</#if>
			</label>
			<#if attribute.required>*</#if>
		</div>
		<div class="${properties.kcInputWrapperClass!}">
			<#if attribute.annotations.inputHelperTextBefore??>
				<div class="${properties.kcInputHelperTextBeforeClass!}" id="form-help-text-before-${name}" aria-live="polite">${kcSanitize(advancedMsg(attribute.annotations.inputHelperTextBefore))?no_esc}</div>
			</#if>
			<@inputFieldByType attribute=attribute name=name values=values/>
			<#if messagesPerField.existsError('${name}')>
				<span id="input-error-${name}" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
					${kcSanitize(messagesPerField.get('${name}'))?no_esc}
				</span>
			</#if>
			<#if attribute.annotations.inputHelperTextAfter??>
				<div class="${properties.kcInputHelperTextAfterClass!}" id="form-help-text-after-${name}" aria-live="polite">${kcSanitize(advancedMsg(attribute.annotations.inputHelperTextAfter))?no_esc}</div>
			</#if>
		</div>
	</div>
</#macro>

<#macro inputFieldByType attribute name values>
	<#switch attribute.annotations.inputType!''>
	<#case 'textarea'>
		<@textareaTag attribute=attribute name=name values=values/>
		<#break>
	<#case 'select'>
	<#case 'multiselect'>
		<@selectTag attribute=attribute name=name values=values/>
		<#break>
	<#case 'select-radiobuttons'>
	<#case 'multiselect-checkboxes'>
		<@inputTagSelects attribute=attribute name=name values=values/>
		<#break>
	<#default>
		<#if attribute.multivalued && values?has_content>
			<#list values as value>
				<@inputTag attribute=attribute name=name value=value!''/>
			</#list>
		<#else>
			<@inputTag attribute=attribute name=name value=attribute.value!''/>
		</#if>
	</#switch>
</#macro>

<#macro inputTag attribute name value>
	<input type="<@inputTagType attribute=attribute/>" id="${name}" name="${name}" value="${(value!'')}" class="${properties.kcInputClass!}"
		aria-invalid="<#if messagesPerField.existsError('${name}')>true</#if>"
		<#if attribute.readOnly>disabled</#if>
		<#--  Checks for attribute annotations that start with "html-attribute:" and sets them as input attributes  -->
		<#list attribute.annotations as key, value>
			<#if key?starts_with("html-attribute:")>${key[15..]}=${value}</#if>
		</#list>
		<#if attribute.annotations.inputTypePlaceholder??>placeholder="${attribute.annotations.inputTypePlaceholder}"</#if>
		<#if attribute.annotations.inputTypePattern??>pattern="${attribute.annotations.inputTypePattern}"</#if>
		<#if attribute.annotations.inputTypeSize??>size="${attribute.annotations.inputTypeSize}"</#if>
		<#if attribute.annotations.inputTypeMaxlength??>maxlength="${attribute.annotations.inputTypeMaxlength}"</#if>
		<#if attribute.annotations.inputTypeMinlength??>minlength="${attribute.annotations.inputTypeMinlength}"</#if>
		<#if attribute.annotations.inputTypeMax??>max="${attribute.annotations.inputTypeMax}"</#if>
		<#if attribute.annotations.inputTypeMin??>min="${attribute.annotations.inputTypeMin}"</#if>
		<#if attribute.annotations.inputTypeStep??>step="${attribute.annotations.inputTypeStep}"</#if>
		<#if attribute.annotations.inputTypeStep??>step="${attribute.annotations.inputTypeStep}"</#if>
		<#list attribute.html5DataAnnotations as key, value>
    		data-${key}="${value}"
		</#list>
	/>
</#macro>

<#macro inputTagType attribute>
	<#compress>
	<#if attribute.annotations.inputType??>
		<#if attribute.annotations.inputType?starts_with("html5-")>
			${attribute.annotations.inputType[6..]}
		<#else>
			${attribute.annotations.inputType}
		</#if>
	<#else>
	text
	</#if>
	</#compress>
</#macro>

<#macro textareaTag attribute name values>
	<textarea id="${name}" name="${name}" class="${properties.kcInputClass!}"
		aria-invalid="<#if messagesPerField.existsError('${name}')>true</#if>"
		<#if attribute.readOnly>disabled</#if>
		<#if attribute.annotations.inputTypeCols??>cols="${attribute.annotations.inputTypeCols}"</#if>
		<#if attribute.annotations.inputTypeRows??>rows="${attribute.annotations.inputTypeRows}"</#if>
		<#if attribute.annotations.inputTypeMaxlength??>maxlength="${attribute.annotations.inputTypeMaxlength}"</#if>
	>${(attribute.value!'')}</textarea>
</#macro>

<#macro selectTag attribute name values>
	<select id="${name}" name="${name}" class="${properties.kcInputClass!}"
		aria-invalid="<#if messagesPerField.existsError('${name}')>true</#if>"
		<#if attribute.readOnly>disabled</#if>
		<#if attribute.annotations.inputType=='multiselect'>multiple</#if>
		<#if attribute.annotations.inputTypeSize??>size="${attribute.annotations.inputTypeSize}"</#if>
		<#if attribute.annotations.filterSelectAttribute??>onchange="filterSelectAttribute(event, '${attribute.annotations.filterSelectAttribute}')"</#if>
	>
	<#if attribute.annotations.inputType=='select'>
		<option value=""></option>
	</#if>

	<#if attribute.annotations.inputOptionsFromValidation?? && attribute.validators[attribute.annotations.inputOptionsFromValidation]?? && attribute.validators[attribute.annotations.inputOptionsFromValidation].options??>
		<#assign options=attribute.validators[attribute.annotations.inputOptionsFromValidation].options>
	<#elseif attribute.validators.options?? && attribute.validators.options.options??>
		<#assign options=attribute.validators.options.options>
	<#else>
		<#assign options=[]>
	</#if>

	<#list options as option>
		<option value="${option}" <#if values?seq_contains(option)>selected</#if>><@selectOptionLabelText attribute=attribute option=option/></option>
	</#list>

	</select>
</#macro>

<#macro inputTagSelects attribute name values>
	<#if attribute.annotations.inputType=='select-radiobuttons'>
		<#assign inputType='radio'>
		<#assign classDiv=properties.kcInputClassRadio!>
		<#assign classInput=properties.kcInputClassRadioInput!>
		<#assign classLabel=properties.kcInputClassRadioLabel!>
	<#else>
		<#assign inputType='checkbox'>
		<#assign classDiv=properties.kcInputClassCheckbox!>
		<#assign classInput=properties.kcInputClassCheckboxInput!>
		<#assign classLabel=properties.kcInputClassCheckboxLabel!>
	</#if>

	<#if attribute.annotations.inputOptionsFromValidation?? && attribute.validators[attribute.annotations.inputOptionsFromValidation]?? && attribute.validators[attribute.annotations.inputOptionsFromValidation].options??>
		<#assign options=attribute.validators[attribute.annotations.inputOptionsFromValidation].options>
	<#elseif attribute.validators.options?? && attribute.validators.options.options??>
		<#assign options=attribute.validators.options.options>
	<#else>
		<#assign options=[]>
	</#if>

	<#list options as option>
		<div class="${classDiv}">
			<input type="${inputType}" id="${name}-${option}" name="${name}" value="${option}" class="${classInput}"
				aria-invalid="<#if messagesPerField.existsError('${name}')>true</#if>"
				<#if attribute.readOnly>disabled</#if>
				<#if values?seq_contains(option)>checked</#if>
				<#if attribute.annotations.disableAttribute??>onclick="readOnlyElementById(event, '${option}')"</#if>
				<#if attribute.annotations.disableElement??>onclick="disableElementById(event, '${option}')"</#if>
			/>
			<label for="${name}-${option}" class="${classLabel}<#if attribute.readOnly> ${properties.kcInputClassRadioCheckboxLabelDisabled!}</#if>"><@selectOptionLabelText attribute=attribute option=option/></label>
		</div>
		<#if attribute.annotations.disableAttribute??>
		<#assign readonlyElements += [{"id":"${option}","checked":"${values?seq_contains(option)?c}"}]>
		</#if>
		<#if attribute.annotations.disableElement??>
		<#assign disabledElements += [{"id":"${option}","checked":"${values?seq_contains(option)?c}"}]>
		</#if>
	</#list>
</#macro>

<#macro selectOptionLabelText attribute option>
	<#compress>
	<#if attribute.annotations.inputOptionLabels??>
		${advancedMsg(attribute.annotations.inputOptionLabels[option]!option)}
	<#else>
		<#if attribute.annotations.inputOptionLabelsI18nPrefix??>
			${msg(attribute.annotations.inputOptionLabelsI18nPrefix + '.' + option)}
		<#else>
			${option}
		</#if>
	</#if>
	</#compress>
</#macro>
