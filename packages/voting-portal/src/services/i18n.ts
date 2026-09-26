// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ETranslationScope, initializeLanguages} from "@sequentech/ui-core"
import {votingPortalTranslations} from "../translations"
import {getLanguageFromURL} from "../utils/queryParams"

const language = getLanguageFromURL()

initializeLanguages(votingPortalTranslations, language, ETranslationScope.VOTING_PORTAL)
