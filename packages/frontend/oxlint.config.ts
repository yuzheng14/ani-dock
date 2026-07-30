import { defineConfig } from "oxlint"

export default defineConfig({
  plugins: [
    "react",
    "eslint",
    "typescript",
    "unicorn",
    "import",
    "jsdoc",
    "react-perf",
    "oxc",
    "promise",
  ],
})
