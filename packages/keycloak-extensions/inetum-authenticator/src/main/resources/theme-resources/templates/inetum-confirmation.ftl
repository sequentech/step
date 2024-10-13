<#--
    SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
    <#import "template.ftl" as layout>
        <@layout.registrationLayout displayMessage=false; section>
            <#if section="form">
                <h2>Please confirm the following data:</h2>
                <ul>
                    <#list storedAttributes as key, value>
                        <li>
                            ${key}: ${value}
                        </li>
                    </#list>
                </ul>
                <form action="${actionUrl}" method="post">
                    <button name="action" value="confirm" type="submit">Confirm</button>
                    <button onclick="location.reload(); return false;">Retry</button>
                </form>
            </#if>
        </@layout.registrationLayout>