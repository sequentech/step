<#import "template.ftl" as layout>
<@layout.registrationLayout ; section>
    <#if section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <span>${msg(error)}</span>
                <p>If the problem persist, please refer in the help desk to identificator 11421 for manual resolution.</p>
            </div>
        </div>
    </#if>
</@layout.registrationLayout>
