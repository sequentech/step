import {Box, Button} from "@mui/material"
import {styled} from "@mui/material/styles"
import React, {useEffect, useState, useTransition} from "react"
import {useRecordContext} from "react-admin"
import {useTranslation} from "react-i18next"

const LinkButton = styled(Button)(({theme}) => ({
    "minWidth": "auto",
    "minHeight": "auto",
    "background": "none",
    "fontSize": "12px",
    "color": theme.palette.primary.main,
    "border": "none",
    "padding": 0,
    "textDecoration": "underline",
    "boxShadow": "none",
    "cursor": "pointer",
    "&:hover": {
        background: "none",
        padding: 0,
        color: theme.palette.black,
        textDecoration: "underline",
        border: "none",
        boxShadow: "none",
    },
    "&:active": {
        background: "none",
        padding: 0,
        color: theme.palette.primary.main,
        outline: "none",
        border: "none",
        boxShadow: "none",
    },
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
    initialLength?: number
}

export const MessageField: React.FC<MessageFieldProps> = ({initialLength = 256}) => {
    const {t} = useTranslation()
    const base = useRecordContext()
    const [data, setData] = useState<string>("")
    const [more, setMore] = useState<boolean>(false)

    useEffect(() => {
        if (base) {
            setData(base.message)
        }
    }, [base])

    return (
        <Box sx={{width: "100%"}}>
            <Box sx={{overflowWrap: "anywhere"}}>
                {more ? (
                    <span>{data}</span>
                ) : (
                    <>
                        <span>{data.slice(0, initialLength)}</span>
                        <span>{data.length > initialLength ? "..." : ""}</span>
                    </>
                )}
            </Box>
            <Box sx={{display: "flex", justifyContent: "flex-end"}}>
                <LinkButton disableRipple onClick={() => setMore(!more)}>
                    {more
                        ? t("electionEventScreen.common.showLess")
                        : t("electionEventScreen.common.showMore")}
                </LinkButton>
            </Box>
        </Box>
    )
}
