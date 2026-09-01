// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETranslationScope} from "@sequentech/ui-core"
import {TFunction} from "i18next"
import React from "react"
import {required, SelectInput} from "react-admin"
import {useTranslation} from "react-i18next"

const scopeFallbackLabels: Record<ETranslationScope, string> = {
    [ETranslationScope.GLOBAL]: "Global",
    [ETranslationScope.VOTING_PORTAL]: "Voting portal",
    [ETranslationScope.BALLOT_VERIFIER]: "Ballot verifier",
    [ETranslationScope.RESULTS_PORTAL]: "Results portal",
    [ETranslationScope.ADMIN_PORTAL]: "Admin portal",
}

export const translationScopeLabel = (
    t: TFunction,
    scope: ETranslationScope | undefined,
    legacyScope: ETranslationScope
): string => {
    if (!scope) {
        const legacyPortalLabel = String(
            t(`electionEventScreen.localization.scopes.${legacyScope}`, {
                defaultValue: scopeFallbackLabels[legacyScope],
            })
        )
        return String(
            t("electionEventScreen.localization.scopes.legacy", {
                defaultValue: `Legacy (${scopeFallbackLabels[legacyScope]})`,
                portal: legacyPortalLabel,
            })
        )
    }

    return String(
        t(`electionEventScreen.localization.scopes.${scope}`, {
            defaultValue: scopeFallbackLabels[scope],
        })
    )
}

interface TranslationScopeInputProps {
    allowedScopes: readonly ETranslationScope[]
    defaultValue: ETranslationScope
    source: string
}

export const TranslationScopeInput: React.FC<TranslationScopeInputProps> = ({
    allowedScopes,
    defaultValue,
    source,
}) => {
    const {t} = useTranslation()
    const choices = allowedScopes.map((scope) => ({
        id: scope,
        name: translationScopeLabel(t, scope, defaultValue),
    }))

    return (
        <SelectInput
            source={source}
            label={String(
                t("electionEventScreen.localization.labels.scope", {defaultValue: "Portal scope"})
            )}
            choices={choices}
            validate={required()}
            defaultValue={defaultValue}
            fullWidth
        />
    )
}
