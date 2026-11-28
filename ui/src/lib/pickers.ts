import { open } from "@tauri-apps/plugin-dialog";
import { translator } from "../i18n";
import { scriptsText } from "../i18n/scripts";
import { shellCommon } from "../i18n/shellCommon";

const tr = translator(shellCommon);
const trScripts = translator(scriptsText);

async function pickFile(title: string, filters: { name: string; extensions: string[] }[]) {
  const result = await open({ multiple: false, directory: false, title, filters });
  return typeof result === "string" ? result : null;
}

export function pickExecutable(): Promise<string | null> {
  return pickFile(tr("chooseExecutable"), [
    { name: tr("filterExecutable"), extensions: ["exe", "bat", "cmd", "lnk"] },
    { name: tr("filterAllFiles"), extensions: ["*"] },
  ]);
}

export function pickImage(): Promise<string | null> {
  return pickFile(tr("chooseCover"), [
    { name: tr("filterImage"), extensions: ["jpg", "jpeg", "png", "webp", "gif"] },
  ]);
}

export async function pickDirectory(): Promise<string | null> {
  const result = await open({ multiple: false, directory: true, title: tr("chooseFolder") });
  return typeof result === "string" ? result : null;
}

/** An image for a game's cover, background or icon (icons also from .ico, .exe and .dll). */
export function pickGameImage(icon: boolean): Promise<string | null> {
  return pickFile(icon ? tr("chooseIcon") : tr("chooseImage"), [
    icon
      ? { name: tr("filterIconOrImage"), extensions: ["png", "jpg", "jpeg", "ico", "exe", "dll"] }
      : { name: tr("filterImage"), extensions: ["png", "jpg", "jpeg", "webp"] },
  ]);
}

/** A tool's icon: a picture, or a program whose icon is taken. */
export function pickToolIcon(): Promise<string | null> {
  return pickFile(tr("chooseIcon"), [
    {
      name: tr("filterIconOrImage"),
      extensions: ["png", "jpg", "jpeg", "webp", "gif", "ico", "svg", "exe"],
    },
  ]);
}

/** A Questkeep backup archive to import. */
export function pickBackupFile(): Promise<string | null> {
  return pickFile(tr("chooseBackup"), [
    { name: tr("filterBackup"), extensions: ["zip"] },
    { name: tr("filterAllFiles"), extensions: ["*"] },
  ]);
}

/** A game's manual: a document, or any file. */
export function pickManualFile(): Promise<string | null> {
  return pickFile(tr("chooseManual"), [
    { name: tr("filterManual"), extensions: ["pdf", "txt", "md", "html", "htm", "doc", "docx"] },
    { name: tr("filterAllFiles"), extensions: ["*"] },
  ]);
}

/** A script file to link or import. */
export function pickScriptFile(): Promise<string | null> {
  return pickFile(trScripts("chooseScriptFile"), [
    { name: trScripts("filterScripts"), extensions: ["ps1", "sh", "cmd", "bat", "py"] },
    { name: trScripts("filterAllFiles"), extensions: ["*"] },
  ]);
}
