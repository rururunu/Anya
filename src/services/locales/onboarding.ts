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
  | "onboarding.stepOf"
  | "onboarding.tour.next"
  | "onboarding.tour.back"
  | "onboarding.tour.done"
  | "onboarding.tour.close"
  | "onboarding.tour.welcome.title"
  | "onboarding.tour.welcome.body"
  | "onboarding.tour.provider.title"
  | "onboarding.tour.provider.body"
  | "onboarding.tour.composer.title"
  | "onboarding.tour.composer.body"
  | "onboarding.tour.hotkey.title"
  | "onboarding.tour.hotkey.body"
  | "onboarding.tour.workspace.title"
  | "onboarding.tour.workspace.body"
  | "onboarding.emptyProviderTitle"
  | "onboarding.emptyHotkey";

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
  "onboarding.tour.next": "Next",
  "onboarding.tour.back": "Back",
  "onboarding.tour.done": "Done",
  "onboarding.tour.close": "Close guide",
  "onboarding.tour.welcome.title": "Welcome to Anya",
  "onboarding.tour.welcome.body":
    "Send one message here first. After that, double-tap Alt in any window to ask without leaving your work.",
  "onboarding.tour.provider.title": "Add an API key",
  "onboarding.tour.provider.body":
    "Paste a DeepSeek API key to start chatting. You can add other models later in Settings. Type it here, then continue.",
  "onboarding.tour.composer.title": "Type a question",
  "onboarding.tour.composer.body": "Write what you want and press Enter.",
  "onboarding.tour.hotkey.title": "Summon from anywhere",
  "onboarding.tour.hotkey.body":
    "Double-tap Alt. A long hold does nothing. Anya opens on the current screen.",
  "onboarding.tour.workspace.title": "Work inside a folder",
  "onboarding.tour.workspace.body":
    "Add a folder when you want Anya to read or edit files. Permission and thinking controls show up then.",
  "onboarding.emptyProviderTitle": "Connect a model to start",
  "onboarding.emptyHotkey": "Double-tap Alt in any window to summon Anya. Don't hold the key.",
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
    "onboarding.tour.next": "下一步",
    "onboarding.tour.back": "上一步",
    "onboarding.tour.done": "完成",
    "onboarding.tour.close": "关闭引导",
    "onboarding.tour.welcome.title": "欢迎使用 Anya",
    "onboarding.tour.welcome.body":
      "先在这里发出第一条消息。之后在任意窗口连按两次 Alt，就可以接着问。",
    "onboarding.tour.provider.title": "填入 API Key",
    "onboarding.tour.provider.body":
      "贴上 DeepSeek 的密钥就能开始对话。其他模型可以稍后在设置里添加。先在这里输入，再继续。",
    "onboarding.tour.composer.title": "在这里输入",
    "onboarding.tour.composer.body": "写下你的问题，按 Enter 发送。",
    "onboarding.tour.hotkey.title": "随时唤出",
    "onboarding.tour.hotkey.body": "快速连按两次 Alt，不要长按。Anya 会在当前屏幕打开。",
    "onboarding.tour.workspace.title": "处理文件夹",
    "onboarding.tour.workspace.body":
      "添加文件夹后，Anya 可以阅读和修改里面的文件。权限和思考强度也会在那时出现。",
    "onboarding.emptyProviderTitle": "先连接一个模型",
    "onboarding.emptyHotkey": "在任意窗口连按两次 Alt 即可唤出，不要长按。",
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
    "onboarding.tour.next": "次へ",
    "onboarding.tour.back": "戻る",
    "onboarding.tour.done": "完了",
    "onboarding.tour.close": "ガイドを閉じる",
    "onboarding.tour.welcome.title": "Anya へようこそ",
    "onboarding.tour.welcome.body":
      "まずここで一文送ってみましょう。その後は、どの画面でも Alt を素早く2回押すと質問できます。",
    "onboarding.tour.provider.title": "API キーを入力",
    "onboarding.tour.provider.body":
      "DeepSeek の API キーを貼り付けると会話を始められます。ほかのモデルはあとから設定で追加できます。ここに入力してから次へ進んでください。",
    "onboarding.tour.composer.title": "ここに入力",
    "onboarding.tour.composer.body": "質問を書いて Enter で送信します。",
    "onboarding.tour.hotkey.title": "どこからでも呼び出す",
    "onboarding.tour.hotkey.body":
      "Alt を素早く2回押します。長押しでは開きません。Anya は今の画面に表示されます。",
    "onboarding.tour.workspace.title": "フォルダを扱う",
    "onboarding.tour.workspace.body":
      "フォルダを追加すると、Anya がその中のファイルを読んだり編集したりできます。権限と思考の強さはそのとき表示されます。",
    "onboarding.emptyProviderTitle": "まずモデルを接続",
    "onboarding.emptyHotkey": "どの画面でも Alt を素早く2回押すと呼び出せます。長押しは無効です。",
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
    "onboarding.tour.next": "Далее",
    "onboarding.tour.back": "Назад",
    "onboarding.tour.done": "Готово",
    "onboarding.tour.close": "Закрыть подсказку",
    "onboarding.tour.welcome.title": "Добро пожаловать в Anya",
    "onboarding.tour.welcome.body":
      "Сначала отправьте одно сообщение здесь. Потом два быстрых нажатия Alt в любом окне откроют Anya.",
    "onboarding.tour.provider.title": "Добавьте ключ API",
    "onboarding.tour.provider.body":
      "Вставьте ключ DeepSeek, чтобы начать диалог. Другие модели можно добавить позже в настройках. Введите ключ здесь и продолжите.",
    "onboarding.tour.composer.title": "Введите вопрос",
    "onboarding.tour.composer.body": "Напишите, что нужно, и нажмите Enter.",
    "onboarding.tour.hotkey.title": "Вызов из любого окна",
    "onboarding.tour.hotkey.body":
      "Два быстрых нажатия Alt. Удержание не сработает. Anya откроется на текущем экране.",
    "onboarding.tour.workspace.title": "Работа с папкой",
    "onboarding.tour.workspace.body":
      "Добавьте папку, если Anya должна читать или менять файлы. Права доступа и уровень мышления появятся тогда.",
    "onboarding.emptyProviderTitle": "Сначала подключите модель",
    "onboarding.emptyHotkey":
      "Два быстрых нажатия Alt в любом окне вызывают Anya. Не удерживайте клавишу.",
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
    "onboarding.tour.next": "Weiter",
    "onboarding.tour.back": "Zurück",
    "onboarding.tour.done": "Fertig",
    "onboarding.tour.close": "Führung schließen",
    "onboarding.tour.welcome.title": "Willkommen bei Anya",
    "onboarding.tour.welcome.body":
      "Schick zuerst hier eine Nachricht. Danach öffnet ein doppeltes Antippen von Alt Anya in jedem Fenster.",
    "onboarding.tour.provider.title": "API-Schlüssel einfügen",
    "onboarding.tour.provider.body":
      "Füge einen DeepSeek-API-Schlüssel ein, um zu chatten. Andere Modelle kannst du später in den Einstellungen hinzufügen. Trag ihn hier ein und geh dann weiter.",
    "onboarding.tour.composer.title": "Hier eingeben",
    "onboarding.tour.composer.body": "Schreib deine Frage und drück Enter.",
    "onboarding.tour.hotkey.title": "Überall aufrufen",
    "onboarding.tour.hotkey.body":
      "Tippe Alt zweimal kurz an. Gedrückt halten öffnet nichts. Anya erscheint auf dem aktuellen Bildschirm.",
    "onboarding.tour.workspace.title": "In einem Ordner arbeiten",
    "onboarding.tour.workspace.body":
      "Füge einen Ordner hinzu, wenn Anya Dateien lesen oder ändern soll. Berechtigungen und Denkintensität erscheinen dann.",
    "onboarding.emptyProviderTitle": "Zuerst ein Modell verbinden",
    "onboarding.emptyHotkey": "Alt zweimal kurz in jedem Fenster antippen. Nicht gedrückt halten.",
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
    "onboarding.tour.next": "Suivant",
    "onboarding.tour.back": "Retour",
    "onboarding.tour.done": "Terminer",
    "onboarding.tour.close": "Fermer le guide",
    "onboarding.tour.welcome.title": "Bienvenue sur Anya",
    "onboarding.tour.welcome.body":
      "Envoyez d'abord un message ici. Ensuite, deux appuis rapides sur Alt ouvrent Anya dans n'importe quelle fenêtre.",
    "onboarding.tour.provider.title": "Ajouter une clé API",
    "onboarding.tour.provider.body":
      "Collez une clé API DeepSeek pour commencer. Vous pourrez ajouter d'autres modèles plus tard dans les réglages. Saisissez-la ici, puis continuez.",
    "onboarding.tour.composer.title": "Écrire ici",
    "onboarding.tour.composer.body": "Écrivez votre question, puis appuyez sur Entrée.",
    "onboarding.tour.hotkey.title": "Appeler de n'importe où",
    "onboarding.tour.hotkey.body":
      "Appuyez deux fois rapidement sur Alt. Un appui long ne fait rien. Anya s'ouvre sur l'écran actuel.",
    "onboarding.tour.workspace.title": "Travailler dans un dossier",
    "onboarding.tour.workspace.body":
      "Ajoutez un dossier pour qu'Anya lise ou modifie des fichiers. Les autorisations et l'intensité de réflexion apparaissent alors.",
    "onboarding.emptyProviderTitle": "Connectez d'abord un modèle",
    "onboarding.emptyHotkey":
      "Deux appuis rapides sur Alt dans n'importe quelle fenêtre. Ne maintenez pas la touche.",
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
    "onboarding.tour.next": "다음",
    "onboarding.tour.back": "이전",
    "onboarding.tour.done": "완료",
    "onboarding.tour.close": "안내 닫기",
    "onboarding.tour.welcome.title": "Anya에 오신 것을 환영합니다",
    "onboarding.tour.welcome.body":
      "먼저 여기서 메시지 하나를 보내 보세요. 이후에는 어느 창에서든 Alt를 빠르게 두 번 누르면 질문할 수 있습니다.",
    "onboarding.tour.provider.title": "API 키 입력",
    "onboarding.tour.provider.body":
      "DeepSeek API 키를 붙여 넣으면 대화를 시작할 수 있습니다. 다른 모델은 나중에 설정에서 추가할 수 있습니다. 여기에 입력한 뒤 계속하세요.",
    "onboarding.tour.composer.title": "여기에 입력",
    "onboarding.tour.composer.body": "질문을 적고 Enter를 누르세요.",
    "onboarding.tour.hotkey.title": "어디서든 부르기",
    "onboarding.tour.hotkey.body":
      "Alt를 빠르게 두 번 누릅니다. 길게 누르면 열리지 않습니다. Anya가 현재 화면에 나타납니다.",
    "onboarding.tour.workspace.title": "폴더 작업",
    "onboarding.tour.workspace.body":
      "폴더를 추가하면 Anya가 그 안의 파일을 읽고 수정할 수 있습니다. 권한과 생각 강도는 그때 나타납니다.",
    "onboarding.emptyProviderTitle": "먼저 모델을 연결하세요",
    "onboarding.emptyHotkey":
      "어느 창에서든 Alt를 빠르게 두 번 누르면 열립니다. 길게 누르지 마세요.",
  },
};
