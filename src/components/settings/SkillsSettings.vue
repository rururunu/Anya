<template>
  <section
    class="settings-page is-wide skills-settings"
    :class="{ 'is-smithery': tab === 'smithery' }"
  >
    <AppConfirmDialog ref="confirmDialogRef" />

    <SettingsPageHeader :title="copy.title">
      <template v-if="tab === 'installed'" #actions>
        <Button
          variant="ghost"
          size="icon"
          class="size-8 shrink-0 text-muted-foreground"
          :title="copy.openDir"
          :aria-label="copy.openDir"
          :disabled="busy"
          @click="openDir"
        >
          <FolderOpen class="size-3.5" />
        </Button>
        <Button variant="ghost" size="sm" class="h-8 gap-1.5" :disabled="busy" @click="installFile">
          <FilePlus class="size-3.5" />
          {{ copy.installFile }}
        </Button>
        <Button size="sm" class="h-8 gap-1.5" :disabled="busy" @click="installFolder">
          <FolderPlus class="size-3.5" />
          {{ copy.installFolder }}
        </Button>
      </template>
    </SettingsPageHeader>

    <div class="settings-tabs" role="tablist">
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'installed' }"
        :aria-selected="tab === 'installed'"
        @click="tab = 'installed'"
      >
        {{ copy.tabInstalled }}
      </button>
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'builtin' }"
        :aria-selected="tab === 'builtin'"
        @click="tab = 'builtin'"
      >
        {{ copy.tabBuiltin }}
      </button>
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'smithery' }"
        :aria-selected="tab === 'smithery'"
        @click="openSmithery"
      >
        {{ copy.tabSmithery }}
      </button>
    </div>

    <p v-if="error" class="form-error">{{ error }}</p>

    <template v-if="tab === 'installed' || tab === 'builtin'">
      <div class="skill-list">
        <p v-if="!loading && visibleLocal.length === 0" class="empty">{{ copy.empty }}</p>
        <p v-else-if="loading" class="empty">{{ copy.loading }}</p>
        <CatalogItemCard
          v-for="skill in visibleLocal"
          :key="`${skill.source}-${skill.name}`"
          :title="skill.title || skill.name"
          :vendor="skill.qualifiedName || ''"
          :meta="skillMeta(skill)"
          :description="skill.description"
          :icon-url="skill.iconUrl"
          :icon-cache-kind="skill.source === 'user' ? 'skill' : null"
          :icon-cache-key="skill.source === 'user' ? skill.name : null"
          :icon-fallback="skill.qualifiedName || skill.title || skill.name"
          :pills="localPills(skill)"
          :expand-label="copy.expand"
          :collapse-label="copy.collapse"
        >
          <template #action>
            <button
              v-if="skill.source === 'builtin'"
              type="button"
              class="setting-toggle"
              :class="{ active: isBuiltinEnabled(skill.name) }"
              :aria-pressed="isBuiltinEnabled(skill.name)"
              :title="isBuiltinEnabled(skill.name) ? copy.enabled : copy.disabled"
              :disabled="busy"
              @click="toggleBuiltin(skill.name)"
            >
              <span class="setting-toggle-knob" />
            </button>
            <CatalogRoundAction
              v-else
              :disabled="busy"
              :label="copy.remove"
              :icon="Trash2"
              :lock-when-done="false"
              @click="remove(skill)"
            />
          </template>
        </CatalogItemCard>
      </div>
    </template>

    <template v-else>
      <div class="smithery-layout">
        <aside class="category-sidebar" :aria-label="copy.categories">
          <nav class="category-nav" role="listbox">
            <button
              type="button"
              class="category-item"
              :class="{ active: !smitheryCategory }"
              role="option"
              :aria-selected="!smitheryCategory"
              @click="selectCategory(null)"
            >
              <LayoutGrid class="size-3.5" />
              <span>{{ copy.categoryAll }}</span>
            </button>
            <button
              v-for="item in categoryItems"
              :key="item.id"
              type="button"
              class="category-item"
              :class="{ active: smitheryCategory === item.id }"
              role="option"
              :aria-selected="smitheryCategory === item.id"
              @click="selectCategory(item.id)"
            >
              <component :is="item.icon" class="size-3.5" />
              <span>{{ item.id }}</span>
            </button>
          </nav>
        </aside>

        <div class="smithery-main">
          <SettingsSearchField
            v-model="smitheryQuery"
            :placeholder="copy.smitherySearch"
            :loading="smitheryLoading"
            :submit-label="copy.search"
            @submit="runSmitherySearch"
          />
          <p v-if="smitheryError" class="form-error">{{ smitheryError }}</p>

          <div class="smithery-list-scroll peek-scrollbar">
            <div class="skill-list">
              <p
                v-if="!smitheryLoading && smitherySkills.length === 0 && smitheryLoaded"
                class="empty"
              >
                {{ copy.smitheryEmpty }}
              </p>
              <p v-else-if="smitheryLoading && smitherySkills.length === 0" class="empty">
                {{ copy.loading }}
              </p>
              <CatalogItemCard
                v-for="entry in smitherySkills"
                :key="entry.id"
                :title="entry.displayName || entry.slug"
                :vendor="entry.qualifiedName || `${entry.namespace}/${entry.slug}`"
                :meta="smitheryMeta(entry)"
                :description="entry.description"
                :icon-url="skillIconUrl(entry)"
                :icon-fallback="entry.qualifiedName || entry.slug"
                :verified="Boolean(entry.verified)"
                :verified-label="copy.verified"
                :pills="isSkillInstalled(entry) ? [copy.added] : []"
                :expand-label="copy.expand"
                :collapse-label="copy.collapse"
              >
                <template v-if="entry.categories?.length" #below-meta>
                  <p class="card-cats">
                    <span v-for="cat in entry.categories.slice(0, 3)" :key="cat" class="cat-tag">
                      {{ cat }}
                    </span>
                  </p>
                </template>
                <template #action>
                  <CatalogRoundAction
                    :done="isSkillInstalled(entry)"
                    :busy="installingId === entry.id"
                    :disabled="busy"
                    :label="
                      isSkillInstalled(entry)
                        ? copy.added
                        : installingId === entry.id
                          ? copy.installing
                          : copy.install
                    "
                    @click="installFromSmithery(entry)"
                  />
                </template>
              </CatalogItemCard>
            </div>

            <InfiniteScrollSentinel
              v-if="smitheryLoaded && (smitheryHasMore || smitheryLoading)"
              :has-more="smitheryHasMore"
              :loading="smitheryLoading"
              @load="loadMoreSmithery"
            />
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, type Component } from "vue";
import {
  BarChart3,
  Box,
  Brain,
  Briefcase,
  CalendarDays,
  Code2,
  FilePlus,
  FolderOpen,
  FolderPlus,
  LayoutGrid,
  MessageCircle,
  Palette,
  Pencil,
  Search,
  Shield,
  Trash2,
  Zap,
} from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import SettingsSearchField from "@/components/settings/SettingsSearchField.vue";
import InfiniteScrollSentinel from "@/components/settings/InfiniteScrollSentinel.vue";
import CatalogItemCard from "@/components/settings/CatalogItemCard.vue";
import CatalogRoundAction from "@/components/settings/CatalogRoundAction.vue";
import {
  installSkill,
  installSkillMarkdown,
  listSkills,
  openSkillsDir,
  selectSkillFile,
  selectSkillFolder,
  uninstallSkill,
  type SkillInfo,
} from "@/commands/skills";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import {
  buildSkillInstallMeta,
  formatSmitheryStars,
  formatSmitheryUses,
  isSameSkillInstall,
  resolveSmitherySkillMarkdown,
  searchSmitherySkills,
  smitherySkillIconUrl,
  sortSmitherySkillsByStars,
  SMITHERY_SKILL_CATEGORIES,
  type SmitherySkillCategory,
  type SmitherySkillSummary,
} from "@/services/skills/smithery";
import { cacheInstallIcon, clearInstallIcon, warmInstallIcons } from "@/services/iconCache";
import { sortByResourceUsage } from "@/services/usage/resourceUsage";

const props = defineProps<{ query?: string }>();
const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

const tab = ref<"installed" | "builtin" | "smithery">("installed");
const skills = ref<SkillInfo[]>([]);
const loading = ref(true);
const busy = ref(false);
const error = ref("");

const smitheryQuery = ref("");
const smitheryCategory = ref<SmitherySkillCategory | null>(null);
const smitherySkills = ref<SmitherySkillSummary[]>([]);
const smitheryLoading = ref(false);
const smitheryError = ref("");
const smitheryLoaded = ref(false);
const smitheryPage = ref(1);
const smitheryTotalPages = ref(1);
const installingId = ref("");

const categoryIcons: Record<SmitherySkillCategory, Component> = {
  Research: Search,
  Coding: Code2,
  Writing: Pencil,
  "Data & Analytics": BarChart3,
  Design: Palette,
  Planning: CalendarDays,
  Communication: MessageCircle,
  Productivity: Zap,
  DevOps: Box,
  "AI & ML": Brain,
  Security: Shield,
  Business: Briefcase,
};

const categoryItems = SMITHERY_SKILL_CATEGORIES.map((id) => ({
  id,
  icon: categoryIcons[id],
}));

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "skills.title"),
    empty: tr(language, "skills.empty"),
    loading: tr(language, "skills.loading"),
    builtin: tr(language, "skills.builtin"),
    user: tr(language, "skills.user"),
    installFolder: tr(language, "skills.installFolder"),
    installFile: tr(language, "skills.installFile"),
    openDir: tr(language, "skills.openDir"),
    remove: tr(language, "skills.remove"),
    deleteTitle: tr(language, "skills.deleteTitle"),
    deleteDesc: tr(language, "skills.deleteDesc"),
    deleteConfirm: tr(language, "skills.deleteConfirm"),
    cancel: tr(language, "skills.cancel"),
    tabInstalled: tr(language, "skills.tabInstalled"),
    tabBuiltin: tr(language, "skills.tabBuiltin"),
    tabSmithery: tr(language, "skills.tabSmithery"),
    smitherySearch: tr(language, "skills.smitherySearch"),
    smitheryEmpty: tr(language, "skills.smitheryEmpty"),
    categories: tr(language, "skills.categories"),
    categoryAll: tr(language, "skills.categoryAll"),
    search: tr(language, "skills.search"),
    searching: tr(language, "skills.searching"),
    install: tr(language, "skills.install"),
    installing: tr(language, "skills.installing"),
    added: tr(language, "skills.added"),
    verified: tr(language, "skills.verified"),
    expand: tr(language, "skills.expand"),
    collapse: tr(language, "skills.collapse"),
    enabled: tr(language, "skills.enabled"),
    disabled: tr(language, "skills.disabled"),
  };
});

const installedSkills = computed(() => skills.value.filter((s) => s.source === "user"));
const builtinSkills = computed(() => skills.value.filter((s) => s.source === "builtin"));

const visibleLocal = computed(() => {
  const list = tab.value === "builtin" ? builtinSkills.value : installedSkills.value;
  const query = props.query?.trim().toLowerCase() ?? "";
  const filtered = !query
    ? list
    : list.filter((skill) => {
        const haystack = [skill.name, skill.title, skill.description, skill.source]
          .join(" ")
          .toLowerCase();
        return haystack.includes(query);
      });
  return sortByResourceUsage(filtered, "skill", (skill) => skill.name);
});

const smitheryHasMore = computed(
  () => smitheryLoaded.value && smitheryPage.value < smitheryTotalPages.value,
);

function isSkillInstalled(entry: SmitherySkillSummary) {
  return skills.value.some((skill) => isSameSkillInstall(skill, entry));
}

function isBuiltinEnabled(name: string) {
  return (settingStore.enabledBuiltinSkills ?? []).includes(name);
}

function localPills(skill: SkillInfo) {
  if (skill.source === "builtin") {
    return [
      copy.value.builtin,
      isBuiltinEnabled(skill.name) ? copy.value.enabled : copy.value.disabled,
    ];
  }
  return [copy.value.user];
}

async function toggleBuiltin(name: string) {
  const current = settingStore.enabledBuiltinSkills ?? [];
  const next = current.includes(name)
    ? current.filter((item) => item !== name)
    : [...current, name];
  busy.value = true;
  error.value = "";
  try {
    await settingStore.update({ enabledBuiltinSkills: next });
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

function skillIconUrl(entry: SmitherySkillSummary) {
  return smitherySkillIconUrl(entry);
}

function skillMeta(skill: SkillInfo) {
  if (skill.qualifiedName) return "";
  if (skill.title && skill.title !== skill.name) return skill.name;
  return "";
}

function smitheryMeta(entry: SmitherySkillSummary) {
  const parts: string[] = [];
  if (entry.externalStars != null) parts.push(formatSmitheryStars(entry.externalStars));
  else if (entry.totalActivations != null) {
    parts.push(formatSmitheryUses(entry.totalActivations));
  }
  return parts.join(" · ");
}

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    skills.value = await listSkills();
    void warmInstallIcons(
      skills.value
        .filter((skill) => skill.source === "user")
        .map((skill) => ({
          kind: "skill" as const,
          cacheKey: skill.name,
          url: skill.iconUrl,
        })),
    );
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    skills.value = [];
  } finally {
    loading.value = false;
  }
}

async function installFolder() {
  busy.value = true;
  error.value = "";
  try {
    const path = await selectSkillFolder();
    if (!path) return;
    await installSkill(path);
    await refresh();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function installFile() {
  busy.value = true;
  error.value = "";
  try {
    const path = await selectSkillFile();
    if (!path) return;
    await installSkill(path);
    await refresh();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function openDir() {
  busy.value = true;
  error.value = "";
  try {
    await openSkillsDir();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function remove(skill: SkillInfo) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.deleteTitle,
    description: copy.value.deleteDesc.replace("{name}", skill.title || skill.name),
    confirmLabel: copy.value.deleteConfirm,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  busy.value = true;
  error.value = "";
  try {
    await uninstallSkill(skill.name);
    void clearInstallIcon("skill", skill.name);
    await refresh();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

async function openSmithery() {
  tab.value = "smithery";
  if (!smitheryLoaded.value && !smitheryLoading.value) {
    await runSmitherySearch();
  }
}

function selectCategory(category: SmitherySkillCategory | null) {
  if (smitheryCategory.value === category) return;
  smitheryCategory.value = category;
  void runSmitherySearch();
}

async function runSmitherySearch() {
  smitheryLoading.value = true;
  smitheryError.value = "";
  smitheryPage.value = 1;
  try {
    const result = await searchSmitherySkills(smitheryQuery.value, {
      page: 1,
      pageSize: 20,
      category: smitheryCategory.value,
    });
    smitherySkills.value = result.skills;
    smitheryTotalPages.value = Math.max(1, result.pagination.totalPages || 1);
    smitheryLoaded.value = true;
  } catch (err) {
    smitheryError.value = err instanceof Error ? err.message : String(err);
    smitherySkills.value = [];
  } finally {
    smitheryLoading.value = false;
  }
}

async function loadMoreSmithery() {
  if (!smitheryHasMore.value || smitheryLoading.value) return;
  smitheryLoading.value = true;
  smitheryError.value = "";
  try {
    const nextPage = smitheryPage.value + 1;
    const result = await searchSmitherySkills(smitheryQuery.value, {
      page: nextPage,
      pageSize: 20,
      category: smitheryCategory.value,
    });
    const seen = new Set(smitherySkills.value.map((s) => s.id));
    const merged = [...smitherySkills.value];
    for (const skill of result.skills) {
      if (seen.has(skill.id)) continue;
      seen.add(skill.id);
      merged.push(skill);
    }
    smitherySkills.value = sortSmitherySkillsByStars(merged);
    smitheryPage.value = nextPage;
    smitheryTotalPages.value = Math.max(1, result.pagination.totalPages || 1);
  } catch (err) {
    smitheryError.value = err instanceof Error ? err.message : String(err);
  } finally {
    smitheryLoading.value = false;
  }
}

async function installFromSmithery(entry: SmitherySkillSummary) {
  if (isSkillInstalled(entry)) return;
  installingId.value = entry.id;
  smitheryError.value = "";
  try {
    const meta = buildSkillInstallMeta(entry);
    const markdown = await resolveSmitherySkillMarkdown(entry);
    await installSkillMarkdown(meta.id, markdown, meta);
    if (meta.iconUrl) {
      await cacheInstallIcon("skill", meta.id, meta.iconUrl);
    }
    await refresh();
  } catch (err) {
    smitheryError.value = err instanceof Error ? err.message : String(err);
  } finally {
    installingId.value = "";
  }
}

onMounted(() => {
  void refresh();
});
</script>

<style scoped>
.skills-settings {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.skills-settings.is-smithery {
  flex: 1;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  padding-bottom: 4px;
}

.form-error,
.empty,
.smithery-hint {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}

.form-error {
  color: #ef4444;
}

.smithery-layout {
  display: grid;
  grid-template-columns: 148px minmax(0, 1fr);
  gap: 12px;
  align-items: stretch;
  flex: 1;
  min-height: 0;
}

.category-sidebar {
  height: 100%;
  overflow-y: auto;
  padding: 2px 2px 8px 0;
  scrollbar-width: thin;
}

.category-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.category-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 32px;
  padding: 6px 10px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.25;
  text-align: left;
  cursor: pointer;
  transition:
    background 0.15s ease,
    color 0.15s ease;
}

.category-item span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.category-item:hover {
  color: var(--foreground);
  background: color-mix(in srgb, var(--foreground) 5%, transparent);
}

.category-item.active {
  color: var(--foreground);
  background: color-mix(in srgb, var(--foreground) 8%, transparent);
  font-weight: 550;
}

.smithery-main {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  min-height: 0;
  height: 100%;
}

.smithery-list-scroll {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  padding-right: 2px;
  padding-bottom: 2px;
  scrollbar-width: thin;
}

.skill-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
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
  margin-top: 5px;
}

.setting-toggle:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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

@media (max-width: 640px) {
  .skills-settings.is-smithery {
    height: auto;
    overflow: visible;
  }

  .smithery-layout {
    grid-template-columns: 1fr;
    flex: none;
    height: auto;
  }

  .category-sidebar {
    height: auto;
    max-height: none;
    overflow: visible;
    padding: 0;
  }

  .category-nav {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px;
  }

  .smithery-main {
    height: auto;
  }

  .smithery-list-scroll {
    max-height: min(55vh, 480px);
  }
}

.card-cats {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin: 0;
}

.cat-tag {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--muted-foreground);
}
</style>
