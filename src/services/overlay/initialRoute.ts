/** Preserve the native window's conversation while normalizing its surface. */
export function initialOverlayRoute(hash: string) {
  const sessionId = new URLSearchParams(hash.split("?")[1] ?? "").get("session");
  return { path: "/overlay", query: sessionId ? { session: sessionId } : {} };
}
