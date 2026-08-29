<template>
  <ArtifactRows v-if="rows.length" :rows="rows" />
</template>

<script setup lang="ts">
import { computed } from "vue";
import { File, FileDiff } from "@lucide/vue";
import ArtifactRows, { type ArtifactRow } from "@/components/chat/ArtifactRows.vue";
import { extractCodeChanges } from "@/services/chat/codeChanges";
import { fileBasename } from "@/services/chat/toolDiff";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, SharedFileOffer } from "@/types/chat";

const props = withDefaults(
  defineProps<{
    message: ChatMessage;
    canUndo?: boolean;
    busy?: boolean;
  }>(),
  {
    canUndo: false,
    busy: false,
  },
);

const emit = defineEmits<{
  undo: [];
  review: [];
  reviewFile: [path: string];
}>();

const settingStore = useSettingStore();
const changes = computed(() => extractCodeChanges([props.message]));
const changedPaths = computed(() => new Set(changes.value.map((change) => change.path)));

const sharedFiles = computed(() => {
  if (props.message.sharedFiles?.length) return props.message.sharedFiles;
  return filesFromActivities(props.message).filter((file) => !changedPaths.value.has(file.path));
});

const rows = computed((): ArtifactRow[] => {
  const language = settingStore.language;
  const items: ArtifactRow[] = [];

  if (changes.value.length === 1) {
    const change = changes.value[0]!;
    items.push({
      key: `change-${change.id}`,
      icon: FileDiff,
      label: fileBasename(change.path),
      title: change.path,
      stats: { added: change.added, removed: change.removed },
      actionLabel: tr(language, "reviewChanges"),
      onOpen: () => emit("reviewFile", change.path),
      onAction: () => emit("review"),
    });
  } else if (changes.value.length > 1) {
    const totals = changes.value.reduce(
      (total, change) => ({
        added: total.added + change.added,
        removed: total.removed + change.removed,
      }),
      { added: 0, removed: 0 },
    );
    items.push({
      key: "changes-summary",
      icon: FileDiff,
      label: tr(language, "editedFiles", { count: changes.value.length }),
      stats: { added: totals.added, removed: totals.removed },
      actionLabel: tr(language, "reviewChanges"),
      onOpen: () => emit("review"),
      onAction: () => emit("review"),
    });
  }

  for (const file of sharedFiles.value) {
    items.push({
      key: `shared-${file.offerId || file.path}`,
      icon: File,
      label: file.name,
      title: file.absolutePath || file.path,
      onOpen: () => void openFile(file),
    });
  }

  return items;
});

async function openFile(file: SharedFileOffer) {
  const path = file.absolutePath || file.path;
  if (!path) return;
  const { openInDefaultApp, revealInExplorer } = await import("@/services/ipc");
  try {
    await openInDefaultApp(path);
  } catch {
    try {
      await revealInExplorer(path);
    } catch {
      /* ignore */
    }
  }
}

function filesFromActivities(message: ChatMessage): SharedFileOffer[] {
  const offers: SharedFileOffer[] = [];
  for (const activity of message.toolActivities ?? []) {
    if (activity.toolName !== "share_to_companion" || activity.status !== "done") continue;
    const args = activity.arguments ?? {};
    const path = String(args.path ?? "");
    if (!path) continue;
    const name = String(args.label ?? "") || path.replace(/\\/g, "/").split("/").pop() || path;
    offers.push({
      offerId: activity.id,
      path,
      name,
      mime: "",
      size: 0,
    });
  }
  return offers;
}
</script>
