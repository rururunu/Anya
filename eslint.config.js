import js from "@eslint/js";
import globals from "globals";
import pluginVue from "eslint-plugin-vue";
import tseslint from "typescript-eslint";
import prettier from "eslint-config-prettier";
import pluginImport from "eslint-plugin-import";
import { fileURLToPath } from "node:url";
import path from "node:path";

const rootDir = path.dirname(fileURLToPath(import.meta.url));

/** @type {import('eslint').Linter.Config[]} */
export default [
  {
    ignores: [
      "dist/**",
      "src-tauri/**",
      "node_modules/**",
      "eval/**",
      "scripts/**",
      "public/**",
      ".anya/**",
      "*.msi",
      "coverage/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs["flat/recommended"],
  {
    files: ["**/*.{ts,vue,js,mjs,cjs}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      import: pluginImport,
    },
    settings: {
      "import/resolver": {
        typescript: {
          alwaysTryTypes: true,
          project: path.join(rootDir, "tsconfig.json"),
          noWarnOnMultipleProjects: true,
        },
        node: true,
      },
    },
    rules: {
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "no-undef": "off",
      "prefer-const": "warn",
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-empty-object-type": "warn",
      "@typescript-eslint/ban-ts-comment": "warn",
      "vue/multi-word-component-names": "off",
      "vue/require-default-prop": "off",
      "vue/attribute-hyphenation": "off",
      "vue/no-required-prop-with-default": "off",
      "vue/no-template-shadow": "warn",
      "vue/no-v-html": "warn",
      "vue/attributes-order": "off",
      "vue/html-self-closing": "off",
      "vue/max-attributes-per-line": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/multiline-html-element-content-newline": "off",
      "import/no-duplicates": "warn",
      "import/order": "off",
    },
  },
  {
    files: ["**/*.vue"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".vue"],
      },
    },
  },
  // Layer boundaries — stores/services must not import UI.
  {
    files: ["src/stores/**/*.{ts,vue}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["@/components", "@/components/*", "@/pages", "@/pages/*", "@/layouts", "@/layouts/*"],
              message: "stores must not import UI layers (components/pages/layouts).",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["src/services/**/*.{ts,vue}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: [
                "@/stores",
                "@/stores/*",
                "@/components",
                "@/components/*",
                "@/pages",
                "@/pages/*",
                "@/layouts",
                "@/layouts/*",
              ],
              message: "services must not import stores or UI layers.",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["src/types/**/*.{ts,vue}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: [
                "@/stores",
                "@/stores/*",
                "@/services",
                "@/services/*",
                "@/components",
                "@/components/*",
                "@/pages",
                "@/pages/*",
                "@/composables",
                "@/composables/*",
              ],
              message: "types must stay free of business/runtime imports.",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["src/services/logger.ts"],
    rules: {
      "no-console": "off",
    },
  },
  {
    files: ["src/components/**/*.{ts,vue}", "src/stores/**/*.{ts,vue}"],
    rules: {
      "max-lines": ["warn", { max: 800, skipBlankLines: true, skipComments: true }],
    },
  },
  prettier,
];
