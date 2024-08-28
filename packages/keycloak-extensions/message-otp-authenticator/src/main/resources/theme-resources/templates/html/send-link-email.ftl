<#--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#import "template.ftl" as layout>
<@layout.emailLayout>
${kcSanitize(msg("messageOtp.sendLink.email.htmlBody",realmName,code,ttl))?no_esc}
</@layout.emailLayout>
