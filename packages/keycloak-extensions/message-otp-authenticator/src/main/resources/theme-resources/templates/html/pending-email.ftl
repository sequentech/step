<#import "template.ftl" as layout>
<@layout.emailLayout>
<#assign rejectReason = msg(rejectReasonKey)>
${kcSanitize(msg("messagePendingEmailHtmlBody", rejectReason, mismatchedFieldsHtml))?no_esc}
</@layout.emailLayout>
