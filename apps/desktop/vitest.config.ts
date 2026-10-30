import { defineConfig } from "vitest/config";

// Kept apart from vite.config.ts so tests don't load the React and Tailwind plugins.
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    testTimeout: 60_000,
  },
});
