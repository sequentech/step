// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import Button from "@mui/material/Button"
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome"
import {Box} from "@mui/material"
import {faArrowRightFromBracket} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Dialog from "../Dialog/Dialog"

export interface LogoutButtonProps {
    logoutFn: () => void
}

const LogoutButton: React.FC<LogoutButtonProps> = ({logoutFn}) => {
    const {t} = useTranslation()
    const [open, setOpen] = useState(false)
    const handleClose = (value: boolean) => (value ? logoutFn() : setOpen(false))

    return (
        <>
            <Button variant="actionbar" onClick={() => setOpen(true)}>
                <FontAwesomeIcon icon={faArrowRightFromBracket} size="lg" />
                <Box component="span" sx={{display: {xs: "none", md: "block"}}}>
                    {t("logout.buttonText")}
                </Box>
            </Button>
            <Dialog
                handleClose={handleClose}
                open={open}
                title={t("logout.modal.title")}
                ok={t("logout.modal.ok")}
                cancel={t("logout.modal.close")}
                variant="action"
            >
                <p>{t("logout.modal.content")}</p>
            </Dialog>
        </>
    )
}

export default LogoutButton
