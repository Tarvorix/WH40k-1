/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Faction colors
        custodes: {
          gold: '#C8A951',
          dark: '#8B6914',
          light: '#E5D48B',
        },
        worldeaters: {
          red: '#8B0000',
          dark: '#5C0000',
          light: '#CC3333',
        },
        // Phase colors
        phase: {
          command: '#4A90D9',
          movement: '#50C878',
          shooting: '#E8A317',
          charge: '#CC5500',
          fight: '#DC143C',
          prebattle: '#9B59B6',
          gameend: '#7F8C8D',
        },
        // UI colors
        surface: {
          DEFAULT: '#1a1a2e',
          light: '#242444',
          lighter: '#2e2e5e',
        },
        accent: {
          DEFAULT: '#C8A951',
          muted: '#8B6914',
        },
      },
      fontFamily: {
        heading: ['"Cinzel"', 'serif'],
        body: ['"Inter"', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'monospace'],
      },
    },
  },
  plugins: [],
};
