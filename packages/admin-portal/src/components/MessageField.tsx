// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import React, {useEffect, useState} from "react"
import {useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"

/**
 * A styled MUI Button that looks like a link.
 * The button will have no background, no border, no padding, and an underline.
 * The text will be black and the decoration will be an underline.
 * The button will have no box shadow.
 * The button will have a pointer cursor.
 * On hover, the button will have a black color and an underline decoration.
 * On active, the button will have a primary color and no decoration.
 * On focus, the button will have a primary color and an underline decoration.
 */
const LinkButton = styled(Button)(({theme}) => ({
    "minWidth": "auto",
    "minHeight": "auto",
    "background": "none",
    "fontSize": "12px",
    "color": theme.palette.primary.main,
    "border": "none",
    "padding": 0,
    "textDecoration": "none",
    "boxShadow": "none",
    "cursor": "pointer",
    /**
     * On hover, the button will have a black color and an underline decoration.
     */
    "&:hover": {
        background: "none",
        padding: 0,
        color: theme.palette.black,
        textDecoration: "underline",
        border: "none",
        boxShadow: "none",
    },
    /**
     * On active, the button will have a primary color and no decoration.
     */
    "&:active": {
        background: "none",
        padding: 0,
        color: theme.palette.primary.main,
        outline: "none",
        border: "none",
        boxShadow: "none",
    },
    /**
     * On focus, the button will have a primary color and an underline decoration.
     */
    "&:focus": {
        background: "none",
        padding: 0,
        color: theme.palette.primary.main,
        boxShadow: "none",
        border: "none",
        outline: "none",
    },
}))

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
    const [more, setMore] = useState<boolean>(false)

    useEffect(() => {
        if (base) {
            setData(content ? content : source ? base[source] : "")
        }
    }, [base, source, content])

    return (
        <Box sx={{width: "100%"}}>
            <Box sx={{flex: 1, overflowWrap: "anywhere"}}>
                {more ? (
                    <span>{data}</span>
                ) : (
                    <>
                        <span>{data.slice(0, initialLength)}</span>
                        <span>{data.length > initialLength ? "..." : ""}</span>
                    </>
                )}
            </Box>
            {data.length > initialLength && (
                <Box sx={{display: "flex", justifyContent: "flex-end"}}>
                    <LinkButton disableRipple onClick={() => setMore(!more)}>
                        {more
                            ? t("electionEventScreen.common.showLess")
                            : t("electionEventScreen.common.showMore")}
                    </LinkButton>
                </Box>
            )}
        </Box>
    )
}
