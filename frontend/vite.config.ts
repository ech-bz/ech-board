import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [solid(), tailwindcss()],

  base: process.env.VITE_BASE_PATH || '/',

  server: {
    port: 5174,
  },
})

