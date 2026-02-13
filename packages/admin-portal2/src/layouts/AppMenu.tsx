import React, { useContext, useState } from 'react';
import { Menu, useSidebarState } from 'react-admin';
import { Box, MenuItem as MMenuItem, Menu as MMenu } from '@mui/material';
import ElectionEvents from '@/components/menu/items/ElectionEvents';
import SelectTenants from '@/components/menu/items/SelectTenants';
import { TenantContext } from '@/providers/TenantContextProvider';
import { AuthContext } from '@/providers/AuthContextProvider';
import { SettingsContext } from '@/providers/SettingsContextProvider';
import { IPermissions } from '@/types/keycloak';
import GroupIcon from '@mui/icons-material/Group';
import SettingsIcon from '@mui/icons-material/Settings';
import MailIcon from '@mui/icons-material/Mail';
import HelpIcon from '@mui/icons-material/Help';
import { useTranslation } from 'react-i18next';

export const AppMenu = () => {
    const { tenant } = useContext(TenantContext);
    const authContext = useContext(AuthContext);
    const [open] = useSidebarState();
    const { t, i18n } = useTranslation();
    const { globalSettings } = useContext(SettingsContext);
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);

    const showUsers = authContext.isAuthorized(true, authContext.tenantId, IPermissions.USERS_MENU);
    const showSettings = authContext.isAuthorized(true, authContext.tenantId, IPermissions.SETTINGS_MENU);
    const showTemplates = authContext.isAuthorized(true, authContext.tenantId, IPermissions.TEMPLATES_MENU);

    const openInNewTab = (url: string) => {
        setAnchorEl(null);
        let replacedUrl = url.replace("${PUBLIC_BUCKET_URL}", globalSettings.PUBLIC_BUCKET_URL);
        window.open(replacedUrl, "_blank", "noopener,noreferrer");
    };

    return (
        <Box sx={{
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'space-between',
            pb: 2
        }}>
            <Box>
                <SelectTenants />
                <Box sx={{ mb: 2 }} />
                <ElectionEvents />

                {tenant && showUsers && (
                    <Menu.Item
                        to="/user-roles"
                        primaryText={t('sideMenu.usersAndRoles')}
                        leftIcon={<GroupIcon />}
                    />
                )}
                {tenant && showSettings && (
                    <Menu.Item
                        to="/settings"
                        primaryText={t('sideMenu.settings')}
                        leftIcon={<SettingsIcon />}
                    />
                )}
                {tenant && showTemplates && (
                    <Menu.Item
                        to="/sequent_backend_template"
                        primaryText={t('sideMenu.templates')}
                        leftIcon={<MailIcon />}
                    />
                )}
            </Box>

             {tenant?.settings?.help_links?.length > 0 && (
                <Box>
                    <Menu.Item
                        to="#"
                        primaryText={t("sideMenu.help")}
                        leftIcon={<HelpIcon />}
                        onClick={(e) => {
                            e.preventDefault();
                            setAnchorEl(e.currentTarget);
                        }}
                    />
                    <MMenu
                        id="menu-sidebar"
                        anchorEl={anchorEl}
                        anchorOrigin={{
                            vertical: "bottom",
                            horizontal: "left",
                        }}
                        keepMounted
                        transformOrigin={{
                            vertical: "top",
                            horizontal: "right",
                        }}
                        open={Boolean(anchorEl)}
                        onClose={() => setAnchorEl(null)}
                    >
                        {tenant?.settings?.help_links?.map((i: any) => {
                            return (
                                <MMenuItem
                                    key={i.url}
                                    onClick={() => openInNewTab(i.url)}
                                >
                                    <Box
                                        sx={{
                                            textOverflow: "ellipsis",
                                            whiteSpace: "nowrap",
                                            overflow: "hidden",
                                        }}
                                    >
                                        <span title={i.title}>
                                            {i.i18n?.[i18n.language]?.title ?? i.title}
                                        </span>
                                    </Box>
                                </MMenuItem>
                            );
                        })}
                    </MMenu>
                </Box>
            )}
        </Box>
    );
};
