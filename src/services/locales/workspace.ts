import type { AppLanguage } from "@/types/setting";

export const workspaceEn = {
  "workspace.title": "Workspaces",
  "workspace.description": "The current workspace is the project root for new conversations.",
  "workspace.newWorkspace": "New Workspace",
  "workspace.empty": "No workspaces yet.",
  "workspace.current": "Current",
  "workspace.archiveWorkspace": "Archive workspace",
  "workspace.deleteWorkspace": "Delete workspace",
  "workspace.cancel": "Cancel",
  "workspace.confirmDelete": "Delete",
  "workspace.deleteConfirm": 'Delete workspace "{name}"? Project files will not be deleted.',
} as const;

export type WorkspaceI18nKey = keyof typeof workspaceEn;

export type WorkspaceLocalePartial = Partial<Record<WorkspaceI18nKey, string>>;

export const workspaceLocales: Record<AppLanguage, WorkspaceLocalePartial> = {
  "en-US": workspaceEn,
  "zh-CN": {
    "workspace.title": "工作区",
    "workspace.description": "当前工作区将作为新对话的项目根目录。",
    "workspace.newWorkspace": "新建工作区",
    "workspace.empty": "暂无工作区",
    "workspace.current": "当前",
    "workspace.archiveWorkspace": "归档工作区",
    "workspace.deleteWorkspace": "删除工作区",
    "workspace.cancel": "取消",
    "workspace.confirmDelete": "删除",
    "workspace.deleteConfirm": "确定删除工作区“{name}”吗？项目文件不会被删除。",
  },
  "ja-JP": {
    "workspace.title": "ワークスペース",
    "workspace.description": "現在のワークスペースは新しいチャットのプロジェクトルートになります。",
    "workspace.newWorkspace": "新しいワークスペース",
    "workspace.empty": "ワークスペースがありません",
    "workspace.current": "現在",
    "workspace.archiveWorkspace": "ワークスペースをアーカイブ",
    "workspace.deleteWorkspace": "ワークスペースを削除",
    "workspace.cancel": "キャンセル",
    "workspace.confirmDelete": "削除",
    "workspace.deleteConfirm":
      "ワークスペース「{name}」を削除しますか？プロジェクトファイルは削除されません。",
  },
  "ru-RU": {
    "workspace.title": "Рабочие области",
    "workspace.description":
      "Текущая рабочая область используется как корень проекта для новых чатов.",
    "workspace.newWorkspace": "Новая рабочая область",
    "workspace.empty": "Рабочих областей нет",
    "workspace.current": "Текущая",
    "workspace.archiveWorkspace": "Архивировать рабочую область",
    "workspace.deleteWorkspace": "Удалить рабочую область",
    "workspace.cancel": "Отмена",
    "workspace.confirmDelete": "Удалить",
    "workspace.deleteConfirm": "Удалить рабочую область «{name}»? Файлы проекта удалены не будут.",
  },
  "de-DE": {
    "workspace.title": "Arbeitsbereiche",
    "workspace.description":
      "Der aktuelle Arbeitsbereich ist das Projektstammverzeichnis für neue Chats.",
    "workspace.newWorkspace": "Neuer Arbeitsbereich",
    "workspace.empty": "Keine Arbeitsbereiche",
    "workspace.current": "Aktuell",
    "workspace.archiveWorkspace": "Arbeitsbereich archivieren",
    "workspace.deleteWorkspace": "Arbeitsbereich löschen",
    "workspace.cancel": "Abbrechen",
    "workspace.confirmDelete": "Löschen",
    "workspace.deleteConfirm":
      "Arbeitsbereich „{name}“ löschen? Projektdateien werden nicht gelöscht.",
  },
  "fr-FR": {
    "workspace.title": "Espaces de travail",
    "workspace.description":
      "L’espace de travail actuel est la racine du projet pour les nouvelles discussions.",
    "workspace.newWorkspace": "Nouvel espace de travail",
    "workspace.empty": "Aucun espace de travail",
    "workspace.current": "Actuel",
    "workspace.archiveWorkspace": "Archiver l’espace de travail",
    "workspace.deleteWorkspace": "Supprimer l’espace de travail",
    "workspace.cancel": "Annuler",
    "workspace.confirmDelete": "Supprimer",
    "workspace.deleteConfirm":
      "Supprimer l’espace de travail « {name} » ? Les fichiers du projet ne seront pas supprimés.",
  },
  "ko-KR": {
    "workspace.title": "작업 영역",
    "workspace.description": "현재 작업 영역은 새 채팅의 프로젝트 루트로 사용됩니다.",
    "workspace.newWorkspace": "새 작업 영역",
    "workspace.empty": "작업 영역이 없습니다",
    "workspace.current": "현재",
    "workspace.archiveWorkspace": "작업 영역 보관",
    "workspace.deleteWorkspace": "작업 영역 삭제",
    "workspace.cancel": "취소",
    "workspace.confirmDelete": "삭제",
    "workspace.deleteConfirm":
      "작업 영역 ‘{name}’을 삭제할까요? 프로젝트 파일은 삭제되지 않습니다.",
  },
};
