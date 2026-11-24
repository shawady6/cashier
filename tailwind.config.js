/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        navy: {
          700: "#1d3557",
          800: "#12284a",
          900: "#0b1b34",
        },
      },
      fontFamily: {
        sans: ['"Segoe UI"', "Tahoma", '"Noto Naskh Arabic"', "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};
