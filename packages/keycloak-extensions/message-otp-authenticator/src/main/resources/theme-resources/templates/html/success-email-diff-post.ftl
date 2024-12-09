<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessDiffPostEmailHtmlBody",realmName ,username ,embassy, enrollmentUrl, loginUrl))?no_esc}
</@layout.emailLayout>
