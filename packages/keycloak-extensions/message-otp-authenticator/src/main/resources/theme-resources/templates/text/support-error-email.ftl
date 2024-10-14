<#ftl output_format="plainText">
${msg("messageSupportErrorEmailTextBody")}

Event Details:
<#if event.details?? && (event.details?size > 0)>
<#list event.details?keys as key>
- ${key}: ${event.details[key]!}
</#list>
<#else>
- No additional details provided.
</#if>
