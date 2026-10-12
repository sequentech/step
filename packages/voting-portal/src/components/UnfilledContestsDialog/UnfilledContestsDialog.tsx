// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {styled} from "@mui/material/styles"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {translate} from "@sequentech/ui-core"
import {IUnfilledContest} from "../../services/UnfilledContests"

// Both answers are sentences, so on a phone they take a row each.
const StackedActionsDialog = styled(Dialog)`
    @media (max-width: 749.95px) {
        .MuiDialogContent-root {
            padding: 16px 20px 0 20px !important;
        }
        .MuiDialogActions-root {
            padding: 15px 20px 20px;
        }
    }
    @media (max-width: 600px) {
        .MuiDialogActions-root {
            flex-direction: column-reverse;
            gap: 8px;
        }
        .MuiDialogActions-root > .MuiButtonBase-root {
            width: 100%;
            margin: 0;
        }
    }
`

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
        <StackedActionsDialog
            className="unfilled-contests-dialog"
            handleClose={handleClose}
            open={open}
            title={t("reviewScreen.unfilledContestsDialog.title")}
            ok={t("reviewScreen.unfilledContestsDialog.ok")}
            cancel={t("reviewScreen.unfilledContestsDialog.cancel")}
            variant="info"
            maxWidth="sm"
        >
            <Typography className="unfilled-contests-content" margin={0}>
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
        </StackedActionsDialog>
    )
}

export default UnfilledContestsDialog
