// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {SlateDialog} from "../SlateDialog/SlateDialog"
import {translate} from "@sequentech/ui-core"
import {IUnfilledContest} from "../../services/UnfilledContests"

const ContestList = styled("ul")`
    list-style: none;
    margin: 22px 0 0;
    padding: 0;

    .unfilled-contest {
        padding: 12px 0;
        border-bottom: 1px solid #e7eaee;
        font-size: 14px;
        line-height: 1.5;
    }

    .unfilled-contest:last-of-type {
        border-bottom: none;
    }

    .unfilled-contest-count {
        color: ${({theme}) => theme.palette.customGrey.dark};
    }
`

const ContestName = styled(Box)`
    margin-bottom: 5px;
    font-weight: bold;
    overflow-wrap: anywhere;
`

interface UnfilledContestsDialogProps {
    open: boolean
    unfilledContests: Array<IUnfilledContest>
    handleClose: (value: boolean) => void
}

const UnfilledContestsDialog: React.FC<UnfilledContestsDialogProps> = ({
    open,
    unfilledContests,
    handleClose,
}) => {
    const {t, i18n} = useTranslation()

    return (
        <SlateDialog
            className="unfilled-contests-dialog"
            handleClose={handleClose}
            open={open}
            title={t("reviewScreen.unfilledContestsDialog.title")}
            ok={t("reviewScreen.unfilledContestsDialog.ok")}
            cancel={t("reviewScreen.unfilledContestsDialog.cancel")}
        >
            <Typography
                className="unfilled-contests-content"
                sx={{
                    margin: "0 0 12px",
                    fontSize: "inherit",
                    lineHeight: "inherit",
                    color: "customGrey.dark",
                }}
            >
                {t("reviewScreen.unfilledContestsDialog.content")}
            </Typography>
            <ContestList className="unfilled-contests-list">
                {unfilledContests.map(({contest, selected, max}) => (
                    <li key={contest.id} className="unfilled-contest">
                        <ContestName className="unfilled-contest-name">
                            {translate(contest, "name", i18n.language)}
                        </ContestName>
                        <Box className="unfilled-contest-count">
                            {t("reviewScreen.unfilledContestsDialog.selected", {selected, max})}
                            {selected === 0
                                ? ` · ${t("reviewScreen.unfilledContestsDialog.nothingSelected")}`
                                : null}
                        </Box>
                    </li>
                ))}
            </ContestList>
        </SlateDialog>
    )
}

export default UnfilledContestsDialog
