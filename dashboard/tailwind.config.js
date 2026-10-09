/** @type {import('tailwindcss').Config} */
export default {
  "content": [
    "./src/**/*.{html,js,svelte,ts}"
  ],
  "darkMode": "class",
  "theme": {
    "extend": {
      "colors": {
        "on-surface-variant": "#d9c3b0",
        "on-background": "#e6e2df",
        "surface-container-low": "#1c1b1a",
        "surface-container-lowest": "#0f0e0d",
        "primary-fixed": "#ffdcbe",
        "background": "#141312",
        "on-error": "#690005",
        "secondary": "#ffb3ae",
        "tertiary-container": "#00aae2",
        "surface-tint": "#ffb86f",
        "error": "#ffb4ab",
        "on-error-container": "#ffdad6",
        "on-tertiary-fixed-variant": "#004c68",
        "outline": "#a18d7c",
        "primary-fixed-dim": "#ffb86f",
        "secondary-fixed-dim": "#ffb3ae",
        "primary": "#ffb86f",
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
        "tertiary": "#78d1ff",
        "on-secondary-container": "#ff9992",
        "on-primary-fixed": "#2c1600",
        "on-secondary-fixed-variant": "#8b191d",
        "error-container": "#93000a",
        "on-secondary-fixed": "#410004",
        "outline-variant": "#534435",
        "inverse-surface": "#e6e2df",
        "surface": "#141312",
        "secondary-fixed": "#ffdad7",
        "surface-container": "#201f1e",
        "surface-container-highest": "#363433"
      },
      "borderRadius": {
        "DEFAULT": "0.25rem",
        "lg": "0.5rem",
        "xl": "0.75rem",
        "full": "9999px"
      },
      "spacing": {
        "space-lg": "1rem",
        "space-xl": "1.5rem",
        "space-sm": "0.5rem",
        "space-xs": "0.25rem",
        "space-md": "0.75rem",
        "gutter": "0.75rem",
        "margin": "1rem"
      },
      "fontFamily": {
        "label-code": [
          "JetBrains Mono"
        ],
        "body-default": [
          "Inter"
        ],
        "headline-md": [
          "Inter"
        ],
        "headline-lg": [
          "Inter"
        ],
        "keycap": [
          "JetBrains Mono"
        ],
        "body-muted": [
          "Inter"
        ]
      },
      "fontSize": {
        "label-code": [
          "12px",
          {
            "lineHeight": "16px",
            "letterSpacing": "-0.01em",
            "fontWeight": "500"
          }
        ],
        "body-default": [
          "14px",
          {
            "lineHeight": "20px",
            "letterSpacing": "0em",
            "fontWeight": "400"
          }
        ],
        "headline-md": [
          "16px",
          {
            "lineHeight": "22px",
            "letterSpacing": "-0.01em",
            "fontWeight": "600"
          }
        ],
        "headline-lg": [
          "20px",
          {
            "lineHeight": "26px",
            "letterSpacing": "-0.015em",
            "fontWeight": "600"
          }
        ],
        "keycap": [
          "11px",
          {
            "lineHeight": "14px",
            "letterSpacing": "0.02em",
            "fontWeight": "600"
          }
        ],
        "body-muted": [
          "13px",
          {
            "lineHeight": "18px",
            "letterSpacing": "0em",
            "fontWeight": "400"
          }
        ]
      }
    }
  }
};
