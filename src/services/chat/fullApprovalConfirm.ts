import { tr } from "@/services/i18n";
import type { AppLanguage } from "@/types/setting";

/** Confirm dialog copy when switching into AlwaysAllow / 完全批准. */
export function fullApprovalConfirmOptions(language: AppLanguage) {
  return {
    title: tr(language, "fullApprovalConfirmTitle"),
    description: tr(language, "fullApprovalConfirmDescription"),
    confirmLabel: tr(language, "fullApprovalConfirmAction"),
    cancelLabel: tr(language, "rewindCancel"),
    tone: "danger" as const,
  };
}
