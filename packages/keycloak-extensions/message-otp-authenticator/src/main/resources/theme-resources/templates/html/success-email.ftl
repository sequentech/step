<#import "template.ftl" as layout>
<@layout.emailLayout>
<#if isKiosk?? && isKiosk>
${kcSanitize(msg("messageSuccessEmailHtmlBodyKiosk", realmName ,username, enrollmentUrl, loginUrl))?no_esc}
<#else>
${kcSanitize(msg("messageSuccessEmailHtmlBody", realmName ,username, enrollmentUrl, loginUrl))?no_esc}
</#if>
</@layout.emailLayout>
