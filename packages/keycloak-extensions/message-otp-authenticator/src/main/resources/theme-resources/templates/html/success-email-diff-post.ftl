<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessDiffPostEmailHtmlBody",realmName ,username))?no_esc}
</@layout.emailLayout>
