<#import "template.ftl" as layout>
<@layout.emailLayout>
<p>${msg("contactChanged.notice.text", organization, channel)}</p>
</@layout.emailLayout>
