import React, {PropsWithChildren} from "react"
import {styled} from "@mui/material/styles"
import Paper from "@mui/material/Paper"
import Box from "@mui/material/Box"

const WarnContainer = styled(Paper)`
    padding: 17px;
    display: flex;
    flex-direction: row;
    gap: 8px;
    border-radius: 4px;
    line-height: 19px;
    align-items: center;
`

interface WarnBoxProps {
    onClose?: () => void
}

const WarnBox: React.FC<PropsWithChildren<WarnBoxProps>> = ({onClose, children}) => (
    <WarnContainer>
        <Box flexGrow={2}>{children}</Box>
    </WarnContainer>
)

export default WarnBox
