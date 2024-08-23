<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessEmailHtmlBody",username))?no_esc}
</@layout.emailLayout>
