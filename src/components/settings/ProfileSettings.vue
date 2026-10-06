<script setup lang="ts">
import { computed, nextTick, onMounted, ref, type Component } from "vue";
import { Pencil, Bot } from "@lucide/vue";
import { getModelIcon } from "@/lib/providerIcons";
import { getTokenUsageReport } from "@/services/ipc";
import ProfileInformationActions from "./ProfileInformationActions.vue";
import ProfileTokenActivity from "./ProfileTokenActivity.vue";
import type { UserInformation } from "@/services/settings/userInformation";
import { saveLocalProfile } from "@/services/settings/localProfile";

import { useSettingStore } from "@/stores/setting";
import {
  useUserAvatar,
  setUserAvatar,
  setSeedUserAvatar,
  getUserAvatarSeed,
  getUserAvatarOptions,
  type UserAvatarOptions,
  userAvatarStyles,
  avatarStylePreview,
  getUserAvatarStyle,
  type UserAvatarStyle,
} from "@/services/chat/userAvatar";
import { loadResourceUsage } from "@/services/usage/resourceUsage";
import { profileStats, formatProfileTokens } from "@/services/usage/profileStats";
import type { TokenUsageReport } from "@/types/tokenUsage";

const settingStore = useSettingStore();
function applyImportedProfile(data: UserInformation) {
  displayName.value = data.profile.displayName;
  handle.value = data.profile.handle;
  selectedAvatarStyle.value = data.profile.avatar.style;
  selectedAvatarSeed.value = data.profile.avatar.seed;
  selectedAvatarOptions.value = data.profile.avatar.options;
  editing.value = false;
  avatarEditing.value = false;
}
const avatarPreview = computed(() =>
  selectedAvatarSeed.value.trim()
    ? avatarStylePreview(
        selectedAvatarStyle.value,
        selectedAvatarSeed.value.trim(),
        selectedAvatarOptions.value,
      )
    : "",
);
function applyAvatar() {
  try {
    setSeedUserAvatar(
      selectedAvatarStyle.value,
      selectedAvatarSeed.value,
      selectedAvatarOptions.value,
    );

    avatarEditing.value = false;
  } catch (cause) {
    avatarError.value = String(cause);
  }
}
const zh = computed(() => settingStore.language === "zh-CN");
const copy = computed(() =>
  zh.value
    ? {
        name: "本地用户",
        edit: "编辑资料",
        nickname: "昵称",
        handle: "个人标识",
        save: "保存",
        cancel: "取消",
        local: "资料仅保存在此设备",
        estimated: "Token 统计包含估算值",
        tokens: "累计 Token 数",
        peak: "单日 Token 峰值",
        calls: "模型调用次数",
        longest: "最长连续天数",
        current: "当前连续天数",
        days: "天",
        activity: "Token 活动",
        daily: "每天",
        weekly: "每周",
        cumulative: "累计总量",
        insights: "习惯",
        avatar: "编辑头像",
        upload: "上传图片",
        random: "换一个随机头像",
        imageHint: "PNG、JPEG 或 WebP，最大 1 MB",
        imageError: "请选择不超过 1 MB 的 PNG、JPEG 或 WebP 图片",
        model: "最常用模型",
        effort: "默认思考强度",
        skills: "已使用技能数",
        skillUses: "技能使用次数",
        tools: "已使用 MCP / 插件数",
        none: "暂无记录",
        loading: "正在读取使用记录…",
        retry: "重新加载",
        less: "少",
        more: "多",
        empty: "还没有 Token 活动。开始一次对话后，这里会逐渐留下你的使用足迹。",
      }
    : {
        name: "Local user",
        edit: "Edit profile",
        nickname: "Display name",
        handle: "Handle",
        save: "Save",
        cancel: "Cancel",
        local: "Profile is saved on this device only",
        estimated: "Token totals include estimates",
        tokens: "Total tokens",
        peak: "Daily token peak",
        calls: "Model calls",
        longest: "Longest streak",
        current: "Current streak",
        days: "days",
        activity: "Token activity",
        daily: "Daily",
        weekly: "Weekly",
        cumulative: "Cumulative",
        insights: "Habits",
        avatar: "Edit avatar",
        upload: "Upload image",
        random: "New random avatar",
        imageHint: "PNG, JPEG or WebP, up to 1 MB",
        imageError: "Choose a PNG, JPEG or WebP image under 1 MB",
        model: "Most used model",
        effort: "Default thinking effort",
        skills: "Skills used",
        skillUses: "Skill uses",
        tools: "MCP / plugins used",
        none: "No activity yet",
        loading: "Loading usage history…",
        retry: "Reload",
        less: "Less",
        more: "More",
        empty: "No token activity yet. Your activity will appear here as you start conversations.",
      },
);
const profileKey = "anya.local-profile.v1";
const displayName = ref("");
const handle = ref("local");
const draftName = ref("");
const draftHandle = ref("");
const editing = ref(false);
const profileError = ref("");
const avatarSource = useUserAvatar();
const displayedAvatar = computed(() =>
  avatarEditing.value && avatarPreview.value ? avatarPreview.value : avatarSource.value,
);
const selectedAvatarStyle = ref<UserAvatarStyle>(getUserAvatarStyle());
const selectedAvatarSeed = ref(getUserAvatarSeed());
const selectedAvatarOptions = ref<UserAvatarOptions>(getUserAvatarOptions());
const avatarChoices = userAvatarStyles.map((id) => ({ id, image: avatarStylePreview(id) }));
const avatarLabels = computed<Record<UserAvatarStyle, string>>(() =>
  zh.value
    ? {
        notionists: "线稿",
        adventurer: "卡通",
        pixelArt: "像素",
        bottts: "机器人",
        emoji: "表情",
        identicon: "几何",
        lorelei: "清新人像",
        micah: "插画",
        openPeeps: "手绘",
        thumbs: "小怪兽",
        shapes: "抽象",
        rings: "光环",
      }
    : {
        notionists: "Line art",
        adventurer: "Cartoon",
        pixelArt: "Pixel",
        bottts: "Robot",
        emoji: "Emoji",
        identicon: "Geometric",
        lorelei: "Portrait",
        micah: "Illustration",
        openPeeps: "Hand drawn",
        thumbs: "Creatures",
        shapes: "Abstract",
        rings: "Rings",
      },
);
const avatarEditing = ref(false);
const avatarError = ref("");
const avatarInputRef = ref<HTMLInputElement | null>(null);
async function uploadAvatar(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  avatarError.value = "";
  if (!["image/png", "image/jpeg", "image/webp"].includes(file.type) || file.size > 1024 * 1024) {
    avatarError.value = copy.value.imageError;
    return;
  }
  try {
    const data = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(new Error(copy.value.imageError));
      reader.readAsDataURL(file);
    });
    const image = new Image();
    image.src = data;
    await image.decode();
    setUserAvatar(data);
    avatarEditing.value = false;
  } catch (cause) {
    avatarError.value = String(cause);
  }
}
function newAvatar() {
  selectedAvatarSeed.value = crypto.randomUUID();
  avatarError.value = "";
}
const nameInputRef = ref<HTMLInputElement | null>(null);
const report = ref<TokenUsageReport | null>(null);
const loading = ref(true);
const error = ref("");
const today = new Date();
const resources = ref(loadResourceUsage());
const stats = computed(() => (report.value ? profileStats(report.value, today) : null));
const number = (value: number) => formatProfileTokens(value, settingStore.language);
const exact = (value: number) => new Intl.NumberFormat(settingStore.language).format(value);
const metrics = computed<{ label: string; value: string; raw?: number }[]>(() => [
  {
    label: copy.value.tokens,
    value: report.value ? number(report.value.total.totalTokens) : "—",
    raw: report.value?.total.totalTokens,
  },
  {
    label: copy.value.peak,
    value: stats.value ? number(stats.value.peak) : "—",
    raw: stats.value?.peak,
  },
  { label: copy.value.calls, value: report.value ? exact(report.value.modelCalls) : "—" },
  {
    label: copy.value.longest,
    value: stats.value ? `${stats.value.longest} ${copy.value.days}` : "—",
  },
  {
    label: copy.value.current,
    value: stats.value ? `${stats.value.current} ${copy.value.days}` : "—",
  },
]);
const insights = computed<{ label: string; value: string | number; icon?: Component }[]>(() => {
  const top = report.value?.byModel.slice().sort((a, b) => b.calls - a.calls)[0];
  const totalCalls = report.value?.modelCalls ?? 0;
  const effort = settingStore.reasoningEffort || "default";
  const names: Record<string, string> = zh.value
    ? {
        none: "关闭",
        minimal: "最低",
        low: "低",
        medium: "中",
        high: "高",
        xhigh: "最高",
        default: "默认",
      }
    : {};
  return [
    {
      label: copy.value.model,
      icon: top ? getModelIcon({ id: top.model }) || Bot : undefined,
      value: top
        ? `${top.model} · ${Math.round((top.calls / Math.max(1, totalCalls)) * 100)}%`
        : copy.value.none,
    },
    { label: copy.value.effort, value: names[effort] ?? effort },
    { label: copy.value.skills, value: Object.keys(resources.value.skill).length },
    {
      label: copy.value.skillUses,
      value: Object.values(resources.value.skill).reduce((sum, entry) => sum + entry.count, 0),
    },
    {
      label: copy.value.tools,
      value: Object.keys(resources.value.mcp).length + Object.keys(resources.value.plugin).length,
    },
  ];
});
async function load() {
  loading.value = true;
  error.value = "";
  try {
    report.value = await getTokenUsageReport({ from: 0, to: Date.now() + 1, granularity: "day" });
    resources.value = loadResourceUsage();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}
function editProfile() {
  draftName.value = displayName.value;
  draftHandle.value = handle.value;
  profileError.value = "";
  editing.value = true;
  void nextTick(() => nameInputRef.value?.focus());
}
function saveProfile() {
  try {
    const name = draftName.value.trim().slice(0, 60);
    const id =
      draftHandle.value.trim().replace(/^@+/, "").replace(/\s+/g, "-").slice(0, 40) || "local";
    saveLocalProfile({ displayName: name, handle: id });
    displayName.value = name;
    handle.value = id;
    editing.value = false;
  } catch (cause) {
    profileError.value = String(cause);
  }
}
onMounted(() => {
  try {
    localStorage.removeItem("anya.avatar-generation-session.v1");
    localStorage.removeItem("anya.avatar-seed-generation-session.v1");
    const stored = JSON.parse(localStorage.getItem(profileKey) || "{}");
    displayName.value =
      typeof stored.displayName === "string" ? stored.displayName.slice(0, 60) : "";
    handle.value = typeof stored.handle === "string" ? stored.handle.slice(0, 40) : "local";
  } catch {
    /* Keep local defaults if storage is unavailable. */
  }
  void load();
});
</script>

<template>
  <section class="profile-page">
    <ProfileInformationActions @imported="applyImportedProfile" />
    <header class="profile-identity">
      <button
        class="avatar-button"
        type="button"
        :aria-label="copy.avatar"
        :aria-expanded="avatarEditing"
        @click="avatarEditing = !avatarEditing"
      >
        <img
          class="profile-avatar"
          :class="{ custom: !displayedAvatar.startsWith('data:image/svg+xml') }"
          :src="displayedAvatar"
          alt=""
        />
        <span class="avatar-edit-mark"><Pencil :size="13" /></span>
      </button>
      <div v-if="avatarEditing" class="avatar-editor">
        <div class="avatar-seed-editor">
          <label for="avatar-seed">{{ zh ? "头像种子" : "Avatar seed" }}</label>
          <input
            id="avatar-seed"
            v-model="selectedAvatarSeed"
            maxlength="128"
            :placeholder="zh ? '可以输入任意文字或数字' : 'Enter any text or number'"
          />
          <p>
            {{
              zh
                ? "相同风格、外观参数与种子会生成相同头像，可随时修改预览。"
                : "The same style, appearance options and seed produce the same avatar."
            }}
          </p>
          <div v-if="avatarPreview" class="avatar-seed-preview">
            <img :src="avatarPreview" alt="" />
            <button type="button" @click="applyAvatar">
              {{ zh ? "使用此头像" : "Use this avatar" }}
            </button>
          </div>
        </div>
        <div class="avatar-style-grid" role="group" :aria-label="copy.random">
          <button
            v-for="choice in avatarChoices"
            :key="choice.id"
            type="button"
            class="avatar-style-option"
            :aria-pressed="selectedAvatarStyle === choice.id"
            @click="
              selectedAvatarStyle = choice.id;
              selectedAvatarOptions = {};
            "
          >
            <img :src="choice.image" alt="" />
            <span>{{ avatarLabels[choice.id] }}</span>
          </button>
        </div>
        <button type="button" @click="avatarInputRef?.click()">{{ copy.upload }}</button>
        <button type="button" @click="newAvatar">{{ copy.random }}</button>
        <p>{{ copy.imageHint }}</p>
        <p v-if="avatarError" role="alert">{{ avatarError }}</p>
      </div>
      <input
        ref="avatarInputRef"
        type="file"
        accept="image/png,image/jpeg,image/webp"
        hidden
        @change="uploadAvatar"
      />
      <h1>{{ displayName || copy.name }}</h1>
      <p class="profile-handle">@{{ handle }}</p>
      <button class="profile-edit" type="button" @click="editProfile">
        <Pencil :size="12" />
        {{ copy.edit }}
      </button>
      <form v-if="editing" class="profile-form" @submit.prevent="saveProfile">
        <label>
          {{ copy.nickname }}
          <input ref="nameInputRef" v-model="draftName" maxlength="60" :placeholder="copy.name" />
        </label>
        <label>
          {{ copy.handle }}
          <input v-model="draftHandle" maxlength="40" />
        </label>
        <p>{{ copy.local }}</p>
        <p v-if="profileError" role="alert">{{ profileError }}</p>
        <div>
          <button type="button" @click="editing = false">{{ copy.cancel }}</button>
          <button type="submit">{{ copy.save }}</button>
        </div>
      </form>
    </header>
    <div class="profile-metrics" :aria-busy="loading">
      <div v-for="metric in metrics" :key="metric.label">
        <strong :title="metric.raw == null ? undefined : `${exact(metric.raw)} tokens`">
          {{ metric.value }}
        </strong>
        <span>{{ metric.label }}</span>
      </div>
    </div>
    <p v-if="report && report.total.accuracy !== 'exact'" class="profile-status">
      {{ copy.estimated }}
    </p>
    <p v-if="loading" class="profile-status" role="status">{{ copy.loading }}</p>
    <div v-else-if="error" class="profile-status" role="alert">
      <p>{{ error }}</p>
      <button type="button" @click="load">{{ copy.retry }}</button>
    </div>
    <ProfileTokenActivity
      :report="report"
      :stats="stats"
      :today="today"
      :loading="loading"
      :language="settingStore.language"
      :labels="copy"
      @refresh="load"
    />
    <section class="profile-insights">
      <h2>{{ copy.insights }}</h2>
      <dl>
        <div v-for="insight in insights" :key="insight.label">
          <dt>{{ insight.label }}</dt>
          <dd>
            <component
              v-if="insight.icon"
              :is="insight.icon"
              class="habit-model-icon"
              :size="16"
              aria-hidden="true"
            />
            {{ insight.value }}
          </dd>
        </div>
      </dl>
    </section>
  </section>
</template>

<style scoped>
.avatar-seed-editor {
  width: 100%;
  display: grid;
  gap: 10px;
  margin-bottom: 8px;
}
.avatar-seed-editor label {
  color: var(--peek-text);
  text-align: left;
}
.avatar-seed-editor input {
  width: 100%;
  box-sizing: border-box;
  padding: 9px 10px;
  background: var(--peek-input-bg);
  color: var(--peek-text);
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  font: inherit;
}
.avatar-seed-editor div {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
}
.avatar-seed-editor button:disabled {
  opacity: 0.45;
  cursor: default;
}
.avatar-seed-preview img {
  width: 60px;
  height: 60px;
  border-radius: 50%;
}
.avatar-style-grid {
  max-height: 270px;
  overflow-y: auto;
  padding: 2px;
}
.avatar-style-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  width: 100%;
}
.avatar-style-option {
  flex-direction: column;
  padding: 10px 8px;
  border: 1px solid var(--peek-border);
  border-radius: 10px;
  gap: 7px;
}
.avatar-style-option[aria-pressed="true"] {
  border-color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 6%, transparent);
}
.avatar-style-option img {
  width: 44px;
  height: 44px;
  border-radius: 50%;
}
.avatar-button {
  position: relative;
  padding: 0;
  border-radius: 50%;
}
.avatar-edit-mark {
  position: absolute;
  right: 0;
  bottom: 2px;
  display: grid;
  place-items: center;
  width: 25px;
  height: 25px;
  border: 1px solid var(--peek-border);
  border-radius: 50%;
  background: var(--peek-surface);
  color: var(--peek-muted);
}
.avatar-editor {
  max-width: 320px;
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 10px;
  margin-top: 16px;
  font-size: 12px;
}
.avatar-editor p {
  flex-basis: 100%;
  margin: 0;
  text-align: center;
  color: var(--peek-muted);
  font-size: 11px;
}
.profile-avatar.custom {
  padding: 0;
  object-fit: cover;
}
.profile-page {
  max-width: 900px;
  margin: 0 auto;
  padding: 32px 8px 48px;
  color: var(--peek-text);
}
.profile-identity {
  display: flex;
  align-items: center;
  flex-direction: column;
  padding: 12px 0 36px;
}
.profile-avatar {
  width: 100px;
  height: 100px;
  border-radius: 50%;
  border: 1px solid var(--peek-border);
  background: var(--peek-surface);
  padding: 6px;
}
h1 {
  margin: 18px 0 8px;
  font-size: 23px;
  font-weight: 600;
  letter-spacing: -0.03em;
}
.profile-handle {
  margin: 0;
  font-size: 13px;
  color: var(--peek-muted);
}
button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  font: inherit;
}
button:hover,
button[aria-pressed="true"] {
  color: var(--peek-text);
}
button:focus-visible,
input:focus-visible {
  outline: 2px solid var(--peek-accent);
  outline-offset: 3px;
}
.profile-edit {
  margin-top: 14px;
  font-size: 12px;
}
.profile-form {
  width: min(100%, 320px);
  padding: 18px;
  margin-top: 16px;
  border: 1px solid var(--peek-border);
  border-radius: 12px;
}
.profile-form label {
  display: grid;
  gap: 6px;
  font-size: 12px;
  margin-bottom: 12px;
}
.profile-form input {
  padding: 8px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 7px;
  background: var(--peek-input-bg);
  color: var(--peek-text);
  width: 100%;
  box-sizing: border-box;
}
.profile-form p {
  color: var(--peek-muted);
  font-size: 11px;
}
.profile-form div {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
.profile-metrics {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  border: 1px solid var(--peek-border);
  border-radius: 15px;
  padding: 17px 0;
}
.profile-metrics div {
  display: grid;
  text-align: center;
  gap: 6px;
  padding: 0 10px;
}
.profile-metrics div + div {
  border-left: 1px solid var(--peek-border);
}
.profile-metrics strong {
  font-size: 16px;
  font-weight: 550;
  font-variant-numeric: tabular-nums;
}
.profile-metrics span {
  font-size: 12px;
  color: var(--peek-muted);
}
h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}
.profile-insights {
  margin-top: 42px;
}
dl {
  margin: 20px 0 0;
}
dl div {
  display: flex;
  justify-content: space-between;
  gap: 24px;
  padding: 10px 0;
  font-size: 13px;
}
dt {
  color: var(--peek-muted);
}
dd {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  text-align: right;
  overflow-wrap: anywhere;
}
.habit-model-icon {
  flex: none;
  color: var(--peek-muted);
}
.profile-status {
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.6;
}
@media (max-width: 700px) {
  .profile-page {
    padding-top: 16px;
  }
  .profile-metrics {
    grid-template-columns: repeat(3, 1fr);
    row-gap: 20px;
  }
  .profile-metrics div:nth-child(4) {
    border-left: 0;
  }
}
</style>
