<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageErrorEmailHtmlBody", errorCode))?no_esc}
</@layout.emailLayout>
