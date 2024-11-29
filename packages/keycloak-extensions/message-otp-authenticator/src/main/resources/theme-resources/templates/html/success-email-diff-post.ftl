<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessDiffPostEmailHtmlBody",realmName ,username ,embassy))?no_esc}
</@layout.emailLayout>
