import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("shared staged queue window permissions", () => {
  it("allows every overlay queue operation including enqueue, guide and auto dispatch", () => {
    const capability = JSON.parse(readFileSync("src-tauri/capabilities/overlay.json", "utf8"));
    const permissions = readFileSync("src-tauri/permissions/remote.toml", "utf8");
    expect(capability.permissions).toContain("allow-staged-commands");
    const staged = permissions.split('identifier = "allow-staged-commands"')[1];
    for (const command of ["list", "push", "remove", "replace", "clear", "insert", "pop", "take"]) {
      expect(staged).toContain(`"remote_${command}_staged"`);
    }
  });
});
