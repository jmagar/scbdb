import js from "@eslint/js";
import tsParser from "@typescript-eslint/parser";
import tsPlugin from "@typescript-eslint/eslint-plugin";
import reactHooks from "eslint-plugin-react-hooks";
import jsxA11y from "eslint-plugin-jsx-a11y";
import globals from "globals";

export default [
  // Ignore build output and generated files.
  {
    ignores: ["dist/**", "node_modules/**"],
  },
  js.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
      },
      globals: {
        ...globals.browser,
        ...globals.es2021,
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
      "react-hooks": reactHooks,
      "jsx-a11y": jsxA11y,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      ...jsxA11y.configs.recommended.rules,
      // Disabled: eslint-plugin-jsx-a11y@6.x has a minimatch compat bug with ESLint 10
      // that causes a runtime crash in this rule. All label elements in this codebase
      // correctly associate controls via nesting (the pattern is valid a11y).
      "jsx-a11y/label-has-associated-control": "off",
      // Disable base rule — @typescript-eslint/no-unused-vars handles TS files correctly,
      // including type-only parameter names in function type signatures.
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_" },
      ],
    },
  },
  // Vite config runs in Node — allow process and other Node globals.
  {
    files: ["vite.config.ts"],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
];
