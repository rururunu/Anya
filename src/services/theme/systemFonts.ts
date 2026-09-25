import { listSystemFonts } from "@/services/ipc";

let installedFontsPromise: Promise<string[]> | undefined;

/** Share one native font query across the three font pickers. */
export function loadSystemFonts(): Promise<string[]> {
  installedFontsPromise ??= listSystemFonts()
    .then((families) => {
      if (!families.length) throw new Error("System font list is empty");
      return families;
    })
    .catch((error) => {
      installedFontsPromise = undefined;
      throw error;
    });
  return installedFontsPromise;
}
