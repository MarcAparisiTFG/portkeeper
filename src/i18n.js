// UI strings. Keys are English; `t(key, vars)` picks the active language.
const STRINGS = {
  en: {
    active: "Active", closed: "Closed", settings: "Settings", minimize: "Minimize to the menu bar",
    appearance: "Appearance", glass: "Glass", dark: "Dark", light: "Light",
    language: "Language", system: "System",
    launchAtLogin: "Launch at login", openOnStart: "Open widget on start", openOnStartHint: "Off: stays in the menu bar until you click it", closeOnBlur: "Close on click outside", closeOnBlurHint: "The minimize button goes away",
    showAll: "Show all processes", showAllHint: "Include app ports like Spotify or editor helpers",
    checkUpdates: "Check for updates", checkUpdatesHint: "Asks GitHub for new versions. Nothing else leaves your computer", updateAvailable: "Version {v} is available", update: "Update", updating: "updating…", whatsNew: "What's new", hide: "Hide", allChanges: "All changes on GitHub",
    lockedProjects: "Locked projects", lockedHint: "Lock a project from its row so it can't be stopped or removed by mistake.",
    unlock: "Unlock", lock: "Lock (no stop / remove)", quit: "Quit", coffee: "Buy me a coffee",
    stop: "Stop", stopAll: "Stop all", forget: "Forget", forgetAll: "Forget all", remove: "Remove", startAgain: "Start again",
    nothingRunning: "Nothing running", nothingClosed: "Nothing closed", open: "Open",
    starting: "starting…", stopping: "stopping…", notResponding: "Listening, not responding yet", runningSince: "Running since {date} · {up}",
    stopQ: "Stop {x}?", forgetQ: "Forget {x}?", forgetAllQ: "Forget all {n} closed projects?", removeQ: "Remove {x}?",
    lockedTip: "Locked: Portkeeper will not stop it", startedBy: "Started by {x}", didntStart: "Didn't start · log: {log}",
    openUrl: "Open {url}", stopContainer: "Stop container {x}", stopX: "Stop {x}", locked: "Locked",
    stoppedContainers: "{n} stopped", now: "now",
  },
  es: {
    active: "Activos", closed: "Cerrados", settings: "Ajustes", minimize: "Minimizar a la barra de menús",
    appearance: "Apariencia", glass: "Cristal", dark: "Oscuro", light: "Claro",
    language: "Idioma", system: "Sistema",
    launchAtLogin: "Abrir al iniciar sesión", openOnStart: "Mostrar el widget al arrancar", openOnStartHint: "Desactivado: se queda en la barra de menús hasta que lo pulses", closeOnBlur: "Cerrar al pulsar fuera", closeOnBlurHint: "Desaparece el botón de minimizar",
    showAll: "Mostrar todos los procesos", showAllHint: "Incluye puertos de apps como Spotify o ayudantes del editor",
    checkUpdates: "Buscar actualizaciones", checkUpdatesHint: "Pregunta a GitHub por versiones nuevas. Nada más sale de tu ordenador", updateAvailable: "Versión {v} disponible", update: "Actualizar", updating: "actualizando…", whatsNew: "Novedades", hide: "Ocultar", allChanges: "Todos los cambios en GitHub",
    lockedProjects: "Proyectos bloqueados", lockedHint: "Bloquea un proyecto desde su fila para que no se pueda parar ni borrar por error.",
    unlock: "Desbloquear", lock: "Bloquear (sin parar / borrar)", quit: "Salir", coffee: "Invítame a un café",
    stop: "Parar", stopAll: "Parar todo", forget: "Olvidar", forgetAll: "Olvidar todo", remove: "Eliminar", startAgain: "Volver a arrancar",
    nothingRunning: "Nada en marcha", nothingClosed: "Nada cerrado", open: "Abrir",
    starting: "arrancando…", stopping: "parando…", notResponding: "Escuchando, aún no responde", runningSince: "En marcha desde {date} · {up}",
    stopQ: "¿Parar {x}?", forgetQ: "¿Olvidar {x}?", forgetAllQ: "¿Olvidar los {n} proyectos cerrados?", removeQ: "¿Eliminar {x}?",
    lockedTip: "Bloqueado: Portkeeper no lo parará", startedBy: "Lanzado por {x}", didntStart: "No arrancó · log: {log}",
    openUrl: "Abrir {url}", stopContainer: "Parar contenedor {x}", stopX: "Parar {x}", locked: "Bloqueado",
    stoppedContainers: "{n} parados", now: "ahora",
  },
  fr: {
    active: "Actifs", closed: "Fermés", settings: "Réglages", minimize: "Réduire dans la barre de menus",
    appearance: "Apparence", glass: "Verre", dark: "Sombre", light: "Clair",
    language: "Langue", system: "Système",
    launchAtLogin: "Lancer à l'ouverture de session", openOnStart: "Afficher le widget au démarrage", openOnStartHint: "Désactivé : reste dans la barre de menus jusqu'au clic", closeOnBlur: "Fermer au clic à l'extérieur", closeOnBlurHint: "Le bouton de réduction disparaît",
    showAll: "Afficher tous les processus", showAllHint: "Inclut les ports d'apps comme Spotify ou les aides de l'éditeur",
    checkUpdates: "Rechercher des mises à jour", checkUpdatesHint: "Demande à GitHub les nouvelles versions. Rien d'autre ne quitte votre ordinateur", updateAvailable: "Version {v} disponible", update: "Mettre à jour", updating: "mise à jour…", whatsNew: "Nouveautés", hide: "Masquer", allChanges: "Tous les changements sur GitHub",
    lockedProjects: "Projets verrouillés", lockedHint: "Verrouillez un projet depuis sa ligne pour éviter de l'arrêter ou de le supprimer par erreur.",
    unlock: "Déverrouiller", lock: "Verrouiller (ni arrêt, ni suppression)", quit: "Quitter", coffee: "Offrez-moi un café",
    stop: "Arrêter", stopAll: "Tout arrêter", forget: "Oublier", forgetAll: "Tout oublier", remove: "Supprimer", startAgain: "Relancer",
    nothingRunning: "Rien en cours", nothingClosed: "Rien de fermé", open: "Ouvrir",
    starting: "démarrage…", stopping: "arrêt…", notResponding: "À l'écoute, ne répond pas encore", runningSince: "En cours depuis {date} · {up}",
    stopQ: "Arrêter {x} ?", forgetQ: "Oublier {x} ?", forgetAllQ: "Oublier les {n} projets fermés ?", removeQ: "Supprimer {x} ?",
    lockedTip: "Verrouillé : Portkeeper ne l'arrêtera pas", startedBy: "Lancé par {x}", didntStart: "N'a pas démarré · log : {log}",
    openUrl: "Ouvrir {url}", stopContainer: "Arrêter le conteneur {x}", stopX: "Arrêter {x}", locked: "Verrouillé",
    stoppedContainers: "{n} arrêtés", now: "maintenant",
  },
  de: {
    active: "Aktiv", closed: "Beendet", settings: "Einstellungen", minimize: "In die Menüleiste minimieren",
    appearance: "Darstellung", glass: "Glas", dark: "Dunkel", light: "Hell",
    language: "Sprache", system: "System",
    launchAtLogin: "Bei Anmeldung starten", openOnStart: "Widget beim Start öffnen", openOnStartHint: "Aus: bleibt in der Menüleiste, bis du darauf klickst", closeOnBlur: "Bei Klick außerhalb schließen", closeOnBlurHint: "Die Minimieren-Taste entfällt",
    showAll: "Alle Prozesse anzeigen", showAllHint: "Auch App-Ports wie Spotify oder Editor-Helfer",
    checkUpdates: "Nach Updates suchen", checkUpdatesHint: "Fragt GitHub nach neuen Versionen. Sonst verlässt nichts deinen Computer", updateAvailable: "Version {v} ist verfügbar", update: "Aktualisieren", updating: "wird aktualisiert…", whatsNew: "Neuigkeiten", hide: "Ausblenden", allChanges: "Alle Änderungen auf GitHub",
    lockedProjects: "Gesperrte Projekte", lockedHint: "Sperre ein Projekt in seiner Zeile, damit es nicht versehentlich gestoppt oder entfernt wird.",
    unlock: "Entsperren", lock: "Sperren (kein Stopp / Entfernen)", quit: "Beenden", coffee: "Spendier mir einen Kaffee",
    stop: "Stoppen", stopAll: "Alle stoppen", forget: "Vergessen", forgetAll: "Alle vergessen", remove: "Entfernen", startAgain: "Erneut starten",
    nothingRunning: "Nichts läuft", nothingClosed: "Nichts beendet", open: "Öffnen",
    starting: "startet…", stopping: "stoppt…", notResponding: "Lauscht, antwortet noch nicht", runningSince: "Läuft seit {date} · {up}",
    stopQ: "{x} stoppen?", forgetQ: "{x} vergessen?", forgetAllQ: "Alle {n} beendeten Projekte vergessen?", removeQ: "{x} entfernen?",
    lockedTip: "Gesperrt: Portkeeper stoppt es nicht", startedBy: "Gestartet von {x}", didntStart: "Nicht gestartet · Log: {log}",
    openUrl: "{url} öffnen", stopContainer: "Container {x} stoppen", stopX: "{x} stoppen", locked: "Gesperrt",
    stoppedContainers: "{n} gestoppt", now: "jetzt",
  },
  pt: {
    active: "Ativos", closed: "Fechados", settings: "Definições", minimize: "Minimizar para a barra de menus",
    appearance: "Aparência", glass: "Vidro", dark: "Escuro", light: "Claro",
    language: "Idioma", system: "Sistema",
    launchAtLogin: "Abrir ao iniciar sessão", openOnStart: "Mostrar o widget ao arrancar", openOnStartHint: "Desligado: fica na barra de menus até clicares", closeOnBlur: "Fechar ao clicar fora", closeOnBlurHint: "O botão de minimizar desaparece",
    showAll: "Mostrar todos os processos", showAllHint: "Inclui portas de apps como Spotify ou ajudantes do editor",
    checkUpdates: "Procurar atualizações", checkUpdatesHint: "Pergunta ao GitHub por novas versões. Mais nada sai do teu computador", updateAvailable: "Versão {v} disponível", update: "Atualizar", updating: "a atualizar…", whatsNew: "Novidades", hide: "Ocultar", allChanges: "Todas as alterações no GitHub",
    lockedProjects: "Projetos bloqueados", lockedHint: "Bloqueia um projeto na sua linha para não o parar nem remover por engano.",
    unlock: "Desbloquear", lock: "Bloquear (sem parar / remover)", quit: "Sair", coffee: "Paga-me um café",
    stop: "Parar", stopAll: "Parar tudo", forget: "Esquecer", forgetAll: "Esquecer tudo", remove: "Remover", startAgain: "Arrancar de novo",
    nothingRunning: "Nada em execução", nothingClosed: "Nada fechado", open: "Abrir",
    starting: "a arrancar…", stopping: "a parar…", notResponding: "À escuta, ainda não responde", runningSince: "Em execução desde {date} · {up}",
    stopQ: "Parar {x}?", forgetQ: "Esquecer {x}?", forgetAllQ: "Esquecer os {n} projetos fechados?", removeQ: "Remover {x}?",
    lockedTip: "Bloqueado: o Portkeeper não o vai parar", startedBy: "Iniciado por {x}", didntStart: "Não arrancou · log: {log}",
    openUrl: "Abrir {url}", stopContainer: "Parar o contentor {x}", stopX: "Parar {x}", locked: "Bloqueado",
    stoppedContainers: "{n} parados", now: "agora",
  },
};
const LANGS = [["", "System"], ["en", "English"], ["es", "Español"], ["fr", "Français"], ["de", "Deutsch"], ["pt", "Português"]];
let currentLang = "en";
function setLanguage(code) {
  const sys = (navigator.language || "en").slice(0, 2).toLowerCase();
  currentLang = STRINGS[code] ? code : STRINGS[sys] ? sys : "en";
  document.documentElement.lang = currentLang;
}
function t(key, vars = {}) {
  const s = (STRINGS[currentLang] || STRINGS.en)[key] ?? STRINGS.en[key] ?? key;
  return s.replace(/\{(\w+)\}/g, (_, k) => (vars[k] ?? ""));
}
