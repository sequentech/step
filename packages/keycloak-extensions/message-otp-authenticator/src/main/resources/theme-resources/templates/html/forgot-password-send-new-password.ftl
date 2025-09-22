<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("newPassword.email.htmlBody",realmName,temporaryPassword))?no_esc}
</@layout.emailLayout>
