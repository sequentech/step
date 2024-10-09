<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("registerErrorEmailHtmlBody", error))?no_esc}
</@layout.emailLayout>
