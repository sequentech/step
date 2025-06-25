<#ftl output_format="plainText">
<#assign rejectReason = msg(rejectReasonKey)>
${msg("messageRejectedEmailTextBody", rejectReason, missmatchedFields)}