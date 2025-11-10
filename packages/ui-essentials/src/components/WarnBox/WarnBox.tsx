import React, {PropsWithChildren} from "react"
import {styled} from "@mui/material/styles"
import Paper from "@mui/material/Paper"
import Box from "@mui/material/Box"
import styledEmotion from "@emotion/styled"

const WarnContainer = styled(Paper)`
    padding: 17px;
    display: flex;
    flex-direction: row;
    gap: 8px;
    border-radius: 4px;
    line-height: 19px;
    align-items: center;
`

const SomeBox = styledEmotion(Box)`
    opacity: 0.7 !important;
`

interface WarnBoxProps {
    onClose?: () => void
}

const WarnBox: React.FC<PropsWithChildren<WarnBoxProps>> = ({onClose, children}) => (
    <WarnContainer>
        <SomeBox>Emotion</SomeBox>
        <Box flexGrow={2}>{children}</Box>
    </WarnContainer>
)

export default WarnBox
