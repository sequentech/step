import { createTheme, ThemeOptions, alpha } from '@mui/material/styles';
import { defaultTheme } from 'react-admin';

// Define brand colors
const brandColor = '#0F054C';
const secondaryColor = '#43E3A1';
const backgroundColor = '#f8f9fa';

const themeOptions: ThemeOptions = {
    ...defaultTheme,
    palette: {
        mode: 'light',
        primary: {
            main: brandColor,
            light: alpha(brandColor, 0.8),
            dark: '#0a0333',
            contrastText: '#ffffff',
        },
        secondary: {
            main: secondaryColor,
            contrastText: '#000000',
        },
        background: {
            default: backgroundColor,
            paper: '#ffffff',
        },
        text: {
            primary: '#1a1a1a',
            secondary: '#6c757d',
        },
    },
    typography: {
        fontFamily: '"Inter", "Roboto", "Helvetica", "Arial", sans-serif',
        h1: { fontWeight: 700 },
        h2: { fontWeight: 600 },
        h3: { fontWeight: 600 },
        h4: { fontWeight: 600 },
        h5: { fontWeight: 500 },
        h6: { fontWeight: 500 },
        button: { textTransform: 'none', fontWeight: 600 },
    },
    shape: {
        borderRadius: 8,
    },
    components: {
        ...defaultTheme.components,
        MuiButton: {
            styleOverrides: {
                root: {
                    boxShadow: 'none',
                    '&:hover': {
                        boxShadow: 'none',
                    },
                },
                contained: {
                    borderRadius: 8,
                },
                outlined: {
                    borderRadius: 8,
                }
            },
        },
        MuiPaper: {
            styleOverrides: {
                root: {
                    backgroundImage: 'none',
                },
                elevation1: {
                    boxShadow: '0px 2px 4px -1px rgba(0,0,0,0.05), 0px 4px 5px 0px rgba(0,0,0,0.01), 0px 1px 10px 0px rgba(0,0,0,0.01)',
                },
            },
        },
        MuiAppBar: {
            styleOverrides: {
                root: {
                    boxShadow: '0px 1px 0px rgba(0,0,0,0.05)',
                    backgroundColor: '#ffffff',
                    color: '#1a1a1a',
                },
            },
        },
        MuiTableCell: {
            styleOverrides: {
                head: {
                    fontWeight: 600,
                    backgroundColor: '#f9fafb',
                    color: '#4b5563',
                    borderBottom: '1px solid #e5e7eb',
                },
                root: {
                    borderBottom: '1px solid #f3f4f6',
                },
            },
        },
        // React Admin overrides
        RaLayout: {
            styleOverrides: {
                root: {
                    '& .RaLayout-content': {
                        padding: 24,
                    },
                },
            },
        },
        RaSidebar: {
            styleOverrides: {
                root: {
                    height: '100vh',
                    borderRight: '1px solid #e0e0e0',
                    '& .MuiDrawer-paper': {
                         borderRight: 'none',
                    }
                },
            },
        },
        RaMenuItemLink: {
            styleOverrides: {
                root: {
                    borderRadius: 8,
                    margin: '4px 8px',
                    '&.RaMenuItemLink-active': {
                        backgroundColor: alpha(brandColor, 0.08),
                        color: brandColor,
                        fontWeight: 600,
                        '& .MuiSvgIcon-root': {
                            color: brandColor,
                        },
                    },
                },
            },
        },
    },
};

export const appTheme = createTheme(themeOptions);
