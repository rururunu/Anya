import { describe, expect, it } from "vitest";
import { selectDefaultChatModel } from "./ensureDefaultModel";

const cached = [{ id: "first-model", provider: "provider-a", ownedBy: "A" }];
describe("saved default model", () => {
  it("does not overwrite a saved model missing from the startup cache", () => {
    expect(selectDefaultChatModel(cached, "chosen-model", "provider-b")?.needsPersist).toBe(false);
  });
  it("does not overwrite a saved provider when the same model belongs to another provider", () => {
    expect(selectDefaultChatModel(cached, "first-model", "provider-b")?.needsPersist).toBe(false);
  });
  it("initializes a default only when no model has been configured", () => {
    expect(selectDefaultChatModel(cached, "", "")?.needsPersist).toBe(true);
    expect(selectDefaultChatModel([], "chosen-model", "provider-b")).toBeNull();
  });
});
