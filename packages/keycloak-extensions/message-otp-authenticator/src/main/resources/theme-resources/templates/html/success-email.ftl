<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessEmailHtmlBody",enrollmentUrl, realmName ,username))?no_esc}
</@layout.emailLayout>
