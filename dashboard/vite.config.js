import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Builds to dashboard/dist, which the API serves as static files.
export default defineConfig({
  plugins: [react()],
  build: { outDir: 'dist' },
})
