<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageRejectedEmailHtmlBody",realmName ,username))?no_esc}
</@layout.emailLayout>
