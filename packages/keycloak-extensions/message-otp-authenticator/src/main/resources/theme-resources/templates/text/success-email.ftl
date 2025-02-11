<#ftl output_format="plainText">
<#if isKiosk?? && isKiosk>
  ${msg("messageSuccessEmailTextBodyKiosk", realmName, username, enrollmentUrl, loginUrl)}
<#else>
  ${msg("messageSuccessEmailTextBody", realmName, username, enrollmentUrl, loginUrl)}
</#if>