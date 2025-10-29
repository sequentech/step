// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ExpandableText} from "@sequentech/ui-essentials"
import React, {useEffect, useState} from "react"
import {useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"

type MessageFieldProps = {
    source?: string
    content?: string | undefined
    initialLength?: number
}

/**
 * This component renders a string field with a show more/show less toggle.
 * The field is rendered as a text field with an ellipsis at the end when the string is longer than the initial length.
 * By default, the initial length is 256 characters.
 * When the user clicks on the field, the show more/show less toggle is displayed.
 * When the user clicks on the toggle, the field is rendered in its entirety or shortened to the initial length.
 * The toggle text is translated using the keys "electionEventScreen.common.showMore" and "electionEventScreen.common.showLess".
 * @param {MessageFieldProps} props
 * @param {string} [props.source] - The source of the data for the field if it is in the record context.
 * @param {string} [props.content] - The content of the field if is rendered directly from parent.
 * @param {number} [props.initialLength=256] - The initial length of the field.
 * @returns {ReactElement}
 */
export const MessageField: React.FC<MessageFieldProps> = ({
    source,
    content,
    initialLength = 256,
}) => {
    const {t} = useTranslation()
    const base = useRecordContext()
    const [data, setData] = useState<string>("")

    useEffect(() => {
        if (base) {
            setData(content ? content : source ? base[source] : "")
        }
    }, [base, source, content])

    return (
        <ExpandableText
            text={data}
            initialLength={initialLength}
            showMoreLabel={t("electionEventScreen.common.showMore")}
            showLessLabel={t("electionEventScreen.common.showLess")}
        />
    )
}
