<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

<#import "user-profile-commons.ftl" as userProfileCommons>
<#import "tel-input-widget.ftl" as telInputWidget>
<#import "select-filter-widget.ftl" as selectFilterWidget>

<#function showRequiredMarkers matchAttributes profile honorRequired>
    <#if !honorRequired || !matchAttributes?has_content>
        <#return false>
    </#if>
    <#return matchAttributes?filter(name ->
        profile.attributesByName[name]?? && !profile.attributesByName[name].required)?has_content>
</#function>

<#macro render matchAttributes profile honorRequired showRequiredMarkers credentialFirst fieldError>
    <#assign hasTelMatchAttribute = matchAttributes?filter(name ->
        profile.attributesByName[name]??
        && ((profile.attributesByName[name].annotations.inputType!'') == 'html5-tel'
            || (profile.attributesByName[name].annotations.inputType!'') == 'tel'))?has_content>
    <#assign hasFilteredMatchAttribute = matchAttributes?filter(name ->
        profile.attributesByName[name]??
        && profile.attributesByName[name].annotations.filterSelectAttribute??)?has_content>
    <#if hasTelMatchAttribute><@telInputWidget.assets/></#if>
    <#if hasFilteredMatchAttribute><@selectFilterWidget.assets/></#if>

    <#nested>

    <#list matchAttributes as name>
        <#if profile.attributesByName[name]??>
            <#assign matchAttribute = profile.attributesByName[name]>
            <#assign matchAttributeRequired = honorRequired && matchAttribute.required>
            <@userProfileCommons.inputFieldWithLabel
                attribute=matchAttribute
                name=name
                values=matchAttribute.values
                required=matchAttributeRequired
                requiredMarker=(matchAttributeRequired && showRequiredMarkers)
                autofocus=(!credentialFirst && name?index == 0)
                autocomplete="off"/>
        <#else>
            <#-- Not declared in User Profile: still usable for matching as a required text field. -->
            <div class="${properties.kcFormGroupClass!}">
                <label for="${name}" class="${properties.kcLabelClass!}">${msg(name)}</label><#if showRequiredMarkers> *</#if>
                <input id="${name}" class="${properties.kcInputClass!}" name="${name}" type="text" autocomplete="off"
                       <#if honorRequired>required</#if>
                       <#if !credentialFirst && name?index == 0>autofocus</#if>
                       aria-invalid="<#if fieldError>true</#if>"
                />
            </div>
        </#if>
    </#list>
</#macro>
