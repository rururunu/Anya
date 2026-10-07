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
  "ja-JP": {
    "onboarding.welcomeTitle": "Anya へようこそ",
    "onboarding.welcomeSubtitle": "仕事や質問をいつでも Anya に任せましょう。",
    "onboarding.continue": "続ける",
    "onboarding.skip": "後で設定する",
    "onboarding.skipTour": "ガイドをスキップ",
    "onboarding.back": "戻る",
    "onboarding.providerTitle": "プロバイダーを接続",
    "onboarding.providerSubtitle":
      "DeepSeek API キーを入力するか、OpenAI 互換の接続先を設定してください。",
    "onboarding.providerDeepSeekHint": "DeepSeek API キーを貼り付け",
    "onboarding.providerCustomHint": "MiMo、GLM、Ark、MiniMax、Kimi、または空欄",
    "onboarding.providerConfigured": "準備完了",
    "onboarding.providerLater": "プロバイダーは設定からいつでも変更できます。",
    "onboarding.hotkeyTitle": "Alt · Alt で呼び出す",
    "onboarding.hotkeySubtitle":
      "どこでも Alt を素早く2回押すと Anya が開きます。作業を中断せずに質問や貼り付けができます。",
    "onboarding.hotkeyHint": "主なショートカット：長押しせずに素早く2回押す",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "ワークスペースへ",
    "onboarding.customName": "プロバイダー名",
    "onboarding.customBaseUrl": "ベース URL",
    "onboarding.customApiKey": "API キー",
    "onboarding.customModels": "モデル",
    "onboarding.saveCustom": "プロバイダーを保存",
    "onboarding.stepOf": "{total} ステップ中 {current} ステップ",
  },
  "ru-RU": {
    "onboarding.welcomeTitle": "Добро пожаловать в Anya",
    "onboarding.welcomeSubtitle": "Доверьте Anya свои задачи и вопросы в любой момент.",
    "onboarding.continue": "Продолжить",
    "onboarding.skip": "Позже",
    "onboarding.skipTour": "Пропустить руководство",
    "onboarding.back": "Назад",
    "onboarding.providerTitle": "Подключите провайдера",
    "onboarding.providerSubtitle":
      "Добавьте ключ DeepSeek API или настройте адрес, совместимый с OpenAI.",
    "onboarding.providerDeepSeekHint": "Вставьте ключ DeepSeek API",
    "onboarding.providerCustomHint": "MiMo, GLM, Ark, MiniMax, Kimi или пусто",
    "onboarding.providerConfigured": "Готово",
    "onboarding.providerLater": "Провайдера можно изменить в настройках в любое время.",
    "onboarding.hotkeyTitle": "Вызов через Alt · Alt",
    "onboarding.hotkeySubtitle":
      "Быстро нажмите Alt дважды в любом месте, чтобы открыть Anya. Задавайте вопросы и вставляйте текст, не прерывая работу.",
    "onboarding.hotkeyHint": "Основное сочетание: два быстрых нажатия, без удержания",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "Открыть рабочую область",
    "onboarding.customName": "Название провайдера",
    "onboarding.customBaseUrl": "Базовый URL",
    "onboarding.customApiKey": "Ключ API",
    "onboarding.customModels": "Модели",
    "onboarding.saveCustom": "Сохранить провайдера",
    "onboarding.stepOf": "Шаг {current} из {total}",
  },
  "de-DE": {
    "onboarding.welcomeTitle": "Willkommen bei Anya",
    "onboarding.welcomeSubtitle": "Übertrage Anya jederzeit deine Aufgaben und Fragen.",
    "onboarding.continue": "Weiter",
    "onboarding.skip": "Später",
    "onboarding.skipTour": "Einführung überspringen",
    "onboarding.back": "Zurück",
    "onboarding.providerTitle": "Anbieter verbinden",
    "onboarding.providerSubtitle":
      "Füge einen DeepSeek-API-Schlüssel hinzu oder richte einen OpenAI-kompatiblen Endpunkt ein.",
    "onboarding.providerDeepSeekHint": "DeepSeek-API-Schlüssel einfügen",
    "onboarding.providerCustomHint": "MiMo, GLM, Ark, MiniMax, Kimi oder leer",
    "onboarding.providerConfigured": "Bereit",
    "onboarding.providerLater": "Du kannst Anbieter jederzeit in den Einstellungen ändern.",
    "onboarding.hotkeyTitle": "Mit Alt · Alt aufrufen",
    "onboarding.hotkeySubtitle":
      "Drücke Alt zweimal kurz hintereinander, um Anya überall zu öffnen. Frage und füge Inhalte ein, ohne deine Arbeit zu unterbrechen.",
    "onboarding.hotkeyHint": "Hauptkürzel: zweimal kurz drücken, nicht gedrückt halten",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "Arbeitsbereich öffnen",
    "onboarding.customName": "Anbietername",
    "onboarding.customBaseUrl": "Basis-URL",
    "onboarding.customApiKey": "API-Schlüssel",
    "onboarding.customModels": "Modelle",
    "onboarding.saveCustom": "Anbieter speichern",
    "onboarding.stepOf": "Schritt {current} von {total}",
  },
  "fr-FR": {
    "onboarding.welcomeTitle": "Bienvenue sur Anya",
    "onboarding.welcomeSubtitle": "Confiez vos tâches et vos questions à Anya à tout moment.",
    "onboarding.continue": "Continuer",
    "onboarding.skip": "Plus tard",
    "onboarding.skipTour": "Ignorer le guide",
    "onboarding.back": "Retour",
    "onboarding.providerTitle": "Connecter un fournisseur",
    "onboarding.providerSubtitle":
      "Ajoutez une clé API DeepSeek ou configurez un point de terminaison compatible avec OpenAI.",
    "onboarding.providerDeepSeekHint": "Collez votre clé API DeepSeek",
    "onboarding.providerCustomHint": "MiMo, GLM, Ark, MiniMax, Kimi ou vide",
    "onboarding.providerConfigured": "Prêt",
    "onboarding.providerLater":
      "Vous pouvez changer de fournisseur à tout moment dans les paramètres.",
    "onboarding.hotkeyTitle": "Ouvrir avec Alt · Alt",
    "onboarding.hotkeySubtitle":
      "Appuyez rapidement deux fois sur Alt pour ouvrir Anya où que vous soyez. Posez vos questions et collez du contenu sans interrompre votre travail.",
    "onboarding.hotkeyHint":
      "Raccourci principal : deux pressions rapides, sans maintenir la touche",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "Accéder à l'espace de travail",
    "onboarding.customName": "Nom du fournisseur",
    "onboarding.customBaseUrl": "URL de base",
    "onboarding.customApiKey": "Clé API",
    "onboarding.customModels": "Modèles",
    "onboarding.saveCustom": "Enregistrer le fournisseur",
    "onboarding.stepOf": "Étape {current} sur {total}",
  },
  "ko-KR": {
    "onboarding.welcomeTitle": "Anya에 오신 것을 환영합니다",
    "onboarding.welcomeSubtitle": "언제든 작업과 질문을 Anya에게 맡기세요.",
    "onboarding.continue": "계속",
    "onboarding.skip": "나중에 하기",
    "onboarding.skipTour": "안내 건너뛰기",
    "onboarding.back": "뒤로",
    "onboarding.providerTitle": "제공업체 연결",
    "onboarding.providerSubtitle":
      "DeepSeek API 키를 추가하거나 OpenAI 호환 엔드포인트를 설정하세요.",
    "onboarding.providerDeepSeekHint": "DeepSeek API 키 붙여넣기",
    "onboarding.providerCustomHint": "MiMo, GLM, Ark, MiniMax, Kimi 또는 빈칸",
    "onboarding.providerConfigured": "준비됨",
    "onboarding.providerLater": "설정에서 언제든 제공업체를 변경할 수 있습니다.",
    "onboarding.hotkeyTitle": "Alt · Alt로 호출",
    "onboarding.hotkeySubtitle":
      "어디서든 Alt를 빠르게 두 번 누르면 Anya가 열립니다. 작업 흐름을 유지하면서 질문하고 붙여넣으세요.",
    "onboarding.hotkeyHint": "기본 단축키: 길게 누르지 말고 빠르게 두 번 누르세요",
    "onboarding.hotkeyGesture": "Alt",
    "onboarding.finish": "작업 영역으로 이동",
    "onboarding.customName": "제공업체 이름",
    "onboarding.customBaseUrl": "기본 URL",
    "onboarding.customApiKey": "API 키",
    "onboarding.customModels": "모델",
    "onboarding.saveCustom": "제공업체 저장",
    "onboarding.stepOf": "{total}단계 중 {current}단계",
  },
};
