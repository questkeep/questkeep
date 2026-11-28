import type { Translations } from ".";

const en = {
  title: "Excluded games",
  intro:
    "Games you removed with “Don't import again when syncing”. Syncs skip them until you import them again.",
  empty: "No excluded games.",
  list: "Excluded games",
  excludedOn: "{store} · removed {date}",
  importAgain: "Import again",
  importAgainLabel: "Import {name} again",
  importing: "Importing “{name}” again: syncing {store}…",
};

export const settingsExclusions: Translations<typeof en> = {
  en,
  de: {
    title: "Ausgeschlossene Spiele",
    intro:
      "Spiele, die du mit „Beim Synchronisieren nicht wieder importieren“ entfernt hast. Synchronisierungen überspringen sie, bis du sie wieder importierst.",
    empty: "Keine ausgeschlossenen Spiele.",
    list: "Ausgeschlossene Spiele",
    excludedOn: "{store} · entfernt am {date}",
    importAgain: "Wieder importieren",
    importAgainLabel: "{name} wieder importieren",
    importing: "„{name}“ wird wieder importiert: {store} wird synchronisiert…",
  },
  fr: {
    title: "Jeux exclus",
    intro:
      "Jeux retirés avec « Ne plus importer lors de la synchronisation ». Les synchronisations les ignorent jusqu'à ce que vous les importiez à nouveau.",
    empty: "Aucun jeu exclu.",
    list: "Jeux exclus",
    excludedOn: "{store} · retiré le {date}",
    importAgain: "Importer à nouveau",
    importAgainLabel: "Importer {name} à nouveau",
    importing: "Réimportation de « {name} » : synchronisation de {store}…",
  },
  it: {
    title: "Giochi esclusi",
    intro:
      "Giochi rimossi con “Non importare di nuovo durante la sincronizzazione”. Le sincronizzazioni li saltano finché non li importi di nuovo.",
    empty: "Nessun gioco escluso.",
    list: "Giochi esclusi",
    excludedOn: "{store} · rimosso il {date}",
    importAgain: "Importa di nuovo",
    importAgainLabel: "Importa di nuovo {name}",
    importing: "Reimportazione di “{name}”: sincronizzazione di {store}…",
  },
  es: {
    title: "Juegos excluidos",
    intro:
      "Juegos que quitaste con «No volver a importar al sincronizar». Las sincronizaciones los omiten hasta que los importes de nuevo.",
    empty: "No hay juegos excluidos.",
    list: "Juegos excluidos",
    excludedOn: "{store} · quitado el {date}",
    importAgain: "Importar de nuevo",
    importAgainLabel: "Importar {name} de nuevo",
    importing: "Importando «{name}» de nuevo: sincronizando {store}…",
  },
  pt: {
    title: "Jogos excluídos",
    intro:
      "Jogos que você removeu com “Não importar novamente ao sincronizar”. As sincronizações os ignoram até que você os importe novamente.",
    empty: "Nenhum jogo excluído.",
    list: "Jogos excluídos",
    excludedOn: "{store} · removido em {date}",
    importAgain: "Importar novamente",
    importAgainLabel: "Importar {name} novamente",
    importing: "Importando “{name}” novamente: sincronizando {store}…",
  },
};
