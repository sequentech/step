/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["./src/**/*.{js,jsx,ts,tsx}"],
    theme: {
        extend: {
            colors: {
                "light-background": "#F7F9FE",
                "brand-color": "#0F054C",
                "brand-success": "#43E3A1",
                "error-color": "#DC2626",
                "red": {
                    light: "#FECACA",
                    main: "#991B1B",
                    dark: "#991B1B",
                },
                "green": {
                    light: "#ECFDF5",
                    main: "#064E3B",
                    dark: "#047857",
                },
                "customGreen": {
                    light: "#CFF0DC",
                    main: "#0EB048",
                    dark: "#0EB048",
                },
                "yellow": {
                    light: "#FFF7D9",
                    main: "#837032",
                    dark: "#837032",
                },
                "blue": {
                    light: "#CCE5FF",
                    main: "#292F99",
                    dark: "#292F99",
                },
                "customGrey": {
                    light: "#E7EAEE",
                    main: "#757575",
                    dark: "#64748B",
                    contrastText: "#191D23",
                },
                "extraGrey": {
                    main: "#B8C0CC",
                },
                "white": "white",
                "black": "black",
            },
        },
    },
    plugins: [],
}
