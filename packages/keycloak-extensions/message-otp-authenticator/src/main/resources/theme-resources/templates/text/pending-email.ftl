<#ftl output_format="plainText">
<#assign rejectReason = msg(rejectReasonKey)>
${msg("messagePendingEmailTextBody", rejectReason, mismatchedFieldsPlain)}