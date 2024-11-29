<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messagePendingEmailHtmlBody",realmName ,username))?no_esc}
</@layout.emailLayout>
