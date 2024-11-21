<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageOtp.sendCode.email.htmlBody",realmName,code,ttl))?no_esc}
</@layout.emailLayout>
