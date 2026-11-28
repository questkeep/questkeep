import type { Translations } from ".";

const games = (n: number) => `${n} ${n === 1 ? "game" : "games"}`;
const spiele = (n: number) => `${n} ${n === 1 ? "Spiel" : "Spiele"}`;
const jeux = (n: number) => `${n} ${n === 1 ? "jeu" : "jeux"}`;
const giochi = (n: number) => `${n} ${n === 1 ? "gioco" : "giochi"}`;
const juegos = (n: number) => `${n} ${n === 1 ? "juego" : "juegos"}`;
const jogos = (n: number) => `${n} ${n === 1 ? "jogo" : "jogos"}`;

const en = {
  title: "Random game",
  pick: "Pick a random game",
  pickAnother: "Pick another",
  openPage: "Open page",
  onlyInstalled: "Only installed games",
  from: (p: { view: string; count: number }) => `From “${p.view}”: ${games(p.count)}`,
  none: "No game in this view to pick from.",
  noneInstalled: "No installed game in this view. Untick “Only installed games” to pick from all.",
  installed: "Installed",
  notInstalled: "Not installed",
  played: "{time} running time",
  lastStarted: "Last started: {when}",
  neverStarted: "Never started",
};

export const randomGame: Translations<typeof en> = {
  en,
  de: {
    title: "Zufälliges Spiel",
    pick: "Zufälliges Spiel auswählen",
    pickAnother: "Ein anderes",
    openPage: "Seite öffnen",
    onlyInstalled: "Nur installierte Spiele",
    from: (p: { view: string; count: number }) => `Aus „${p.view}“: ${spiele(p.count)}`,
    none: "Diese Ansicht hat kein Spiel zur Auswahl.",
    noneInstalled:
      "Kein installiertes Spiel in dieser Ansicht. Entferne den Haken bei „Nur installierte Spiele“, um aus allen zu wählen.",
    installed: "Installiert",
    notInstalled: "Nicht installiert",
    played: "{time} Laufzeit",
    lastStarted: "Zuletzt gestartet: {when}",
    neverStarted: "Nie gestartet",
  },
  fr: {
    title: "Jeu au hasard",
    pick: "Choisir un jeu au hasard",
    pickAnother: "Un autre",
    openPage: "Ouvrir la page",
    onlyInstalled: "Jeux installés uniquement",
    from: (p: { view: string; count: number }) => `Dans « ${p.view} » : ${jeux(p.count)}`,
    none: "Aucun jeu à choisir dans cette vue.",
    noneInstalled:
      "Aucun jeu installé dans cette vue. Décochez « Jeux installés uniquement » pour choisir parmi tous.",
    installed: "Installé",
    notInstalled: "Non installé",
    played: "{time} de temps de jeu",
    lastStarted: "Dernier lancement : {when}",
    neverStarted: "Jamais lancé",
  },
  it: {
    title: "Gioco a caso",
    pick: "Scegli un gioco a caso",
    pickAnother: "Un altro",
    openPage: "Apri pagina",
    onlyInstalled: "Solo giochi installati",
    from: (p: { view: string; count: number }) => `Da “${p.view}”: ${giochi(p.count)}`,
    none: "Nessun gioco da scegliere in questa vista.",
    noneInstalled:
      "Nessun gioco installato in questa vista. Togli la spunta da “Solo giochi installati” per scegliere tra tutti.",
    installed: "Installato",
    notInstalled: "Non installato",
    played: "{time} di tempo di esecuzione",
    lastStarted: "Ultimo avvio: {when}",
    neverStarted: "Mai avviato",
  },
  es: {
    title: "Juego al azar",
    pick: "Elegir un juego al azar",
    pickAnother: "Otro",
    openPage: "Abrir página",
    onlyInstalled: "Solo juegos instalados",
    from: (p: { view: string; count: number }) => `De «${p.view}»: ${juegos(p.count)}`,
    none: "No hay ningún juego para elegir en esta vista.",
    noneInstalled:
      "No hay juegos instalados en esta vista. Desmarca «Solo juegos instalados» para elegir entre todos.",
    installed: "Instalado",
    notInstalled: "No instalado",
    played: "{time} de tiempo de ejecución",
    lastStarted: "Último inicio: {when}",
    neverStarted: "Nunca iniciado",
  },
  pt: {
    title: "Jogo aleatório",
    pick: "Escolher um jogo aleatório",
    pickAnother: "Outro",
    openPage: "Abrir página",
    onlyInstalled: "Somente jogos instalados",
    from: (p: { view: string; count: number }) => `De “${p.view}”: ${jogos(p.count)}`,
    none: "Nenhum jogo para escolher nesta visualização.",
    noneInstalled:
      "Nenhum jogo instalado nesta visualização. Desmarque “Somente jogos instalados” para escolher entre todos.",
    installed: "Instalado",
    notInstalled: "Não instalado",
    played: "{time} de tempo de execução",
    lastStarted: "Último início: {when}",
    neverStarted: "Nunca iniciado",
  },
};
