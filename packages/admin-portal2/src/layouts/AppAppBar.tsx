import React, { useContext, useEffect, useState } from 'react';
import { AppBar, useGetOne } from 'react-admin';
import { Box } from '@mui/material';
import { Header } from '@sequentech/ui-essentials';
import { AuthContext } from '@/providers/AuthContextProvider';
import { SettingsContext } from '@/providers/SettingsContextProvider';
import { TenantContext } from '@/providers/TenantContextProvider';
import { Sequent_Backend_Tenant } from '@/gql/graphql';
import SequentLogo from '@sequentech/ui-essentials/public/Sequent_logo.svg';
import BlankLogoImg from '@sequentech/ui-essentials/public/blank_logo.svg';
import { ITenantSettings, ITenantTheme } from '@sequentech/ui-core';

export const AppAppBar = () => {
    const authContext = useContext(AuthContext);
    const { globalSettings } = useContext(SettingsContext);
    const { tenantId, tenant, setTenant } = useContext(TenantContext);
    const { data: tenantData } = useGetOne<Sequent_Backend_Tenant>('sequent_backend_tenant', {
        id: tenantId,
    });

    const [isFetching, setIsFetching] = useState(true);

    useEffect(() => {
        if (tenantData) {
            setTenant(tenantData);
            setIsFetching(false);
        }
    }, [tenantData, setTenant]);

    const langList = (tenant?.settings as ITenantSettings | undefined)?.language_conf
        ?.enabled_language_codes ?? ['en'];

    const [logoUrl, setLogoUrl] = useState<string | undefined | null>(
        (tenant?.annotations as ITenantTheme | undefined)?.logo_url
    );

    const [logoImg, setLogoImg] = useState<string | undefined>(BlankLogoImg);

    useEffect(() => {
        setLogoImg(logoUrl ?? BlankLogoImg);
    }, [logoUrl]);

    useEffect(() => {
        const newLogoState = (tenant?.annotations as ITenantTheme | undefined)?.logo_url;
        setLogoUrl(newLogoState);
        if (!isFetching) {
            setLogoImg(newLogoState ?? SequentLogo);
        }
    }, [(tenant?.annotations as ITenantTheme | undefined)?.logo_url, logoUrl, isFetching]);

    return (
        <AppBar
            toolbar={<></>}
            sx={{
                '& .RaAppBar-toolbar': {
                    padding: 0,
                },
                backgroundColor: 'background.paper',
                color: 'text.primary',
                boxShadow: 'none',
                borderBottom: '1px solid #E5E7EB',
            }}
        >
            <Box width="100%">
                 <Header
                    appVersion={{ main: globalSettings.APP_VERSION }}
                    appHash={{ main: globalSettings.APP_HASH }}
                    userProfile={{
                        firstName: authContext.firstName,
                        username: authContext.username,
                        email: authContext.email,
                        openLink: authContext.openProfileLink,
                    }}
                    logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
                    languagesList={langList}
                    logoUrl={logoImg}
                />
            </Box>
        </AppBar>
    );
};
