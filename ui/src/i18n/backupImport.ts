import type { Translations } from ".";

const en = {
  importTitle: "Import this backup?",
  importMessage:
    "Questkeep replaces the library, settings and images with the backup from {date}, made by Questkeep {version}, and restarts to finish. Your current library is backed up first. API keys and store sign-ins are not part of a backup and stay as they are.",
  importConfirm: "Import and restart",
  restoreTitle: "Restore this backup?",
  restoreMessage:
    "Questkeep takes the library, settings and images from the backup from {date}, made by Questkeep {version}, and restarts to finish. API keys and store sign-ins are not part of a backup and stay as they are.",
  restoreConfirm: "Restore and restart",
  askTitle: "Back up the current library first?",
  askMessage:
    "Your library already has games. Questkeep replaces it with the backup from {date}, made by Questkeep {version}, and restarts to finish. Back it up first to be able to go back to it later.",
  backUpAndRestore: "Back up and restore",
  restoreWithoutBackup: "Restore without backup",
  cancel: "Cancel",
  backingUp: "Backing up the current library, then Questkeep restarts to finish…",
  restarting: "Questkeep restarts to finish restoring the backup…",
};

/** Importing a backup archive from anywhere on disk (BackupImportDialog). */
export const backupImport: Translations<typeof en> = {
  en,
  de: {
    importTitle: "Dieses Backup importieren?",
    importMessage:
      "Questkeep ersetzt Bibliothek, Einstellungen und Bilder durch das Backup vom {date}, erstellt mit Questkeep {version}, und startet zum Abschluss neu. Deine aktuelle Bibliothek wird zuvor gesichert. API-Schlüssel und Store-Anmeldungen gehören nicht zu einem Backup und bleiben unverändert.",
    importConfirm: "Importieren und neu starten",
    restoreTitle: "Dieses Backup wiederherstellen?",
    restoreMessage:
      "Questkeep übernimmt Bibliothek, Einstellungen und Bilder aus dem Backup vom {date}, erstellt mit Questkeep {version}, und startet zum Abschluss neu. API-Schlüssel und Store-Anmeldungen gehören nicht zu einem Backup und bleiben unverändert.",
    restoreConfirm: "Wiederherstellen und neu starten",
    askTitle: "Aktuelle Bibliothek zuerst sichern?",
    askMessage:
      "Deine Bibliothek enthält bereits Spiele. Questkeep ersetzt sie durch das Backup vom {date}, erstellt mit Questkeep {version}, und startet zum Abschluss neu. Sichere sie zuerst, um später zu ihr zurückkehren zu können.",
    backUpAndRestore: "Sichern und wiederherstellen",
    restoreWithoutBackup: "Ohne Sicherung wiederherstellen",
    cancel: "Abbrechen",
    backingUp:
      "Die aktuelle Bibliothek wird gesichert, danach startet Questkeep zum Abschluss neu…",
    restarting: "Questkeep startet neu, um die Wiederherstellung abzuschließen…",
  },
  fr: {
    importTitle: "Importer cette sauvegarde ?",
    importMessage:
      "Questkeep remplace la bibliothèque, les paramètres et les images par la sauvegarde du {date}, créée avec Questkeep {version}, puis redémarre pour terminer. Votre bibliothèque actuelle est d'abord sauvegardée. Les clés API et les connexions aux boutiques ne font pas partie d'une sauvegarde et restent inchangées.",
    importConfirm: "Importer et redémarrer",
    restoreTitle: "Restaurer cette sauvegarde ?",
    restoreMessage:
      "Questkeep reprend la bibliothèque, les paramètres et les images de la sauvegarde du {date}, créée avec Questkeep {version}, puis redémarre pour terminer. Les clés API et les connexions aux boutiques ne font pas partie d'une sauvegarde et restent inchangées.",
    restoreConfirm: "Restaurer et redémarrer",
    askTitle: "Sauvegarder d'abord la bibliothèque actuelle ?",
    askMessage:
      "Votre bibliothèque contient déjà des jeux. Questkeep la remplace par la sauvegarde du {date}, créée avec Questkeep {version}, puis redémarre pour terminer. Sauvegardez-la d'abord pour pouvoir y revenir plus tard.",
    backUpAndRestore: "Sauvegarder et restaurer",
    restoreWithoutBackup: "Restaurer sans sauvegarder",
    cancel: "Annuler",
    backingUp: "Sauvegarde de la bibliothèque actuelle, puis Questkeep redémarre pour terminer…",
    restarting: "Questkeep redémarre pour terminer la restauration…",
  },
  it: {
    importTitle: "Importare questo backup?",
    importMessage:
      "Questkeep sostituisce libreria, impostazioni e immagini con il backup del {date}, creato con Questkeep {version}, e si riavvia per completare. La libreria attuale viene prima salvata. Le chiavi API e gli accessi agli store non fanno parte di un backup e restano invariati.",
    importConfirm: "Importa e riavvia",
    restoreTitle: "Ripristinare questo backup?",
    restoreMessage:
      "Questkeep prende libreria, impostazioni e immagini dal backup del {date}, creato con Questkeep {version}, e si riavvia per completare. Le chiavi API e gli accessi agli store non fanno parte di un backup e restano invariati.",
    restoreConfirm: "Ripristina e riavvia",
    askTitle: "Salvare prima la libreria attuale?",
    askMessage:
      "La tua libreria contiene già dei giochi. Questkeep la sostituisce con il backup del {date}, creato con Questkeep {version}, e si riavvia per completare. Salvala prima per poterci tornare in seguito.",
    backUpAndRestore: "Salva e ripristina",
    restoreWithoutBackup: "Ripristina senza backup",
    cancel: "Annulla",
    backingUp: "Backup della libreria attuale, poi Questkeep si riavvia per completare…",
    restarting: "Questkeep si riavvia per completare il ripristino…",
  },
  es: {
    importTitle: "¿Importar esta copia?",
    importMessage:
      "Questkeep sustituye la biblioteca, la configuración y las imágenes por la copia del {date}, creada con Questkeep {version}, y se reinicia para terminar. Antes se hace una copia de tu biblioteca actual. Las claves API y los inicios de sesión de las tiendas no forman parte de una copia y se quedan como están.",
    importConfirm: "Importar y reiniciar",
    restoreTitle: "¿Restaurar esta copia?",
    restoreMessage:
      "Questkeep toma la biblioteca, la configuración y las imágenes de la copia del {date}, creada con Questkeep {version}, y se reinicia para terminar. Las claves API y los inicios de sesión de las tiendas no forman parte de una copia y se quedan como están.",
    restoreConfirm: "Restaurar y reiniciar",
    askTitle: "¿Hacer antes una copia de la biblioteca actual?",
    askMessage:
      "Tu biblioteca ya tiene juegos. Questkeep la sustituye por la copia del {date}, creada con Questkeep {version}, y se reinicia para terminar. Haz antes una copia para poder volver a ella más tarde.",
    backUpAndRestore: "Copiar y restaurar",
    restoreWithoutBackup: "Restaurar sin copia",
    cancel: "Cancelar",
    backingUp: "Copiando la biblioteca actual; después Questkeep se reinicia para terminar…",
    restarting: "Questkeep se reinicia para terminar de restaurar la copia…",
  },
  pt: {
    importTitle: "Importar este backup?",
    importMessage:
      "O Questkeep substitui a biblioteca, as configurações e as imagens pelo backup de {date}, feito com o Questkeep {version}, e reinicia para concluir. A biblioteca atual passa antes por um backup. Chaves de API e logins das lojas não fazem parte de um backup e continuam como estão.",
    importConfirm: "Importar e reiniciar",
    restoreTitle: "Restaurar este backup?",
    restoreMessage:
      "O Questkeep usa a biblioteca, as configurações e as imagens do backup de {date}, feito com o Questkeep {version}, e reinicia para concluir. Chaves de API e logins das lojas não fazem parte de um backup e continuam como estão.",
    restoreConfirm: "Restaurar e reiniciar",
    askTitle: "Fazer backup da biblioteca atual antes?",
    askMessage:
      "Sua biblioteca já tem jogos. O Questkeep a substitui pelo backup de {date}, feito com o Questkeep {version}, e reinicia para concluir. Faça um backup dela antes para poder voltar a ela depois.",
    backUpAndRestore: "Fazer backup e restaurar",
    restoreWithoutBackup: "Restaurar sem backup",
    cancel: "Cancelar",
    backingUp: "Fazendo backup da biblioteca atual; depois o Questkeep reinicia para concluir…",
    restarting: "O Questkeep reinicia para concluir a restauração…",
  },
};
