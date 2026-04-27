/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Menlo', 'Monaco', 'Consolas', 'monospace'],
      },
      colors: {
        // Solana-accented palette on a near-black background.
        ink: {
          900: '#0a0a0f',
          800: '#111119',
          700: '#1a1a24',
          600: '#262635',
          500: '#3a3a52',
        },
        accent: {
          purple: '#9945FF',
          teal: '#14F195',
          cyan: '#00D1FF',
        },
      },
      boxShadow: {
        glow: '0 0 60px rgba(153, 69, 255, 0.25)',
      },
    },
  },
  plugins: [],
};
