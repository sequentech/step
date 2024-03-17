<#import "template.ftl" as layout>
<@layout.registrationLayout ; section>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <span>${msg(error)}</span>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
