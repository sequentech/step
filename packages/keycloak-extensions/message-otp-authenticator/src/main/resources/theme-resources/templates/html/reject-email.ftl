<#import "template.ftl" as layout>
<@layout.emailLayout>
<#assign rejectReason = msg(rejectReasonKey)>
${kcSanitize(msg("messageRejectedEmailHtmlBody", rejectReason, missmatchedFields))?no_esc}
</@layout.emailLayout>
