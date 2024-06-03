export const getLanguageFromURL = () => {
    const params = new URLSearchParams(window.location.search);
    return params.get('lang') || undefined;
};