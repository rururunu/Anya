<template>
  <ArtifactRows v-if="rows.length" :rows="rows" />
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { ChevronDown, ChevronUp, File, FileDiff } from "@lucide/vue";
import ArtifactRows, { type ArtifactRow } from "@/components/chat/ArtifactRows.vue";
import { extractCodeChanges } from "@/services/chat/codeChanges";
import { fileBasename } from "@/services/chat/toolDiff";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, SharedFileOffer } from "@/types/chat";

const FILE_PREVIEW = 3;

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
const expanded = ref(false);
const changes = computed(() => extractCodeChanges([props.message]));
const changedPaths = computed(() => new Set(changes.value.map((change) => change.path)));

const sharedFiles = computed(() => {
  if (props.message.sharedFiles?.length) return props.message.sharedFiles;
  return filesFromActivities(props.message).filter((file) => !changedPaths.value.has(file.path));
});

const rows = computed((): ArtifactRow[] => {
  const language = settingStore.language;
  const items: ArtifactRow[] = [];
  const files = changes.value;

  if (files.length === 1) {
    const change = files[0]!;
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
  } else if (files.length > 1) {
    const totals = files.reduce(
      (total, change) => ({
        added: total.added + change.added,
        removed: total.removed + change.removed,
      }),
      { added: 0, removed: 0 },
    );
    items.push({
      key: "changes-summary",
      icon: FileDiff,
      label: tr(language, "editedFiles", { count: files.length }),
      stats: { added: totals.added, removed: totals.removed },
      actionLabel: tr(language, "reviewChanges"),
      onOpen: () => emit("review"),
      onAction: () => emit("review"),
    });

    const hidden = files.length - FILE_PREVIEW;
    const visible = expanded.value || hidden <= 0 ? files : files.slice(0, FILE_PREVIEW);
    for (const change of visible) {
      items.push({
        key: `change-${change.id}`,
        icon: File,
        label: displayChangePath(change.path),
        title: change.path,
        stats: { added: change.added, removed: change.removed },
        nested: true,
        onOpen: () => emit("reviewFile", change.path),
      });
    }
    if (hidden > 0) {
      items.push(
        expanded.value
          ? {
              key: "changes-more",
              icon: ChevronUp,
              label: tr(language, "collapse"),
              nested: true,
              onOpen: () => {
                expanded.value = false;
              },
            }
          : {
              key: "changes-more",
              icon: ChevronDown,
              label: tr(language, "moreEditedFiles", { count: hidden }),
              nested: true,
              onOpen: () => {
                expanded.value = true;
              },
            },
      );
    }
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

function displayChangePath(path: string) {
  return path.replace(/\\/g, "/");
}

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
