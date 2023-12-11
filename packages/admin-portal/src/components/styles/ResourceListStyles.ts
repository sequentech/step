import styled from "@emotion/styled"
import {Box} from "@mui/material"
import {IconButton} from "@sequentech/ui-essentials"

export const ResourceListStyles = {
    EmptyBox: styled(Box)`
        display: flex;
        margin: 1em;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        text-align: center;
        width: 100%;
    `,
    CreateIcon: styled(IconButton)`
        font-size: 24px;
        margin-right: 0.5em;
    `,
}
