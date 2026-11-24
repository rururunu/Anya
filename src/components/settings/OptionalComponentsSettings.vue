<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";
import { ExternalLink, Files, LoaderCircle } from "@lucide/vue";
import { onMounted, ref } from "vue";
import SettingsFormError from "@/components/settings/SettingsFormError.vue";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import SettingsToggle from "@/components/settings/SettingsToggle.vue";
import { Button } from "@/components/ui/button";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const settings = useSettingStore();

interface OfficeStatus {
  version: string | null;
  enabled: boolean;
}

const status = ref<OfficeStatus>();
const busy = ref(true);
const failure = ref("");
const officialPackage = "https://github.com/deepseek-ai/dsh-libreoffice-kit";

async function openOfficialPackage() {
  try {
    await openUrl(officialPackage);
  } catch (error) {
    failure.value = String(error);
  }
}

async function run(command: string, args?: Record<string, unknown>) {
  busy.value = true;
  failure.value = "";
  try {
    status.value = await invoke<OfficeStatus>(command, args);
  } catch (error) {
    failure.value = String(error);
  } finally {
    busy.value = false;
  }
}

onMounted(() => run("office_runtime_status"));
</script>

<template>
  <section class="settings-page">
    <SettingsPageHeader :title="tr(settings.language, 'settings.categories.components')" />
    <SettingsFormError :message="failure" />
    <div class="settings-card">
      <article class="settings-row is-wide">
        <div class="flex items-start gap-3">
          <span
            class="grid size-10 shrink-0 place-items-center rounded-lg bg-muted text-foreground"
          >
            <Files class="size-5" aria-hidden="true" />
          </span>
          <div class="settings-row-copy">
            <h3>{{ tr(settings.language, "settings.components.office") }}</h3>
            <p>{{ tr(settings.language, "settings.components.officeDescription") }}</p>
            <p>{{ tr(settings.language, "settings.components.engineSource") }}</p>
            <p v-if="status?.version">{{ status.version }}</p>
          </div>
        </div>
        <div class="settings-row-control flex-wrap gap-2">
          <LoaderCircle
            v-if="busy"
            class="size-4 animate-spin"
            role="status"
            :aria-label="tr(settings.language, 'settings.components.working')"
          />
          <Button size="sm" variant="outline" @click="openOfficialPackage">
            <ExternalLink class="size-4" aria-hidden="true" />
            {{ tr(settings.language, "settings.components.officialSource") }}
          </Button>
          <SettingsToggle
            v-if="status?.version"
            :model-value="status.enabled"
            :disabled="busy"
            :aria-label="tr(settings.language, 'settings.components.office')"
            @update:model-value="(enabled) => run('set_office_runtime_enabled', { enabled })"
          />
        </div>
      </article>
      <div v-if="status" class="settings-row">
        <div class="settings-row-copy">
          <p role="status">
            {{
              tr(
                settings.language,
                !status.version
                  ? "settings.components.notInstalled"
                  : status.enabled
                    ? "settings.components.enabled"
                    : "settings.components.disabled",
              )
            }}
          </p>
        </div>
      </div>
    </div>
  </section>
</template>
