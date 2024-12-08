<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageSuccessEmailHtmlBody", realmName ,username, enrollmentUrl, loginUrl))?no_esc}
</@layout.emailLayout>
