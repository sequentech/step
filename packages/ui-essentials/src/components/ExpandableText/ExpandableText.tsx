// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Box, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import React, {useState} from "react"

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

export type ExpandableTextProps = {
    text: string
    initialLength?: number
    showMoreLabel: string
    showLessLabel: string
    preformatted?: boolean
}

/**
 * This component renders a text field with a show more/show less toggle.
 * The field is rendered as a text field with an ellipsis at the end when the text is longer than the initial length.
 * By default, the initial length is 256 characters.
 * When the user clicks on the toggle, the field is rendered in its entirety or shortened to the initial length.
 * @param {ExpandableTextProps} props
 * @param {string} props.text - The text content to display
 * @param {number} [props.initialLength=256] - The initial length of the field.
 * @param {string} props.showMoreLabel - The label for the "show more" button
 * @param {string} props.showLessLabel - The label for the "show less" button
 * @param {boolean} [props.preformatted=false] - Whether to preserve whitespace and line breaks
 * @returns {ReactElement}
 */
const ExpandableText: React.FC<ExpandableTextProps> = ({
    text,
    initialLength = 256,
    showMoreLabel,
    showLessLabel,
    preformatted = false,
}) => {
    const [expanded, setExpanded] = useState<boolean>(false)

    return (
        <Box sx={{width: "100%"}}>
            <Box
                sx={{
                    flex: 1,
                    overflowWrap: "anywhere",
                    whiteSpace: preformatted ? "pre" : "normal",
                }}
            >
                {expanded ? (
                    <span>{text}</span>
                ) : (
                    <>
                        <span>{text.slice(0, initialLength)}</span>
                        <span>{text.length > initialLength ? "..." : ""}</span>
                    </>
                )}
            </Box>
            {text.length > initialLength && (
                <Box sx={{display: "flex", justifyContent: "flex-end"}}>
                    <LinkButton disableRipple onClick={() => setExpanded(!expanded)}>
                        {expanded ? showLessLabel : showMoreLabel}
                    </LinkButton>
                </Box>
            )}
        </Box>
    )
}

export default ExpandableText
