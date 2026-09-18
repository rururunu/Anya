/** Parsed `run_shell` / foreground shell tool result. */
export type ShellResultParts = {
  exitCode: number | null;
  duration: string | null;
  stdout: string;
  stderr: string;
  /** True when the payload matched `exit_code:` / `stdout:` / `stderr:` layout. */
  structured: boolean;
};

/** Split shell tool output into exit code, duration, and streams. */
export function parseShellResult(raw: string): ShellResultParts {
  const text = raw.trimEnd();
  if (!text) {
    return { exitCode: null, duration: null, stdout: "", stderr: "", structured: false };
  }

  const exitMatch = text.match(/^exit_code:\s*(-?\d+)\s*$/m);
  const durationMatch = text.match(/^duration:\s*(.+)\s*$/m);
  const hasStdoutHeader = /^stdout:\s*$/m.test(text);
  const hasStderrHeader = /^stderr:\s*$/m.test(text);

  if (!exitMatch && !hasStdoutHeader && !hasStderrHeader) {
    return {
      exitCode: null,
      duration: null,
      stdout: text,
      stderr: "",
      structured: false,
    };
  }

  let stdout = "";
  let stderr = "";
  if (hasStdoutHeader || hasStderrHeader) {
    const stdoutIdx = text.search(/^stdout:\s*$/m);
    const stderrIdx = text.search(/^stderr:\s*$/m);
    if (stdoutIdx >= 0 && stderrIdx > stdoutIdx) {
      const afterStdout = text.indexOf("\n", stdoutIdx);
      stdout = text
        .slice(afterStdout + 1, stderrIdx)
        .replace(/^\n/, "")
        .replace(/\n$/, "");
      const afterStderr = text.indexOf("\n", stderrIdx);
      stderr = afterStderr >= 0 ? text.slice(afterStderr + 1) : "";
    } else if (stdoutIdx >= 0) {
      const afterStdout = text.indexOf("\n", stdoutIdx);
      stdout = afterStdout >= 0 ? text.slice(afterStdout + 1) : "";
    } else if (stderrIdx >= 0) {
      const afterStderr = text.indexOf("\n", stderrIdx);
      stderr = afterStderr >= 0 ? text.slice(afterStderr + 1) : "";
    }
  }

  return {
    exitCode: exitMatch ? Number(exitMatch[1]) : null,
    duration: durationMatch?.[1]?.trim() || null,
    stdout,
    stderr,
    structured: true,
  };
}
