import materialIconThemeJson from "./material-icons.json";

export type CodeLanguageInfo = {
  id: string;
  label: string;
  badge: string;
  icon?: string;
  family: "data" | "markup" | "script" | "shell" | "style" | "systems" | "text";
};

type MaterialIconTheme = {
  iconDefinitions: Record<string, { iconPath: string }>;
  fileNames?: Record<string, string>;
  fileExtensions?: Record<string, string>;
};

const materialIconTheme = materialIconThemeJson as MaterialIconTheme;

const LANGUAGE_BY_EXTENSION: Record<string, CodeLanguageInfo> = {
  bash: { id: "bash", label: "Shell", badge: ">_", family: "shell" },
  c: { id: "c", label: "C", badge: "C", family: "systems" },
  cc: { id: "cpp", label: "C++", badge: "C++", family: "systems" },
  cpp: { id: "cpp", label: "C++", badge: "C++", family: "systems" },
  cs: { id: "csharp", label: "C#", badge: "C#", family: "systems" },
  css: { id: "css", label: "CSS", badge: "CSS", family: "style" },
  dart: { id: "dart", label: "Dart", badge: "D", family: "script" },
  go: { id: "go", label: "Go", badge: "GO", family: "systems" },
  h: { id: "c", label: "C header", badge: "H", family: "systems" },
  hpp: { id: "cpp", label: "C++ header", badge: "H++", family: "systems" },
  html: { id: "xml", label: "HTML", badge: "<>", family: "markup" },
  java: { id: "java", label: "Java", badge: "J", family: "systems" },
  js: {
    id: "javascript",
    label: "JavaScript",
    badge: "JS",
    icon: "/file-icons/javascript.svg",
    family: "script",
  },
  json: { id: "json", label: "JSON", badge: "{}", icon: "/file-icons/json.svg", family: "data" },
  jsx: { id: "javascript", label: "JavaScript JSX", badge: "JSX", family: "script" },
  kt: { id: "kotlin", label: "Kotlin", badge: "KT", family: "systems" },
  md: { id: "markdown", label: "Markdown", badge: "MD", family: "text" },
  php: { id: "php", label: "PHP", badge: "PHP", family: "script" },
  ps1: { id: "powershell", label: "PowerShell", badge: ">_", family: "shell" },
  py: { id: "python", label: "Python", badge: "PY", family: "script" },
  rb: { id: "ruby", label: "Ruby", badge: "RB", family: "script" },
  rs: { id: "rust", label: "Rust", badge: "RS", icon: "/file-icons/rust.svg", family: "systems" },
  scss: { id: "scss", label: "SCSS", badge: "SCSS", family: "style" },
  sh: { id: "bash", label: "Shell", badge: ">_", family: "shell" },
  sql: { id: "sql", label: "SQL", badge: "DB", family: "data" },
  svelte: { id: "xml", label: "Svelte", badge: "SV", family: "markup" },
  swift: { id: "swift", label: "Swift", badge: "SW", family: "systems" },
  toml: { id: "ini", label: "TOML", badge: "T", family: "data" },
  ts: {
    id: "typescript",
    label: "TypeScript",
    badge: "TS",
    icon: "/file-icons/typescript.svg",
    family: "script",
  },
  tsx: { id: "typescript", label: "TypeScript JSX", badge: "TSX", family: "script" },
  vue: { id: "xml", label: "Vue", badge: "V", family: "markup" },
  xml: { id: "xml", label: "XML", badge: "<>", family: "markup" },
  yaml: { id: "yaml", label: "YAML", badge: "YML", icon: "/file-icons/yaml.svg", family: "data" },
  yml: { id: "yaml", label: "YAML", badge: "YML", icon: "/file-icons/yaml.svg", family: "data" },
  zig: { id: "", label: "Zig", badge: "Z", icon: "/file-icons/zig.svg", family: "systems" },
};

const LANGUAGE_BY_FILENAME: Record<string, CodeLanguageInfo> = {
  dockerfile: { id: "dockerfile", label: "Dockerfile", badge: "DK", family: "systems" },
  makefile: { id: "makefile", label: "Makefile", badge: "MK", family: "systems" },
};

const FALLBACK_LANGUAGE: CodeLanguageInfo = {
  id: "",
  label: "Code",
  badge: "<>",
  family: "text",
};

export function codeLanguageForPath(path: string): CodeLanguageInfo {
  const filename = path.replace(/\\/g, "/").split("/").pop()?.toLowerCase() ?? "";
  const byFilename = LANGUAGE_BY_FILENAME[filename];
  const extension = filename.includes(".") ? (filename.split(".").pop() ?? "") : "";
  const language = byFilename ?? LANGUAGE_BY_EXTENSION[extension] ?? FALLBACK_LANGUAGE;
  const iconId = findMaterialIconId(filename, extension);
  return {
    ...language,
    icon: iconPath(iconId) ?? language.icon,
  };
}

function findMaterialIconId(filename: string, extension: string) {
  const fileNames = materialIconTheme.fileNames ?? {};
  const fileExtensions = materialIconTheme.fileExtensions ?? {};
  return fileNames[filename] ?? fileExtensions[filename] ?? fileExtensions[extension] ?? undefined;
}

function iconPath(iconId?: string) {
  const iconPath = iconId ? materialIconTheme.iconDefinitions[iconId]?.iconPath : undefined;
  const filename = iconPath?.split("/").pop();
  return filename ? `/file-icons/${filename}` : undefined;
}

import type { Extension } from "@codemirror/state";
export async function codeLanguage(path: string): Promise<Extension> {
  const extension = path.split(/[\\/]/).pop()?.split(".").pop()?.toLowerCase() ?? "";
  const names: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    yml: "yaml",
    md: "markdown",
    rs: "rust",
    py: "python",
    kt: "java",
    h: "cpp",
    cc: "cpp",
    html: "xml",
    svg: "xml",
  };
  const language = names[extension] ?? extension;
  switch (language) {
    case "javascript": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true });
    }
    case "typescript": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true, typescript: true });
    }
    case "json": {
      const { json } = await import("@codemirror/lang-json");
      return json();
    }
    case "xml": {
      const { xml } = await import("@codemirror/lang-xml");
      return xml();
    }
    case "css":
    case "scss": {
      const { css } = await import("@codemirror/lang-css");
      return css();
    }
    case "rust": {
      const { rust } = await import("@codemirror/lang-rust");
      return rust();
    }
    case "python": {
      const { python } = await import("@codemirror/lang-python");
      return python();
    }
    case "yaml": {
      const { yaml } = await import("@codemirror/lang-yaml");
      return yaml();
    }
    case "markdown": {
      const { markdown } = await import("@codemirror/lang-markdown");
      return markdown();
    }
    case "sql": {
      const { sql } = await import("@codemirror/lang-sql");
      return sql();
    }
    case "go": {
      const { go } = await import("@codemirror/lang-go");
      return go();
    }
    case "java": {
      const { java } = await import("@codemirror/lang-java");
      return java();
    }
    case "cpp":
    case "c": {
      const { cpp } = await import("@codemirror/lang-cpp");
      return cpp();
    }
    case "php": {
      const { php } = await import("@codemirror/lang-php");
      return php();
    }
    case "html": {
      const { html } = await import("@codemirror/lang-html");
      return html();
    }
    default:
      return [];
  }
}
