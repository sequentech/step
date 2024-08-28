<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessEmailHtmlBody",realmName ,username))?no_esc}
</@layout.emailLayout>
