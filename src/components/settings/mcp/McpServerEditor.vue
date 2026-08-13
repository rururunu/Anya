<template>
  <article class="editor-card">
    <div class="editor-grid">
      <label>
        <span>{{ copy.id }}</span>
        <Input
          v-model="id"
          class="h-8"
          :placeholder="copy.idPlaceholder"
          :disabled="editor.mode === 'edit'"
        />
      </label>
      <label>
        <span>{{ metaLabels.displayName }}</span>
        <Input v-model="title" class="h-8" :placeholder="metaLabels.displayNamePlaceholder" />
      </label>
      <label class="full">
        <span>{{ metaLabels.blurb }}</span>
        <Input v-model="description" class="h-8" :placeholder="metaLabels.blurbPlaceholder" />
      </label>
      <label>
        <span>{{ copy.command }}</span>
        <Input v-model="command" class="h-8" :placeholder="copy.commandPlaceholder" />
      </label>
      <label class="full">
        <span>{{ copy.args }}</span>
        <Input v-model="argsText" class="h-8" :placeholder="copy.argsPlaceholder" />
      </label>
      <label class="full">
        <span>{{ copy.env }}</span>
        <textarea
          v-model="envText"
          class="env-input peek-scrollbar"
          rows="3"
          :placeholder="copy.envPlaceholder"
        />
      </label>
      <label class="toggle-row">
        <span>{{ copy.enabled }}</span>
        <button
          type="button"
          class="setting-toggle"
          :class="{ active: editor.enabled }"
          :aria-pressed="editor.enabled"
          @click="setField('enabled', !editor.enabled)"
        >
          <span class="setting-toggle-knob" />
        </button>
      </label>
    </div>
    <div class="editor-actions">
      <Button variant="ghost" size="sm" class="h-8" @click="$emit('cancel')">
        {{ copy.cancel }}
      </Button>
      <Button size="sm" class="h-8" :disabled="saving" @click="$emit('save')">
        {{ copy.save }}
      </Button>
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type EditorState = {
  mode: "create" | "edit";
  id: string;
  title: string;
  description: string;
  command: string;
  argsText: string;
  envText: string;
  enabled: boolean;
};

const props = defineProps<{
  editor: EditorState;
  saving: boolean;
  copy: {
    id: string;
    idPlaceholder: string;
    command: string;
    commandPlaceholder: string;
    args: string;
    argsPlaceholder: string;
    env: string;
    envPlaceholder: string;
    enabled: string;
    cancel: string;
    save: string;
  };
  metaLabels: {
    displayName: string;
    displayNamePlaceholder: string;
    blurb: string;
    blurbPlaceholder: string;
  };
}>();

const emit = defineEmits<{
  cancel: [];
  save: [];
  "update:editor": [value: EditorState];
}>();

function setField<K extends keyof EditorState>(key: K, value: EditorState[K]) {
  emit("update:editor", { ...props.editor, [key]: value });
}

function field<K extends keyof EditorState>(key: K) {
  return computed({
    get: () => props.editor[key],
    set: (value: EditorState[K]) => setField(key, value),
  });
}

const id = field("id");
const title = field("title");
const description = field("description");
const command = field("command");
const argsText = field("argsText");
const envText = field("envText");
</script>

<style scoped>
.editor-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.editor-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 12px;
}

.editor-grid label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 11px;
  color: var(--muted-foreground);
}

.editor-grid label.full {
  grid-column: 1 / -1;
}

.editor-grid label.toggle-row {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}

.env-input {
  width: 100%;
  min-height: 72px;
  resize: vertical;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--background);
  color: inherit;
  padding: 8px 10px;
  font-size: 12px;
  font-family: var(--font-mono, ui-monospace, Consolas, monospace);
  line-height: 1.45;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.setting-toggle {
  position: relative;
  width: 36px;
  height: 20px;
  border: 0;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted-foreground) 28%, transparent);
  cursor: pointer;
  padding: 0;
  flex: none;
}

.setting-toggle.active {
  background: color-mix(in srgb, var(--primary) 75%, transparent);
}

.setting-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: white;
  transition: transform 140ms ease;
}

.setting-toggle.active .setting-toggle-knob {
  transform: translateX(16px);
}
</style>
