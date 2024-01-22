import React, { useEffect } from "react"
import {useTranslation} from "react-i18next"
import {ElectionHeaderStyles} from "./styles/ElectionHeaderStyles"

type ElectionHeaderProps = {
    title: string
    subtitle: string
}

const ElectionHeader: React.FC<ElectionHeaderProps> = ({title, subtitle}) => {
    const {t, i18n} = useTranslation()

    useEffect(() => {
        const dir = i18n.dir(i18n.language)
        document.documentElement.dir = dir
    }, [i18n, i18n.language])
    
    return (
        <ElectionHeaderStyles.Wrapper dir={i18n.dir(i18n.language)}>
            <ElectionHeaderStyles.Title dir={i18n.dir(i18n.language)}>
                {t(title)}
            </ElectionHeaderStyles.Title>
            <ElectionHeaderStyles.SubTitle dir={i18n.dir(i18n.language)}>
                {t(subtitle)}
            </ElectionHeaderStyles.SubTitle>
        </ElectionHeaderStyles.Wrapper>
    )
}

export default ElectionHeader
