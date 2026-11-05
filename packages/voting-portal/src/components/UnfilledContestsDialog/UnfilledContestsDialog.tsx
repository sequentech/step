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
    @media (max-width: 600px) {
        .MuiDialogActions-root {
            flex-direction: column;
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
    margin: 16px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
`

const ContestName = styled(Box)`
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
            variant="softwarning"
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
