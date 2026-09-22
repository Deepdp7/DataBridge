/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // DataBridge design system — dark trading terminal palette
        bg: {
          DEFAULT: 'hsl(220, 13%, 9%)',
          surface: 'hsl(220, 13%, 13%)',
          elevated: 'hsl(220, 13%, 17%)',
          hover: 'hsl(220, 13%, 20%)',
        },
        border: {
          DEFAULT: 'hsl(220, 13%, 20%)',
          subtle: 'hsl(220, 13%, 16%)',
          strong: 'hsl(220, 13%, 28%)',
        },
        text: {
          primary: 'hsl(220, 13%, 95%)',
          secondary: 'hsl(220, 10%, 70%)',
          muted: 'hsl(220, 10%, 50%)',
          disabled: 'hsl(220, 10%, 35%)',
        },
        accent: {
          DEFAULT: 'hsl(210, 100%, 60%)',
          hover: 'hsl(210, 100%, 68%)',
          muted: 'hsl(210, 60%, 25%)',
        },
        status: {
          green: 'hsl(142, 76%, 48%)',
          'green-muted': 'hsl(142, 60%, 20%)',
          yellow: 'hsl(45, 96%, 54%)',
          'yellow-muted': 'hsl(45, 60%, 20%)',
          red: 'hsl(0, 84%, 60%)',
          'red-muted': 'hsl(0, 60%, 22%)',
          blue: 'hsl(210, 100%, 60%)',
          'blue-muted': 'hsl(210, 60%, 20%)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Cascadia Code', 'Consolas', 'monospace'],
      },
      fontSize: {
        '2xs': ['10px', { lineHeight: '14px' }],
        xs: ['11px', { lineHeight: '16px' }],
        sm: ['12px', { lineHeight: '18px' }],
        base: ['13px', { lineHeight: '20px' }],
        md: ['14px', { lineHeight: '20px' }],
        lg: ['16px', { lineHeight: '24px' }],
        xl: ['18px', { lineHeight: '28px' }],
      },
      spacing: {
        '13': '3.25rem',
        '15': '3.75rem',
        '18': '4.5rem',
        '22': '5.5rem',
      },
      borderRadius: {
        sm: '3px',
        DEFAULT: '4px',
        md: '6px',
        lg: '8px',
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'fade-in': 'fadeIn 0.15s ease-in',
        'slide-in': 'slideIn 0.15s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideIn: {
          '0%': { transform: 'translateX(-4px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
}
