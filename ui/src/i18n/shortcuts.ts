import type { Translations } from ".";

const en = {
  createShortcut: "Create shortcut",
  createShortcuts: "Create shortcuts",
  desktop: "On the desktop",
  startMenu: "In the Start menu",
  both: "Both",
  created: "Shortcut created",
  createdMany: "{count} shortcuts created",
  failed: "Some shortcuts could not be created: {reasons}",
};

/** Creating desktop and Start menu shortcuts (game menus, multi-select). */
export const shortcutsText: Translations<typeof en> = {
  en,
  de: {
    createShortcut: "Verknüpfung erstellen",
    createShortcuts: "Verknüpfungen erstellen",
    desktop: "Auf dem Desktop",
    startMenu: "Im Startmenü",
    both: "Beides",
    created: "Verknüpfung erstellt",
    createdMany: "{count} Verknüpfungen erstellt",
    failed: "Einige Verknüpfungen konnten nicht erstellt werden: {reasons}",
  },
  fr: {
    createShortcut: "Créer un raccourci",
    createShortcuts: "Créer des raccourcis",
    desktop: "Sur le bureau",
    startMenu: "Dans le menu Démarrer",
    both: "Les deux",
    created: "Raccourci créé",
    createdMany: "{count} raccourcis créés",
    failed: "Certains raccourcis n'ont pas pu être créés : {reasons}",
  },
  it: {
    createShortcut: "Crea collegamento",
    createShortcuts: "Crea collegamenti",
    desktop: "Sul desktop",
    startMenu: "Nel menu Start",
    both: "Entrambi",
    created: "Collegamento creato",
    createdMany: "{count} collegamenti creati",
    failed: "Alcuni collegamenti non sono stati creati: {reasons}",
  },
  es: {
    createShortcut: "Crear acceso directo",
    createShortcuts: "Crear accesos directos",
    desktop: "En el escritorio",
    startMenu: "En el menú Inicio",
    both: "Ambos",
    created: "Acceso directo creado",
    createdMany: "{count} accesos directos creados",
    failed: "No se pudieron crear algunos accesos directos: {reasons}",
  },
  pt: {
    createShortcut: "Criar atalho",
    createShortcuts: "Criar atalhos",
    desktop: "No ambiente de trabalho",
    startMenu: "No menu Iniciar",
    both: "Ambos",
    created: "Atalho criado",
    createdMany: "{count} atalhos criados",
    failed: "Não foi possível criar alguns atalhos: {reasons}",
  },
};
