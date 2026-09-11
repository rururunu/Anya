import type { AppLanguage } from "@/types/setting";

export type OnboardingI18nKey =
  | "onboarding.welcomeTitle"
  | "onboarding.welcomeSubtitle"
  | "onboarding.continue"
  | "onboarding.skip"
  | "onboarding.skipTour"
  | "onboarding.back"
  | "onboarding.providerTitle"
  | "onboarding.providerSubtitle"
  | "onboarding.providerDeepSeekHint"
  | "onboarding.providerCustomHint"
  | "onboarding.providerConfigured"
  | "onboarding.providerLater"
  | "onboarding.hotkeyTitle"
  | "onboarding.hotkeySubtitle"
  | "onboarding.hotkeyHint"
  | "onboarding.hotkeyGesture"
  | "onboarding.finish"
  | "onboarding.customName"
  | "onboarding.customBaseUrl"
  | "onboarding.customApiKey"
  | "onboarding.customModels"
  | "onboarding.saveCustom"
  | "onboarding.stepOf";

export const onboardingEn: Record<OnboardingI18nKey, string> = {
  "onboarding.welcomeTitle": "Welcome to Anya",
  "onboarding.welcomeSubtitle": "Hand your work & questions to Anya anytime.",
  "onboarding.continue": "Continue",
  "onboarding.skip": "Skip for now",
  "onboarding.skipTour": "Skip guide",
  "onboarding.back": "Back",
  "onboarding.providerTitle": "Connect a provider",
  "onboarding.providerSubtitle":
    "Add a DeepSeek API key or configure a custom OpenAI-compatible endpoint.",
  "onboarding.providerDeepSeekHint": "Paste your DeepSeek API key",
  "onboarding.providerCustomHint": "MiMo, GLM, Ark, MiniMax, Kimi, or blank",
  "onboarding.providerConfigured": "Ready",
  "onboarding.providerLater": "You can change providers anytime in Settings.",
  "onboarding.hotkeyTitle": "Summon with Alt · Alt",
  "onboarding.hotkeySubtitle":
    "Double-tap Alt anywhere to open the Anya overlay. Ask, paste, and keep working without leaving your flow.",
  "onboarding.hotkeyHint": "Primary shortcut — two quick taps (not a long hold)",
  "onboarding.hotkeyGesture": "Alt",
  "onboarding.finish": "Enter workspace",
  "onboarding.customName": "Provider name",
  "onboarding.customBaseUrl": "Base URL",
  "onboarding.customApiKey": "API Key",
  "onboarding.customModels": "Models",
  "onboarding.saveCustom": "Save provider",
  "onboarding.stepOf": "Step {current} of {total}",
};

export const onboardingLocales: Record<AppLanguage, Partial<Record<OnboardingI18nKey, string>>> = {
  "en-US": {},
  "zh-CN": {
    "onboarding.welcomeTitle": "欢迎您使用 Anya",
    "onboarding.welcomeSubtitle": "将你的工作&疑问随手交给Anya",
    "onboarding.continue": "继续",
    "onboarding.skip": "稍后再说",
    "onboarding.skipTour": "跳过引导",
    "onboarding.back": "返回",
    "onboarding.providerTitle": "配置模型提供商",
    "onboarding.providerSubtitle": "填写 DeepSeek API Key，或添加自定义 OpenAI 兼容接口。",
    "onboarding.providerDeepSeekHint": "粘贴 DeepSeek API Key",
    "onboarding.providerCustomHint": "MiMo、GLM、方舟、MiniMax、Kimi 或空白",
    "onboarding.providerConfigured": "已就绪",
    "onboarding.providerLater": "之后可随时在设置中修改提供商。",
    "onboarding.hotkeyTitle": "连按 Alt · Alt 呼出",
    "onboarding.hotkeySubtitle":
      "在任意界面连按两次 Alt，即可唤出 Anya 悬浮窗，提问与粘贴无需打断当前工作。",
    "onboarding.hotkeyHint": "主快捷键：快速连按两下短按（长按无效）",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "进入工作区",
    "onboarding.customName": "提供商名称",
    "onboarding.customBaseUrl": "Base URL",
    "onboarding.customApiKey": "API Key",
    "onboarding.customModels": "模型列表",
    "onboarding.saveCustom": "保存提供商",
    "onboarding.stepOf": "第 {current} / {total} 步",
  },
  "ja-JP": {},
  "ru-RU": {},
  "de-DE": {},
  "fr-FR": {},
  "ko-KR": {},
};
