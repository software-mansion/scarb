/** @type {import("tailwindcss").Config} */
module.exports = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx}",
    "./theme.config.tsx",
  ],
  theme: {
    extend: {
      fontFamily: {
        humanist: [
          "Seravek",
          "'Gill Sans Nova'",
          "Ubuntu",
          "Calibri",
          "'DejaVu Sans'",
          "source-sans-pro",
          "sans-serif",
        ],
      },
    },
    colors: {
      inherit: "inherit",
      current: "currentColor",
      transparent: "transparent",
      black: {
        DEFAULT: "#000",
        40: "#30354A",
        60: "#272B3C",
        80: "#232736",
      },
      white: {
        DEFAULT: "#fff",
        20: "#FCFCFF",
        40: "#F8F9FF",
        60: "#EEF0FF",
        80: "#C1C6E5",
      },
      blue: {
        20: "#C1C6E5",
        40: "#919FCF",
        60: "#6676AA",
        80: "#33488E",
        100: "#001A72",
      },
      darkblue: {
        20: "#ABBCF5",
        40: "#7485BD",
        60: "#0A2688",
        80: "#001A72",
        100: "#122154",
        120: "#1B2445",
      },
      sea: {
        20: "#E1F3FA",
        40: "#B5E1F1",
        60: "#87CCE8",
        80: "#5BB9E0",
        100: "#38ACDD",
      },
      darksea: {
        20: "#D7F0FA",
        40: "#A8DBF0",
        60: "#6FCEF5",
        80: "#00A9F0",
        100: "#126893",
        120: "#1B4865",
      },
      yellow: {
        20: "#FFFAE1",
        40: "#FFF1B2",
        60: "#FFE780",
        80: "#FFE04B",
        100: "#FFD61E",
      },
      red: {
        20: "#FFEDF0",
        40: "#FFD2D7",
        60: "#FFA3A1",
        80: "#FA7F7C",
        100: "#FF6259",
      },
      green: {
        20: "#EBFCF7",
        40: "#DFF2EC",
        60: "#B1DFD0",
        80: "#82CAB2",
        100: "#57B495",
      },
    },
  },
  plugins: [],
  darkMode: "class",
};
