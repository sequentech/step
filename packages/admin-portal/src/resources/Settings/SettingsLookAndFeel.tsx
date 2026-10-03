// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {SimpleForm, TextInput, useEditController, Toolbar, SaveButton, useNotify} from "react-admin"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IHelpLink, ITenantSettings, ITenantTheme} from "@sequentech/ui-core"
import {IPermissions} from "@/types/keycloak"

/** The organization's name in messages that name it (`settings.display_name`). */
interface ITenantDisplayName {
    display_name?: string
}

/**
 * The settings to save with an edited display name: trimmed, or removed when
 * emptied (the tenant's slug is shown instead). Unedited, they stay as read.
 */
const withDisplayName = (
    settings: Record<string, unknown>,
    displayName: string | undefined
): Record<string, unknown> => {
    if (displayName === undefined) return settings
    const rest = {...settings}
    delete rest.display_name
    const trimmed = displayName.trim()
    return trimmed ? {...rest, display_name: trimmed} : rest
}

export const SettingsLookAndFeel: React.FC<void> = () => {
    const [tenantId] = useTenantStore()
    const {t, i18n} = useTranslation()
    const notify = useNotify()
    const authContext = useContext(AuthContext)

    const {record, save, isLoading} = useEditController({
        resource: "sequent_backend_tenant",
        id: tenantId,
        redirect: false,
        undoable: false,
    })

    const canEdit = authContext.isAuthorized(true, authContext.tenantId, IPermissions.TENANT_WRITE)

    const [logoUrl, setLogoUrl] = useState<string | undefined>(
        (record?.annotations as ITenantTheme | undefined)?.logo_url
    )

    const [cssContent, setCssContent] = useState<string | undefined>(
        (record?.annotations as ITenantTheme | undefined)?.css
    )

    const [helpLinks, setHelpLinks] = useState<Array<IHelpLink>>(
        (record?.settings as ITenantSettings | undefined)?.help_links ?? []
    )

    // Undefined until edited, so saving other fields keeps the saved name.
    const [displayName, setDisplayName] = useState<string | undefined>(undefined)

    const [saveDisabled, setSaveDisabled] = useState<boolean>(false)

    useEffect(() => {
        if (saveDisabled) {
            setSaveDisabled(false)
        }
    }, [logoUrl, cssContent, helpLinks, displayName])

    const handleHelpLinksChange = (
        event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
    ) => {
        try {
            const parsedHelpLinks =
                !event.target.value || event.target.value.trim().length === 0
                    ? []
                    : JSON.parse(event.target.value)

            if (Array.isArray(parsedHelpLinks)) {
                setHelpLinks(parsedHelpLinks as Array<IHelpLink>)
            } else {
                notify(t("lookAndFeelScreen.errors.invalidHelpLinks"), {type: "error"})
            }
        } catch (error) {
            notify(t("lookAndFeelScreen.errors.invalidHelpLinks"), {type: "error"})
        }
    }

    const onSave = async () => {
        const logoUrlToSave = logoUrl === "" ? null : logoUrl
        const cssContentToSave = cssContent === "" ? null : cssContent

        console.log("onSave logoUrl: ", logoUrlToSave)
        console.log("onSave cssContent: ", cssContentToSave)
        console.log("onSave helpLinks: ", helpLinks)
        save!({
            annotations: {
                ...((record?.annotations as ITenantTheme | undefined) ?? {}),
                logo_url: logoUrlToSave,
                css: cssContentToSave,
            },
            settings: withDisplayName(
                {
                    ...(record?.settings ?? {}),
                    help_links: helpLinks,
                },
                displayName
            ),
        })
        setSaveDisabled(true)
    }

    if (isLoading) return null

    return (
        <SimpleForm
            toolbar={
                <Toolbar>
                    {canEdit ? (
                        <SaveButton
                            onClick={() => {
                                onSave()
                            }}
                            type="button"
                            disabled={saveDisabled}
                        />
                    ) : null}
                </Toolbar>
            }
        >
            <TextInput
                resettable={true}
                source={"annotations.logo_url"}
                defaultValue={logoUrl}
                label={String(t("lookAndFeelScreen.common.logoUrl"))}
                onBlur={(event) => setLogoUrl(event.target.value)}
            />
            <TextInput
                resettable={true}
                multiline={true}
                source={"annotations.css"}
                defaultValue={cssContent}
                label={String(t("lookAndFeelScreen.common.css"))}
                onBlur={(event) => setCssContent(event.target.value)}
            />
            <TextInput
                resettable={true}
                source={"settings.display_name"}
                defaultValue={(record?.settings as ITenantDisplayName | undefined)?.display_name}
                label={String(t("lookAndFeelScreen.common.displayName"))}
                helperText={String(t("lookAndFeelScreen.common.displayNameHelp"))}
                onBlur={(event) => setDisplayName(event.target.value)}
            />
            <TextInput
                resettable={true}
                multiline={true}
                source={"settings.help_links"}
                defaultValue={JSON.stringify(helpLinks, null, 2)}
                label={String(t("lookAndFeelScreen.common.helpLinks"))}
                onBlur={handleHelpLinksChange}
            />
        </SimpleForm>
    )
}
