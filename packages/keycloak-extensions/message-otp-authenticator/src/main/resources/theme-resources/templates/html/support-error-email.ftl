<#ftl output_format="HTML">
<#import "template.ftl" as layout>
<@layout.emailLayout>
    <p>${msg("messageSupportErrorEmailHtmlBody")?no_esc}</p>
    
    <p>Event Details:</p>
    <#if event.details?? && (event.details?size > 0)>
        <ul>
        <#list event.details?keys as key>
            <li>${key}: ${event.details[key]!}</li>
        </#list>
        </ul>
    <#else>
        <p>No additional details provided.</p>
    </#if>
</@layout.emailLayout>
