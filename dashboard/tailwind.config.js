/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/**/*.{html,js,svelte,ts}"
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        /* Libadwaita dark foundation tokens */
        desktop: "var(--desktop, #1e1d1c)",
        bg: "var(--bg, #252423)",
        surface: "var(--surface, #2d2c2b)",
        raised: "var(--raised, #383635)",
        fg: "var(--fg, #f0eeeb)",
        muted: "var(--muted, #b4b1ad)",
        border: "var(--border, #4c4a48)",
        accent: "var(--accent, #3584e4)",
        ok: "var(--ok, #57ab5a)",
        warn: "var(--warn, #e3b341)",
        danger: "var(--danger, #e01b24)",

        /* Existing semantic colors for backwards compatibility */
        "on-surface-variant": "#d9c3b0",
        "on-background": "#e6e2df",
        "surface-container-low": "#1c1b1a",
        "surface-container-lowest": "#0f0e0d",
        "primary-fixed": "#ffdcbe",
        background: "#141312",
        "on-error": "#690005",
        secondary: "#ffb3ae",
        "tertiary-container": "#00aae2",
        "surface-tint": "#ffb86f",
        error: "#ffb4ab",
        "on-error-container": "#ffdad6",
        "on-tertiary-fixed-variant": "#004c68",
        outline: "#a18d7c",
        "primary-fixed-dim": "#ffb86f",
        "secondary-fixed-dim": "#ffb3ae",
        primary: "#ffb86f",
        "on-tertiary": "#003549",
        "on-primary-container": "#502d00",
        "surface-variant": "#363433",
        "on-secondary": "#68000b",
        "inverse-primary": "#8a5100",
        "on-primary-fixed-variant": "#693c00",
        "surface-container-high": "#2b2a29",
        "on-tertiary-container": "#003a50",
        "on-primary": "#4a2800",
        "inverse-on-surface": "#31302f",
        "tertiary-fixed": "#c3e8ff",
        "on-tertiary-fixed": "#001e2c",
        "tertiary-fixed-dim": "#78d1ff",
        "primary-container": "#e08a1e",
        "on-surface": "#e6e2df",
        "surface-bright": "#3a3938",
        "secondary-container": "#8b191d",
        "surface-dim": "#141312",
        tertiary: "#78d1ff",
        "on-secondary-container": "#ff9992",
        "on-primary-fixed": "#2c1600",
        "on-secondary-fixed-variant": "#8b191d",
        "error-container": "#93000a",
        "on-secondary-fixed": "#410004",
        "outline-variant": "#534435",
        "inverse-surface": "#e6e2df",
        "secondary-fixed": "#ffdad7",
        "surface-container": "#201f1e",
        "surface-container-highest": "#363433"
      },
      borderRadius: {
        radius: "8px",
        "window-radius": "12px",
        DEFAULT: "8px",
        lg: "8px",
        xl: "12px",
        full: "9999px"
      },
      spacing: {
        "space-lg": "1rem",
        "space-xl": "1.5rem",
        "space-sm": "0.5rem",
        "space-xs": "0.25rem",
        "space-md": "0.75rem",
        gutter: "0.75rem",
        margin: "1rem"
      },
      fontFamily: {
        body: [
          "IBM Plex Sans",
          "Cantarell",
          "Adwaita Sans",
          "Noto Sans",
          "sans-serif"
        ],
        mono: [
          "IBM Plex Mono",
          "Noto Sans Mono",
          "ui-monospace",
          "monospace"
        ],
        sans: [
          "IBM Plex Sans",
          "Cantarell",
          "Adwaita Sans",
          "Noto Sans",
          "sans-serif"
        ],
        display: [
          "IBM Plex Sans",
          "Cantarell",
          "Adwaita Sans",
          "Noto Sans",
          "sans-serif"
        ],
        "label-code": [
          "IBM Plex Mono",
          "JetBrains Mono",
          "monospace"
        ],
        "body-default": [
          "IBM Plex Sans",
          "Inter",
          "sans-serif"
        ],
        "headline-md": [
          "IBM Plex Sans",
          "Inter",
          "sans-serif"
        ],
        "headline-lg": [
          "IBM Plex Sans",
          "Inter",
          "sans-serif"
        ],
        keycap: [
          "IBM Plex Mono",
          "JetBrains Mono",
          "monospace"
        ],
        "body-muted": [
          "IBM Plex Sans",
          "Inter",
          "sans-serif"
        ]
      },
      fontSize: {
        "label-code": [
          "12px",
          {
            lineHeight: "16px",
            letterSpacing: "-0.01em",
            fontWeight: "500"
          }
        ],
        "body-default": [
          "14px",
          {
            lineHeight: "20px",
            letterSpacing: "0em",
            fontWeight: "400"
          }
        ],
        "headline-md": [
          "16px",
          {
            lineHeight: "22px",
            letterSpacing: "-0.01em",
            fontWeight: "600"
          }
        ],
        "headline-lg": [
          "20px",
          {
            lineHeight: "26px",
            letterSpacing: "-0.015em",
            fontWeight: "600"
          }
        ],
        keycap: [
          "11px",
          {
            lineHeight: "14px",
            letterSpacing: "0.02em",
            fontWeight: "600"
          }
        ],
        "body-muted": [
          "13px",
          {
            lineHeight: "18px",
            letterSpacing: "0em",
            fontWeight: "400"
          }
        ]
      }
    }
  }
};
