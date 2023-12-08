import React from "react"
import {useTranslation} from "react-i18next"
import {ElectionHeaderStyles} from "./styles/ElectionHeaderStyles"

type ElectionHeaderProps = {
    title: string
    subtitle: string
}

const ElectionHeader: React.FC<ElectionHeaderProps> = ({title, subtitle}) => {
    const {t} = useTranslation()

    return (
        <ElectionHeaderStyles.Wrapper>
            <ElectionHeaderStyles.Title>{t(title)}</ElectionHeaderStyles.Title>
            <ElectionHeaderStyles.SubTitle>{t(subtitle)}</ElectionHeaderStyles.SubTitle>
        </ElectionHeaderStyles.Wrapper>
    )
}

export default ElectionHeader
