// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ReactNode} from "react"
import type {PageProps} from "keycloakify/login/pages/PageProps"
import type {I18n} from "../i18n"
import type {KcContext} from "../KcContext"
import type {SequentTemplateProps} from "../Template"

// The identity verification pages use the Template's progress, symbol and layouts.
export type ScanovatePageProps<PageId extends KcContext["pageId"]> = Omit<
    PageProps<Extract<KcContext, {pageId: PageId}>, I18n>,
    "Template"
> & {Template: (props: SequentTemplateProps) => ReactNode}
