import type { AppLanguage } from "@/types/setting";

export const slashEn = {
  "slash.history.description": "Open chat history",
  "slash.model.description": "Switch the chat model",
  "slash.thinking.description": "Adjust thinking effort",
  "slash.settings.description": "Open settings",
  "slash.work.description": "Switch workspace",
  "slash.exit.description": "Close the current chat",
  "slash.context.description": "Show the current environment context",
  "slash.clear.description": "Clear the current input and local draft",
} as const;

export type SlashI18nKey = keyof typeof slashEn;
type SlashLocale = Partial<Record<SlashI18nKey, string>>;

export const slashLocales: Record<AppLanguage, SlashLocale> = {
  "en-US": slashEn,
  "zh-CN": {
    "slash.history.description": "打开历史对话",
    "slash.model.description": "切换对话模型",
    "slash.thinking.description": "调整思考强度",
    "slash.settings.description": "打开设置",
    "slash.work.description": "切换工作区",
    "slash.exit.description": "关闭当前会话",
    "slash.context.description": "显示当前环境上下文",
    "slash.clear.description": "清空当前输入与本地草稿",
  },
  "ja-JP": {
    "slash.history.description": "チャット履歴を開く",
    "slash.model.description": "チャットモデルを切り替える",
    "slash.thinking.description": "思考の強さを調整する",
    "slash.settings.description": "設定を開く",
    "slash.work.description": "ワークスペースを切り替える",
    "slash.exit.description": "現在のチャットを閉じる",
    "slash.context.description": "現在の環境コンテキストを表示する",
    "slash.clear.description": "現在の入力とローカル下書きを消去する",
  },
  "ru-RU": {
    "slash.history.description": "Открыть историю чатов",
    "slash.model.description": "Сменить модель чата",
    "slash.thinking.description": "Настроить глубину рассуждений",
    "slash.settings.description": "Открыть настройки",
    "slash.work.description": "Сменить рабочую область",
    "slash.exit.description": "Закрыть текущий чат",
    "slash.context.description": "Показать текущий контекст среды",
    "slash.clear.description": "Очистить текущий ввод и локальный черновик",
  },
  "de-DE": {
    "slash.history.description": "Chatverlauf öffnen",
    "slash.model.description": "Chatmodell wechseln",
    "slash.thinking.description": "Denkintensität anpassen",
    "slash.settings.description": "Einstellungen öffnen",
    "slash.work.description": "Arbeitsbereich wechseln",
    "slash.exit.description": "Aktuellen Chat schließen",
    "slash.context.description": "Aktuellen Umgebungskontext anzeigen",
    "slash.clear.description": "Aktuelle Eingabe und lokalen Entwurf leeren",
  },
  "fr-FR": {
    "slash.history.description": "Ouvrir l'historique des discussions",
    "slash.model.description": "Changer de modèle de discussion",
    "slash.thinking.description": "Régler l'intensité de réflexion",
    "slash.settings.description": "Ouvrir les paramètres",
    "slash.work.description": "Changer d'espace de travail",
    "slash.exit.description": "Fermer la discussion actuelle",
    "slash.context.description": "Afficher le contexte actuel de l'environnement",
    "slash.clear.description": "Effacer la saisie actuelle et le brouillon local",
  },
  "ko-KR": {
    "slash.history.description": "채팅 기록 열기",
    "slash.model.description": "채팅 모델 전환",
    "slash.thinking.description": "사고 강도 조절",
    "slash.settings.description": "설정 열기",
    "slash.work.description": "작업 영역 전환",
    "slash.exit.description": "현재 채팅 닫기",
    "slash.context.description": "현재 환경 컨텍스트 표시",
    "slash.clear.description": "현재 입력 및 로컬 초안 지우기",
  },
};
