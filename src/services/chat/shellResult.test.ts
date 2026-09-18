import { describe, expect, it } from "vitest";
import { parseShellResult } from "./shellResult";

describe("parseShellResult", () => {
  it("parses exit code, duration, and streams", () => {
    const parsed = parseShellResult(
      "exit_code: 0\nduration: 1.2s\nstdout:\ndom length: 121\nstderr:\n",
    );
    expect(parsed.structured).toBe(true);
    expect(parsed.exitCode).toBe(0);
    expect(parsed.duration).toBe("1.2s");
    expect(parsed.stdout).toBe("dom length: 121");
    expect(parsed.stderr).toBe("");
  });

  it("keeps unstructured output as stdout", () => {
    const parsed = parseShellResult("hello world");
    expect(parsed.structured).toBe(false);
    expect(parsed.stdout).toBe("hello world");
  });
});
