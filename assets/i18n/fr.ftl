# Textes de l'interface d'Ettin Edit en français.
# Traduction automatique : elle doit être relue par un locuteur natif.
#
# Each line is `id = text`. Translations copy the ids and change the text.
# Text in { $braces } is filled in by the app; keep those names as they are.
# Fluent syntax: https://projectfluent.org/fluent/guide/

# Le nom de la langue dans sa propre langue, affiché dans les Préférences.
language-name = Français

## Fenêtre principale : barre de menus, onglets, barre d'état, dialogues (src/app.rs)
theme-problem = Le thème a un problème : { $error }
theme-save-failed = Impossible d'enregistrer le thème dans { $path } : { $error }
open-lossy = { $path } contient des octets non valides en { $encoding }. Ils sont affichés comme � et seront remplacés à l'enregistrement. Si le texte semble incorrect, choisissez son encodage dans Encodage > Rouvrir avec.
open-failed = Impossible d'ouvrir { $path } : { $error }
dialog-open = Ouvrir
dialog-save-as = Enregistrer sous
save-failed = Impossible d'enregistrer { $path } : { $error }
rename-bad-name = Le nom ne peut pas contenir / ni \.
rename-exists = « { $name } » existe déjà dans ce dossier.
rename-failed = Impossible de renommer le fichier : { $error }
not-saved-yet = { $name } (pas encore enregistré)
restore-themes-failed = Impossible de restaurer les thèmes intégrés : { $error }
reload-failed = Impossible de recharger { $name } : { $error }
reopen-lossy = Certains octets de { $name } ne sont pas valides en { $encoding }. Ils sont affichés comme �.
command-palette-title = Palette de commandes
command-palette-hint = Tapez une partie du nom d'une commande
tab-list-title = Onglets ouverts
tab-list-hint = Tapez une partie du nom d'un onglet
no-results = Aucun résultat
selected-count = { $count } sélectionnés
marked-count = { $count } marqués
dialog-open-workspace = Ouvrir un dossier comme espace de travail
distraction-free-notice = Mode sans distraction. Appuyez sur F11 pour en sortir.
wrapped-to-top = Retour au début
wrapped-to-bottom = Retour à la fin
not-found = Introuvable
no-more-matches = Plus d'occurrences
replaced-count = { $count } remplacés
search-start-failed = Impossible de lancer la recherche : { $error }
find-in-files-title = « { $text } » dans { $folder }
invalid-regex = Expression régulière non valide : { $error }
about-ettin-edit = À propos d'Ettin Edit
menu-file = Fichier
menu-edit = Édition
menu-search = Rechercher
menu-view = Affichage
menu-encoding = Encodage
menu-theme = Thème
menu-language = Langage
menu-settings = Paramètres
menu-tools = Outils
menu-plugins = Modules
menu-macro = Macro
menu-help = Aide
open-recent = Fichiers récents
no-recent-files = Aucun fichier récent
new = Nouveau
open-dots = Ouvrir…
clear-recent-files = Effacer les fichiers récents
save = Enregistrer
save-as = Enregistrer sous…
save-all = Tout enregistrer
reload-from-disk = Recharger depuis le disque
rename-dots = Renommer…
move-to-trash = Mettre à la corbeille…
open-containing-folder = Ouvrir le dossier parent
open-terminal-here = Ouvrir un terminal ici
print-dots = Imprimer…
export-html = Exporter en HTML…
open-folder-as-workspace = Ouvrir un dossier comme espace de travail…
close-workspace = Fermer l'espace de travail
close = Fermer
close-all = Tout fermer
close-others = Fermer les autres
exit = Quitter
find-dots = Rechercher…
replace-dots = Remplacer…
find-next = Suivant
find-previous = Précédent
find-selected-next = Rechercher la sélection (suivant)
find-selected-previous = Rechercher la sélection (précédent)
find-in-files = Rechercher dans les fichiers…
find-all-in-document = Tout rechercher dans le document courant
find-all-in-tabs = Tout rechercher dans les onglets ouverts
select-all-matches = Sélectionner toutes les occurrences
mark-all = Tout marquer
clear-all-marks = Effacer toutes les marques
go-to-line = Aller à la ligne…
menu-bookmark = Signet
toggle-bookmark = Basculer le signet
next-bookmark = Signet suivant
previous-bookmark = Signet précédent
clear-all-bookmarks = Supprimer tous les signets
cut-bookmarked-lines = Couper les lignes avec signet
copy-bookmarked-lines = Copier les lignes avec signet
remove-bookmarked-lines = Supprimer les lignes avec signet
remove-unbookmarked-lines = Supprimer les lignes sans signet
inverse-bookmarks = Inverser les signets
word-wrap = Retour à la ligne automatique
line-numbers = Numéros de ligne
highlight-current-line = Surligner la ligne courante
menu-show-symbol = Afficher les symboles
show-whitespace = Afficher les espaces
show-end-of-line = Afficher les fins de ligne
show-invisible = Afficher les caractères invisibles
show-all-characters = Afficher tous les caractères
indent-guides = Guides d'indentation
long-line-marker-column = Repère de ligne longue (colonne { $column })
function-list = Liste des fonctions
document-map = Carte du document
character-panel = Panneau des caractères
hex-view = Vue hexadécimale
folder-as-workspace = Dossier comme espace de travail
distraction-free-mode = Mode sans distraction
tab-list = Liste des onglets…
command-palette = Palette de commandes…
move-to-other-view = Déplacer vers l'autre vue
clone-to-other-view = Cloner dans l'autre vue
focus-other-view = Activer l'autre vue
menu-folding = Repliage
fold-all = Tout replier
unfold-all = Tout déplier
fold-current-level = Replier le niveau courant
unfold-current-level = Déplier le niveau courant
fold-level = Replier le niveau { $level }
zoom-in = Zoom avant
zoom-out = Zoom arrière
reset-zoom = Réinitialiser le zoom
summary-dots = Résumé…
themes-dark = Sombre
themes-light = Clair
themes-custom = Personnalisé
edit-theme-colours = Modifier les couleurs du thème…
open-themes-folder = Ouvrir le dossier des thèmes
reload-themes = Recharger les thèmes
restore-built-in-themes = Restaurer les thèmes intégrés
preferences-dots = Préférences…
shortcut-mapper-dots = Raccourcis clavier…
auto-close-brackets = Fermer automatiquement parenthèses et guillemets
smart-indentation = Indentation intelligente
indent-with-spaces = Indenter avec des espaces
word-completion = Complétion des mots
keyword-completion = Complétion des mots-clés et fonctions
parameter-hints = Aide aux paramètres
single-window-long = Fenêtre unique (ouvrir dans l'Ettin Edit déjà lancé)
remember-unsaved = Mémoriser les modifications non enregistrées en quittant
auto-reload = Recharger automatiquement les fichiers modifiés sur le disque
long-line-column = Colonne de ligne longue
user-manual = Manuel d'utilisation
user-manual-web = Manuel d'utilisation en ligne
check-for-updates = Rechercher des mises à jour…
check-updates-automatically = Rechercher automatiquement les mises à jour
encoding-save-as = Enregistrer en
reopen-with = Rouvrir avec
only-saved-files = Uniquement pour les fichiers enregistrés sur le disque
stop-recording = Arrêter l'enregistrement
start-recording = Démarrer l'enregistrement
playback = Exécuter
run-macro-multiple = Exécuter une macro plusieurs fois…
save-current-macro = Enregistrer la macro courante…
delete-saved-macro = Supprimer une macro enregistrée
new-language = Nouveau langage…
open-languages-folder = Ouvrir le dossier des langages
open-completion-lists-folder = Ouvrir le dossier des listes de complétion
reload-languages = Recharger les langages
language-guide = Guide des langages
all-open-tabs = Tous les onglets ouverts
new-tab = Nouvel onglet
unpin-tab = Désépingler l'onglet
pin-tab = Épingler l'onglet
copy-full-path = Copier le chemin complet
copy-file-name = Copier le nom du fichier
status-length-lines = longueur : { $length }   lignes : { $lines }
click-to-copy = Cliquer pour copier
recording-macro = Enregistrement de macro
read-only-status = Lecture seule
column-mode-status = Mode colonne
column-mode-status-hover = Glisser et Maj+flèches sélectionnent un rectangle. Cliquer pour désactiver.
click-to-allow-editing = Cliquer pour autoriser la modification
status-position = Ln { $line }, Col { $column }
status-selection = Sél { $count }
status-carets = { $count } curseurs
go-to-line-hover = Aller à la ligne (Ctrl+G)
save-changes-title = Enregistrer les modifications ?
save-changes-question = « { $name } » contient des modifications non enregistrées. Les enregistrer avant de fermer ?
cancel = Annuler
dont-save = Ne pas enregistrer
trash-failed = Impossible de mettre { $name } à la corbeille : { $error }
about-version = Version { $version }
about-tagline = Un éditeur de texte rapide et moderne pour Linux et Windows.
about-built-with = Écrit en Rust avec egui et syntect.
error-title = Une erreur s'est produite
ok = OK

## Documents (src/document.rs)
eol-unix = Unix (LF)
eol-windows = Windows (CR LF)
eol-mac = Macintosh (CR)
cant-encode-char = « { $char } » (U+{ $code }) ne peut pas être enregistré en { $encoding }. Choisissez un encodage qui le contient, comme UTF-8, dans le menu Encodage.
untitled-name = nouveau { $number }

## Menus Édition et Outils (src/app/menus.rs)
duplicate = Dupliquer
delete-lines = Supprimer les lignes
move-lines-up = Monter
move-lines-down = Descendre
join-lines = Joindre les lignes
split-lines = Couper les lignes à la colonne de ligne longue
insert-blank-above = Insérer une ligne vide au-dessus
insert-blank-below = Insérer une ligne vide en dessous
sort-lexical-asc = Tri lexicographique croissant
sort-lexical-desc = Tri lexicographique décroissant
sort-ignore-case-asc = Tri croissant sans tenir compte de la casse
sort-ignore-case-desc = Tri décroissant sans tenir compte de la casse
sort-integer-asc = Tri croissant des entiers
sort-integer-desc = Tri décroissant des entiers
sort-decimal-asc = Tri croissant des décimaux
sort-decimal-desc = Tri décroissant des décimaux
sort-length-asc = Tri croissant par longueur
sort-length-desc = Tri décroissant par longueur
reverse-lines = Inverser l'ordre des lignes
remove-duplicate-lines = Supprimer les lignes en double
remove-consecutive-duplicates = Supprimer les doublons consécutifs
remove-empty-lines = Supprimer les lignes vides
remove-blank-lines = Supprimer les lignes vides (y compris blanches)
undo = Annuler
redo = Rétablir
cut = Couper
copy = Copier
paste = Coller
select-all = Tout sélectionner
menu-line-operations = Opérations sur les lignes
menu-convert-case = Convertir la casse en
case-upper = MAJUSCULES
case-lower = minuscules
case-title = Majuscule À Chaque Mot
case-sentence = Majuscule en début de phrase
case-invert = iNVERSER lA cASSE
menu-comment = Commentaire
toggle-line-comment = Basculer le commentaire de ligne
toggle-block-comment = Basculer le commentaire de bloc
menu-blank-operations = Opérations sur les blancs
trim-trailing = Supprimer les espaces en fin de ligne
trim-leading = Supprimer les espaces en début de ligne
trim-both = Supprimer les espaces en début et fin de ligne
eol-to-space = Fins de ligne en espaces
tabs-to-spaces = Tabulations en espaces
leading-spaces-to-tabs = Espaces de début en tabulations
all-spaces-to-tabs = Tous les espaces en tabulations
menu-insert = Insérer
date-time-short = Date et heure (court)
date-time-long = Date et heure (long)
menu-multi-select = Multi-sélection
select-next-occurrence = Sélectionner l'occurrence suivante
select-all-occurrences = Sélectionner toutes les occurrences
menu-auto-completion = Auto-complétion
complete-word = Compléter le mot
show-parameter-hint = Afficher l'aide aux paramètres
column-editor = Éditeur de colonnes…
read-only = Lecture seule
column-mode = Mode colonne
mime-base64-encode = Encoder en Base64
mime-base64-encode-wrapped = Encoder en Base64 avec retours à la ligne
mime-base64-url-encode = Encoder en Base64, compatible URL
mime-base64-decode = Décoder le Base64
mime-url-encode-component = Encoder pour URL (partie d'URL)
mime-url-encode = Encoder pour URL (URL entière)
mime-url-decode = Décoder l'URL
mime-qp-encode = Encoder en Quoted-Printable
mime-qp-decode = Décoder le Quoted-Printable
mime-html-encode = Encoder en HTML
mime-html-encode-all = Encoder en HTML, accents et symboles compris
mime-html-decode = Décoder le HTML
mime-saml-decode = Décoder le SAML
json-format = Formater
json-minify = Minifier
json-validate = Valider le JSON
json-sort-keys = Trier les clés JSON
json-tree = Arbre JSON
xml-pretty-print = Mettre en forme
xml-pretty-print-attributes = Mettre en forme, attributs sur des lignes séparées
xml-linearize = Linéariser
xml-check-syntax = Vérifier la syntaxe
xml-check-on-save = Vérifier la syntaxe à l'enregistrement
xml-validate = Valider
xml-validate-against = Valider avec…
xml-xsl-transform = Transformation XSL…
xpath-panel = Panneau XPath
copy-current-xpath = Copier le XPath courant
xml-escape = Échapper les caractères spéciaux
xml-unescape = Déséchapper les caractères spéciaux
close-tags-automatically = Fermer automatiquement les balises
csvquery-panel = Panneau CsvQuery
run-query = Exécuter la requête
copy-results = Copier les résultats
results-to-new-tab = Résultats dans un nouvel onglet
menu-compare = Comparer
menu-csvquery = CsvQuery
menu-mime-tools = Outils MIME
menu-json = JSON et JavaScript
menu-xml-tools = Outils XML
compare-two-views = Comparer les deux vues
compare-previous-tab = Comparer avec l'onglet précédent
compare-with = Comparer avec
compare-to-saved = Comparer au dernier enregistrement
compare-to-clipboard = Comparer au presse-papiers
compare-to-git = Comparer au dernier commit (Git)
compare-selections = Comparer les sélections
next-difference = Différence suivante
previous-difference = Différence précédente
show-comparison = Afficher la comparaison
export-as-patch = Exporter en patch
close-comparison = Fermer la comparaison
compare-ignore-whitespace = Ignorer les espaces et tabulations
compare-ignore-case = Ignorer la casse
compare-ignore-empty-lines = Ignorer les lignes vides
compare-detect-moves = Détecter les lignes déplacées

## Noms des commandes dans la palette de commandes (src/app/commands.rs)
long-line-marker = Repère de ligne longue
next-tab = Onglet suivant
previous-tab = Onglet précédent
single-window = Fenêtre unique
start-or-stop-recording = Démarrer ou arrêter l'enregistrement
manage-plugins = Gérer les modules…
plugin-log = Journal des modules
open-plugins-folder = Ouvrir le dossier des modules
reload-plugins = Recharger les modules
install-sample-plugins = Installer les modules d'exemple
plugin-guide = Guide des modules

## Menu Modules et notifications des modules (src/app/plugins.rs)
install-plugin-from-file = Installer un module depuis un fichier
plugin-zip-filter = Module (.zip)
install-plugin-from-folder = Installer un module depuis un dossier
no-plugins-folder = Aucun dossier de configuration où placer les modules.
plugin-removed = { $name } supprimé ; son dossier est dans la corbeille.
plugin-updated = { $name } mis à jour.
plugin-updated-version = { $name } mis à jour en { $version }.
plugin-installed = { $name } installé.
plugin-installed-version = { $name } { $version } installé.
plugins-running = { $count ->
    [one] { $count } module actif
   *[other] { $count } modules actifs
}
sample-install-failed = Impossible d'installer le module d'exemple { $name } dans { $folder } : { $error }
samples-already-installed = Les modules d'exemple sont déjà dans le dossier des modules.
samples-installed = { $names } installés. Retrouvez-les dans le menu Modules.
no-plugin-commands = Aucune commande de module

## Outils de texte (src/app/text_tools.rs)
tool-error-at = { $tool } : ligne { $line }, colonne { $column } : { $message }
tool-error = { $tool } : { $message }
tool-read-only = le document est en lecture seule.
tool-ok-selection = { $result } (sélection)
json-handled-as-js = Ce n'est pas du JSON strict, il a donc été traité comme du JavaScript. Valider le JSON montre les différences.
json-valid = JSON valide
xml-well-formed = Le XML est bien formé
copied-text = { $text } copié
xpath-no-element = le curseur n'est pas dans un élément, ou une balise de fin ne correspond pas à sa balise de début.
xpath-not-xml = ce document n'est pas du XML.
copy-xpath = Copier le XPath
saved-not-well-formed = Enregistré, mais { $name } n'est pas bien formé : ligne { $line }, colonne { $column } : { $message }
xpath-type-expression = Tapez une expression XPath, comme //item[@id='1'].
xpath-not-well-formed = Le document n'est pas du XML bien formé (ligne { $line }, colonne { $column } : { $message })
xpath-error-at = { $message } (au caractère { $position })

## Comparaison (src/app/compare.rs)
compare-selection-title = { $name } (sélection à la ligne { $line })
compare-same = Les deux côtés sont identiques.
compare-need-second-tab = Ouvrez un second onglet à comparer, ou choisissez-en un dans Outils > Comparer > Comparer avec.
compare-not-saved = Cet onglet n'a pas encore été enregistré.
compare-saved-title = { $name } (enregistré)
compare-read-saved-failed = Impossible de lire le fichier enregistré : { $error }
compare-commit-title = { $name } (dernier commit)
compare-need-selections = Sélectionnez deux morceaux de texte : Ctrl+glisser pour une seconde sélection, ou sélectionnez dans chaque vue.
clipboard = Presse-papiers
compare-clipboard-empty = Le presse-papiers ne contient pas de texte à comparer.
compare-closed-title = { $name } (fermé)
compare-tab-title = Comparaison : { $left } | { $right }

## Dossier des langages (src/app/languages.rs)
make-folder-failed = Impossible de créer { $path } : { $error }
write-failed = Impossible d'écrire { $path } : { $error }

## Impression (src/app/printing.rs)
dialog-export-html = Exporter en HTML

## Validation XML et XSL (src/app/xml_schema.rs)
dialog-validate-against = Valider avec un schéma
filter-schema = Schéma XML ou DTD
filter-all-files = Tous les fichiers
the-doctype = le DOCTYPE
read-failed = Impossible de lire { $path } : { $error }
validate-against = Valider avec { $schema }
valid-against = Le document est valide selon { $schema }
xsl-transform = Transformation XSL
filter-xsl = Feuille de style XSL

## Préférences (src/prefs_ui.rs)
prefs-editing = Édition
prefs-display = Affichage
prefs-new-documents = Nouveaux documents
prefs-general = Fichiers et démarrage
preferences = Préférences
theme-colours-dots = Couleurs du thème…
prefs-font = Police
built-in-font = Intégrée (Hack)
editor-font-label = Police de l'éditeur :
use-built-in = Utiliser l'intégrée
font-size-label = Taille :
font-size-hint = Ctrl+0 revient à cette taille après un zoom
font-find-label = Rechercher :
font-name-hint = Nom de la police
show-proportional-fonts = Afficher les polices proportionnelles
looking-for-fonts = Recherche des polices installées…
no-fonts-match = Aucune police ne correspond
proportional = proportionnelle
prefs-show = Afficher
prefs-word-wrap = Retour à la ligne automatique
prefs-line-numbers = Numéros de ligne
prefs-highlight-line = Surligner la ligne courante
prefs-whitespace = Symboles des espaces
prefs-eol = Symboles de fin de ligne
prefs-invisible = Caractères invisibles (caractères de contrôle, espaces inhabituelles, caractères sans chasse)
prefs-indent-guides = Guides d'indentation
prefs-edge = Repère de ligne longue à la colonne
prefs-indentation = Indentation
prefs-tab-width = Largeur de tabulation :
prefs-columns = colonnes
prefs-indent-tabs = Indenter avec des tabulations
prefs-indent-spaces = Indenter avec des espaces
prefs-smart-indent = Indentation intelligente (conserver le retrait, et indenter après une parenthèse ouvrante)
prefs-typing = Saisie
prefs-auto-close = Fermer automatiquement parenthèses et guillemets
prefs-close-tags = Fermer automatiquement les balises HTML et XML
prefs-word-completion = Suggérer les mots du document pendant la saisie
prefs-language-completion = Suggérer les mots-clés et fonctions du langage
prefs-parameter-hints = Afficher les paramètres d'une fonction pendant la saisie d'un appel
prefs-line-ending = Fin de ligne
prefs-system-line-ending = Valeur par défaut du système ({ $ending })
prefs-startup = Démarrage et sortie
prefs-restore-session = Rouvrir les fichiers ouverts la dernière fois
prefs-remember-unsaved = Quitter sans demander pour les modifications non enregistrées, et les restaurer la fois suivante
prefs-single-instance = Ouvrir les fichiers dans la fenêtre d'Ettin Edit déjà lancée
prefs-files = Fichiers
prefs-auto-reload = Recharger automatiquement les fichiers modifiés sur le disque s'ils n'ont pas de modifications non enregistrées
prefs-xml-check = Vérifier que le XML est bien formé à chaque enregistrement
prefs-printing = Impression
prefs-print-line-numbers = Imprimer les numéros de ligne
prefs-print-colours = Imprimer les couleurs syntaxiques (assombries pour rester lisibles sur papier)
prefs-updates = Mises à jour
prefs-check-updates = Rechercher une nouvelle version d'Ettin Edit une fois par semaine
prefs-plugin-programs = Programmes qui exécutent les modules
prefs-plugin-path-hint = Laisser vide pour les chercher dans le PATH.
prefs-plugin-reload-hint = Les changements s'appliquent au rechargement des modules.
prefs-interface-language = Langue de l'interface
prefs-system-language = Comme le système ({ $language })
open-translations-folder = Ouvrir le dossier des traductions
reload-translations = Recharger
prefs-translations-hint = Ajoutez une langue ou modifiez une traduction en plaçant un fichier .ftl dans le dossier des traductions. Voir Traductions dans le manuel d'utilisation.

## Traductions (src/i18n.rs)
no-config-folder = Aucun dossier de configuration où conserver les traductions.

## Barre de recherche, résultats, Aller à la ligne, Rechercher dans les fichiers (src/search_ui.rs)
toggle-replace = Afficher/masquer le remplacement
find-hint = Rechercher
match-case = Respecter la casse
whole-word = Mot entier
regular-expression = Expression régulière
extended-search = Étendu : \n, \r, \t, \0, \xNN et \uNNNN désignent ces caractères
extended-search-short = Étendu (\n, \t, \xNN)
match-of-total = { $current } sur { $total }
matches-found = { $total } trouvés
find-status = { $count }, { $message }
previous-match = Occurrence précédente (Shift+F3)
next-match = Occurrence suivante (F3)
wrap-around = Boucler
mark-all-bookmark = Tout marquer et poser des signets
close-esc = Fermer (Échap)
replace-hint-regex = Remplacer  ($1 ou \1 pour les groupes)
replace-hint-extended = Remplacer  (\n, \t, \xNN acceptés)
replace = Remplacer
replace-hover = Remplacer cette occurrence et chercher la suivante
replace-all = Tout remplacer
in-selection = Dans la sélection
search-stopped = arrêté
searched-files = { $count ->
    [one] { $count } fichier parcouru
   *[other] { $count } fichiers parcourus
}
results-hits = { $count ->
    [one] { $count }{ $plus } résultat
   *[other] { $count }{ $plus } résultats
}
results-in-files = { $count ->
    [one] { $found } dans { $count } fichier
   *[other] { $found } dans { $count } fichiers
}
results-in-documents = { $count ->
    [one] { $found } dans { $count } document
   *[other] { $found } dans { $count } documents
}
stop = Arrêter
close-results = Fermer les résultats
collapse-all = Tout réduire
expand-all = Tout développer
go-to-line-title = Aller à la ligne
go-to-line-prompt = Numéro de ligne (1–{ $lines }), ou ligne:colonne
enter-line-number = Saisissez un numéro de ligne
go = Aller
find-in-files-heading = Rechercher dans les fichiers
find-what = Rechercher
folder = Dossier
browse = Parcourir…
search-in-folder = Rechercher dans le dossier
file-filter-hint = *.rs *.toml   (vide pour tous les fichiers, !nom pour exclure)
file-filter-hover = Motifs de noms de fichiers séparés par des espaces ou des points-virgules. Commencez-en un par ! pour exclure ces fichiers.
include-subfolders = Inclure les sous-dossiers
include-hidden = Inclure les fichiers et dossiers cachés
find-all = Tout rechercher
enter-search-text = Saisissez un texte à rechercher.
folder-missing = Ce dossier n'existe pas.
results-problems = { $count ->
    [one] { $count }{ $plus } problème
   *[other] { $count }{ $plus } problèmes
}

## Renommer, Mettre à la corbeille, Résumé, fichier modifié sur le disque (src/file_ui.rs)
rename-file-title = Renommer le fichier
new-name = Nouveau nom
rename = Renommer
move-to-trash-title = Mettre à la corbeille
move-to-trash-question = { $windows ->
    [yes] Mettre « { $name } » dans la Corbeille et fermer son onglet ? Vous pourrez le restaurer depuis la Corbeille.
   *[no] Mettre « { $name } » à la corbeille et fermer son onglet ? Vous pourrez le restaurer depuis la corbeille.
}
summary-title = Résumé
summary-file = Fichier
summary-created = Créé
summary-modified = Modifié
summary-characters = Caractères (sans les fins de ligne)
summary-words = Mots
summary-lines = Lignes
summary-size = Taille une fois enregistré
summary-bytes = { $count ->
    [one] { $bytes } octet
   *[other] { $bytes } octets
}
summary-cant-encode = ne peut pas être enregistré dans cet encodage
summary-selection = Sélection
summary-selection-size = { $line_count ->
    [one] { $characters } caractères sur { $lines } ligne
   *[other] { $characters } caractères sur { $lines } lignes
}
disk-changed = Ce fichier a été modifié par un autre programme.
disk-changed-unsaved = Ce fichier a été modifié par un autre programme, et vous avez ici des modifications non enregistrées. Recharger les remplace (Annuler les restaure).
disk-deleted = Ce fichier a été supprimé ou déplacé par un autre programme.
reload = Recharger
keep-my-version = Garder ma version
keep-open = Garder ouvert

## Panneau des dossiers (src/folder_ui.rs)
no-folder-open = Aucun dossier ouvert.
close-folder-panel = Fermer le panneau des dossiers
refresh-folder = Relire le dossier
collapse-folders = Réduire tous les dossiers
show-hidden-files = Afficher les fichiers cachés
open = Ouvrir
find-in-files-here = Rechercher dans les fichiers ici…
copy-path = Copier le chemin
folder-truncated = Seules les { $count } premières entrées sont affichées

## Liste des fonctions (src/function_list_ui.rs)
show-caret-symbol = Montrer où est le curseur
filter-hint = Filtrer
reading-document = Lecture du document…
nothing-found-in-document = Rien trouvé dans ce document.
line-number = Ligne { $line }

## Palette de commandes (src/palette_ui.rs)
nothing-matches = Aucune correspondance

## Raccourcis clavier (src/shortcut_ui.rs)
shortcut-reserved-clipboard = Ctrl+C, Ctrl+X et Ctrl+V sont réservés à copier, couper et coller
shortcut-mapper = Raccourcis clavier
command-or-shortcut = Commande ou raccourci
only-assigned = Seulement les commandes avec un raccourci
shortcut-in-use = { $shortcut } est déjà utilisé par « { $command } ».
move-it-here = Le déplacer ici
select-command = Sélectionnez une commande pour changer son raccourci.
shortcut-fixed = « { $command } » appartient à la zone de texte et ne peut pas être modifié.
press-shortcut = Appuyez sur le nouveau raccourci… (Échap pour annuler)
change-shortcut = Modifier…
set-shortcut = Définir…
add-another-shortcut = Ajouter un autre…
remove = Supprimer
reset-to-default = Rétablir la valeur par défaut
reset-all-shortcuts = Réinitialiser tous les raccourcis
shortcut-rule = Les raccourcis nécessitent Ctrl ou Alt, sauf avec une touche de fonction (F1–F12).
command = Commande
shortcut = Raccourci

## Couleurs du thème (src/style_ui.rs)
style-background = Arrière-plan
style-text = Texte
style-caret = Curseur
style-selection = Sélection
style-current-line = Ligne courante
style-line-numbers = Numéros de ligne
style-indent-guides = Guides d'indentation
style-whitespace = Symboles des espaces
style-search-matches = Résultats de recherche
style-accent = Accentuation (onglets, boutons, repliage)
unset-colour = Ne pas définir cette couleur
set-colour = Définir
choose-colour = Choisir une couleur
theme-read-failed = Impossible de lire le fichier de thème : { $error }
theme-colours = Couleurs du thème
theme-label = Thème :
discard-changes = Abandonner les modifications
save-as-new-theme = Enregistrer comme nouveau thème…
new-theme-name = Nom du nouveau thème :
new-theme-name-hint = Nom du nouveau thème
theme-name-taken = Un thème porte déjà ce nom
theme-colours-hint = Les changements s'affichent immédiatement. Enregistrer les conserve ; fermer cette fenêtre sans enregistrer les abandonne.
style-editor = Éditeur
style-syntax = Syntaxe
rule-filter-hint = Nom ou portée de règle, comme string ou keyword
rules-hint = Chaque règle colore le texte dont la portée correspond. Séparez plusieurs portées par des virgules.
rule-name = Nom
rule-scope = Portée
rule-style = Style
no-name = (sans nom)
delete-rule = Supprimer cette règle
add-rule = Ajouter une règle
new-rule = Nouvelle règle

## Notifications, arbre JSON, panneau XPath (src/text_tools_ui.rs)
go-to = Aller à
dismiss = Ignorer
json-too-large = Ce document est trop volumineux pour être affiché en arbre.
close-json-tree = Fermer l'arbre JSON
json-filter-hint = Filtrer les clés et valeurs
json-not-valid = JSON non valide
line-column-message = Ligne { $line }, colonne { $column } : { $message }
go-to-error = Aller à l'erreur
no-matches = Aucune correspondance
first-matches = { $count } premières correspondances
copy-key = Copier la clé
copy-value = Copier la valeur
expand-all-inside = Tout développer à l'intérieur
close-xpath-panel = Fermer le panneau XPath
evaluate = Évaluer
recent = Récentes
recent-expressions = Expressions récentes
result-label = Résultat :
xpath-nodes = { $count ->
    [one] { $count } nœud
   *[other] { $count } nœuds
}
xpath-nodes-stale = { $count ->
    [one] { $count } nœud (le document a changé depuis ; évaluez à nouveau pour actualiser)
   *[other] { $count } nœuds (le document a changé depuis ; évaluez à nouveau pour actualiser)
}

## Éditeur de colonnes et macros (src/tools_ui.rs)
column-editor-title = Éditeur de colonnes
column-editor-hint = Insère à chaque curseur, ou à la colonne du curseur sur chaque ligne en dessous.
hex-offset = Position 0x{ $hex } ({ $dec })
hex-selected = { $count } octets sélectionnés
hex-size = { $count } octets au total
hex-read-only = Cet onglet est en lecture seule.
hex-cant-encode = « { $char } » ne peut pas être écrit en { $encoding }.
hex-paste-hint = Collez des chiffres hexadécimaux, comme 48 65 6C.
hex-invalid = Pas modifié : ces octets ne sont pas du texte { $encoding } valide.
hex-round-trip = Pas modifié : le fichier serait enregistré autrement (vérifiez les fins de ligne).
char-search-hint = Rechercher : é, U+00E9, 233 ou un nom
char-panel-hint = Double-cliquez sur un caractère pour l'insérer. Clic droit pour copier.
char-insert-character = Insérer le caractère
char-insert-html = Insérer l'entité HTML
char-insert-hex = Insérer le code (U+…)
char-insert-decimal = Insérer le code décimal
char-col-char = Car.
char-col-dec = Déc
char-col-hex = Hex
char-col-html = HTML
char-col-name = Nom
char-copy-character = Copier le caractère
char-copy-html = Copier l'entité HTML
char-copy-code = Copier le code (U+…)
text-to-insert = Texte à insérer
numbers-to-insert = Nombres à insérer
initial-number = Nombre initial
increase-by = Incrément
repeat = Répéter
leading-zeros = Zéros non significatifs
format-dec = Déc
format-hex = Hex
format-oct = Oct
format-bin = Bin
insert = Insérer
must-be-whole-number = { $field } doit être un nombre entier
run-macro-title = Exécuter une macro plusieurs fois
run = Exécuter
times = fois
run-to-end = Exécuter jusqu'à la fin du fichier
record-macro-first = Enregistrez d'abord une macro.
save-macro-title = Enregistrer la macro courante

## Vue de comparaison (src/compare_ui.rs)
no-differences = Aucune différence
lines-changed = { $count } modifiées
lines-added = { $count } ajoutées
lines-removed = { $count } supprimées
lines-moved = { $count } déplacées
compare-lines-summary = Lignes : { $parts }
first = Première
first-difference = Première différence
previous-difference-hover = Différence précédente (Shift+F7)
next-difference-hover = Différence suivante (F7)
last = Dernière
last-difference = Dernière différence
identical = Identiques
difference-of = Différence { $current } sur { $total }
differences-count = { $count ->
    [one] { $count } différence
   *[other] { $count } différences
}
compare-approximate = (approximatif : les fichiers sont très différents)
close-comparison-hover = Fermer la comparaison
export-patch = Exporter le patch
export-patch-hover = Ouvrir les différences au format diff unifié dans un nouvel onglet
swap-sides = Inverser les côtés
options = Options
line-count = { $count ->
    [one] { $count } ligne
   *[other] { $count } lignes
}

## Types de colonnes et délimiteurs CSV (src/tools/csv.rs)
column-type-text = Texte
column-type-integer = Entier
column-type-decimal = Décimal
delimiter-comma = Virgule
delimiter-semicolon = Point-virgule
delimiter-tab = Tab
delimiter-pipe = Barre verticale
delimiter-space = Espace

## Panneau CsvQuery (src/csv_query.rs)
query-engine-stopped = Le moteur de requêtes s'est arrêté. Réessayez.
csv-shape = { $rows } lignes × { $columns } colonnes
csv-this-hover = L'onglet courant est la table `this` ; les autres onglets sont des tables nommées d'après leur fichier.
csv-open-file = Ouvrez un fichier CSV ou TSV pour l'interroger.
close-csvquery = Fermer CsvQuery
tables = Tables
click-to-insert-name = Cliquer pour insérer un nom
no-queries-yet = Aucune requête pour l'instant
run-query-hover = Exécuter la requête (Ctrl+Enter)
results = Résultats
columns = Colonnes
csv-tables-list = Tables : this, { $names }
csv-rows-changed = { $count ->
    [one] { $count } ligne modifiée dans la copie de requête (le fichier n'est pas modifié)
   *[other] { $count } lignes modifiées dans la copie de requête (le fichier n'est pas modifié)
}
csv-first-rows = { $rows } premières lignes · { $ms } ms
csv-rows = { $count ->
    [one] { $rows } ligne · { $ms } ms
   *[other] { $rows } lignes · { $ms } ms
}
open-in-new-tab = Ouvrir dans un nouvel onglet
open-in-new-tab-hover = Écrire toutes les lignes de résultat dans un nouvel onglet en CSV
copy-results-hover = Copier les résultats, séparés par des tabulations pour un tableur
click-to-sort = Cliquer pour trier
copy-row = Copier la ligne
csv-run-for-columns = Exécutez une requête pour voir les colonnes de cet onglet.
csv-column = Colonne
csv-type = Type
csv-filled = Remplies
csv-empty = Vides
csv-distinct = Distinctes
csv-min = Min
csv-max = Max
csv-average = Moyenne
csv-type-hover = Comment la colonne est stockée et comparée. Auto choisit le type le plus restreint qui convient à toutes les valeurs.
auto-with = Auto ({ $detected })
delimiter = Délimiteur
auto-quotes = Guillemets auto
quotes-char = Guillemets { $char }
no-quotes = Sans guillemets
quote-character = Caractère de guillemet
header-row = Ligne d'en-tête
header-row-hover = La première ligne nomme les colonnes
reset = Réinitialiser
reset-format-hover = Revenir au format détecté

## Recherche de mises à jour (src/update.rs)
update-bad-json = La liste des versions est incompréhensible : { $error }
update-no-version = La liste des versions ne contient pas de version
update-not-found = Aucune version publiée trouvée (HTTP 404). La recherche de mises à jour ne fonctionne que si le dépôt d'Ettin Edit est public.
update-http-error = GitHub a répondu HTTP { $code }.
update-unreachable = Impossible de joindre GitHub : { $error }
update-read-failed = Impossible de lire la réponse de GitHub : { $error }
check-for-updates-title = Rechercher des mises à jour
update-asking = Demande de la dernière version à GitHub…
update-available = Ettin Edit { $version } est disponible.
update-current = Vous avez la version { $version }.
open-download-page = Ouvrir la page de téléchargement
dont-check-automatically = Ne pas vérifier automatiquement
update-latest = Vous avez la dernière version, { $version }.
update-newer-local = Vous avez la version { $version } ; la plus récente publiée est { $latest }.
update-failed = Impossible de rechercher des mises à jour.

## Fenêtre du manuel d'utilisation (src/manual.rs)
back = Précédent
forward = Suivant
search-manual = Rechercher dans le manuel
open-on-web = Ouvrir sur le Web
web-manual-failed = Impossible d'ouvrir le manuel en ligne : { $error }
open-url-failed = Impossible d'ouvrir { $url } : { $error }
copy-to-clipboard = Copier ceci dans le presse-papiers

## Groupes d'encodages (src/encoding.rs)
enc-western-european = Europe occidentale
enc-central-european = Europe centrale
enc-cyrillic = Cyrillique
enc-greek = Grec
enc-turkish = Turc
enc-baltic = Balte
enc-nordic = Nordique
enc-hebrew = Hébreu
enc-arabic = Arabe
enc-vietnamese = Vietnamien
enc-thai = Thaï
enc-romanian = Roumain
enc-chinese = Chinois
enc-japanese = Japonais
enc-korean = Coréen

## Fichiers, dossiers, terminal et Git (src/system.rs)
explorer-failed = Impossible d'ouvrir l'Explorateur : { $error }
no-parent-folder = { $path } n'a pas de dossier
terminal-failed = Impossible d'ouvrir un terminal : { $error }
no-terminal = Aucun terminal trouvé. Définissez la variable d'environnement TERMINAL avec la commande de votre terminal.
git-missing = Git n'est pas installé (ou n'est pas dans le PATH).
git-failed = Impossible d'exécuter git : { $error }
not-in-git = { $path } n'est pas dans un dépôt Git.
not-in-repository = { $path } n'est pas dans le dépôt situé à { $root }.
git-not-committed = Ce fichier n'est pas encore dans le dernier commit.
git-no-commits = Le dépôt n'a encore aucun commit.

## Thèmes (src/theme.rs)
themes-write-failed = Impossible d'écrire les thèmes intégrés dans { $path } : { $error }
theme-file-problem = Le fichier de thème { $path } a un problème, le thème { $name } d'origine est donc utilisé : { $error }
theme-skipped = Thème { $path } ignoré : { $error }

## Polices (src/fonts.rs)
font-unusable = { $path } n'est pas une police utilisable par Ettin Edit.
font-read-failed = Impossible de lire la police de l'éditeur { $path } : { $error }

## Éditeur (src/editor/view.rs)
completion-keyword = mot-clé
completion-function = fonction

## Gérer les modules et journal des modules (src/plugins/ui.rs)
plugins-warning = Les modules s'exécutent comme des programmes séparés, avec votre accès aux fichiers et au réseau. N'exécutez que des modules de confiance.
plugins-installed-tab = Installés ({ $count })
plugins-available-count = Disponibles ({ $count })
plugins-available = Disponibles
install-from-folder = Installer depuis un dossier…
install-from-folder-hover = Copier le dossier d'un module (contenant un plugin.json) dans le dossier des modules
install-from-file = Installer depuis un fichier…
install-from-file-hover = Installer un module depuis un fichier .zip
remove-plugin-title = Supprimer « { $name } » ?
remove-plugin-hint = Son dossier va dans la corbeille, d'où vous pourrez le récupérer.
no-plugins-yet = Aucun module pour l'instant.
no-plugins-hint = Installez les exemples pour voir comment fonctionnent les modules, trouvez des modules sous Disponibles, ou placez le dossier d'un module dans le dossier des modules.
found-on-path = { $program } (trouvé dans le PATH)
reload-plugins-hover = Arrêter tous les modules, les rechercher à nouveau et les démarrer
plugin-filter-hint = Nom, auteur ou description
refresh = Actualiser
refresh-index-hover = Relire l'index des modules
reading-plugin-index = Lecture de l'index des modules…
try-again = Réessayer
index-empty = L'index ne liste aucun module.
no-plugins-match = Aucun module ne correspond au filtre.
plugin-index = Index
plugin-index-hover = L'adresse de la liste JSON des modules. Appuyez sur Enter pour la lire.
default = Par défaut
default-index-hover = Utiliser l'index de modules d'Ettin Edit
needs-version = Nécessite Ettin Edit { $version }
update-to = Mettre à jour en { $version }
installed-no-version = Installé : sans version
installed-version = Installé : { $version }
installed = Installé
install = Installer
by-author = Par { $author }
home-page = Page d'accueil
plugin-toggle-hover = Activer ou désactiver le module
plugin-running = Actif
plugin-stopped = Arrêté
plugin-off = Désactivé
plugin-needs-permission = En attente d'autorisation
plugin-not-running = Inactif
update-plugin-hover = Télécharger la nouvelle version depuis l'index ; l'ancienne va dans la corbeille
restart = Redémarrer
allow-dots = Autoriser…
start = Démarrer
open-folder = Ouvrir le dossier
remove-dots = Supprimer…
remove-plugin-hover = Mettre le dossier du module à la corbeille
trust-changes = Ne pas demander à chaque modification
trust-changes-hover = Pour un module que vous développez : l'exécuter après chaque modification sans redemander
clear = Effacer
copy-all = Tout copier
plugin-log-hint = Messages et sorties de print() et console.log() des modules
nothing-yet = Rien pour l'instant.
allow-plugin-title = Autoriser « { $name } » à s'exécuter ?
plugin-changed = Ce module a changé depuis que vous l'avez autorisé.
allow-plugin-warning = Les modules s'exécutent comme des programmes séparés avec votre compte utilisateur. Celui-ci pourrait lire et modifier vos documents ouverts et vos fichiers, et utiliser le réseau. N'autorisez que des modules de confiance.
author = Auteur
runs = Exécute
allow = Autoriser
dont-allow = Ne pas autoriser
dont-allow-hover = Désactiver le module. Vous pourrez le réactiver dans Gérer les modules.

## Installation des modules (src/plugins/store.rs)
index-bad-json = L'index des modules n'est pas du JSON valide : { $error }
index-no-list = L'index des modules n'a pas de liste "plugins"
zip-unsafe-name = Le zip contient un nom de fichier dangereux, rien n'a été installé : { $name }
zip-no-manifest = Le zip ne contient pas { $file }, ni à la racine ni dans un dossier unique.
zip-unreadable = Ce n'est pas un fichier zip lisible par Ettin Edit : { $error }
zip-too-many-files = Le zip contient plus de { $count } fichiers.
zip-damaged = Le zip est endommagé : { $error }
zip-has-link = Le zip contient un lien, ce qui est interdit dans un module : { $name }
zip-encrypted = Le zip est protégé par mot de passe.
zip-too-big = Le zip décompressé dépasse 256 Mo.
checksum-mismatch = Le téléchargement ne correspond pas à sa somme SHA-256 (attendu { $expected }, obtenu { $got }). Il est peut-être endommagé ou modifié ; rien n'a été installé.
already-in-plugins-folder = Ce dossier est déjà dans le dossier des modules.
copy-failed = Impossible de copier { $path } : { $error }
not-a-plugin = Ce n'est pas un module exécutable par Ettin Edit : { $error }
plugin-trash-old-failed = Impossible de mettre la copie installée de { $id } à la corbeille : { $error }
plugin-move-failed = Impossible de mettre le module en place : { $error }
index-unreachable = L'index des modules est injoignable ({ $error }). Vérifiez l'adresse et qu'elle est publique :
    { $url }
installing = Installation de { $name }…

## Exécution des modules (src/plugins/mod.rs)
plugin-started = Démarré avec { $program } { $script }
runtime-missing = { $runtime } n'est pas installé (« { $program } » introuvable). Installez-le ou indiquez son chemin dans Modules > Gérer les modules.
plugin-start-failed = Impossible de démarrer { $program } : { $error }
plugin-exited = Il s'est arrêté.
plugin-exit-code = Il s'est arrêté avec le code de sortie { $code }. Voir le journal des modules.
plugin-died = Il s'est arrêté. Voir le journal des modules.
plugin-stopped-notice = Le module { $name } s'est arrêté. Voir Modules > Journal des modules.

## Noms des raccourcis (src/plugins/shortcut.rs)
unknown-modifier = Modificateur inconnu « { $modifier } » dans « { $shortcut } »
unknown-key = Touche inconnue « { $key } » dans « { $shortcut } »
shortcut-needs-modifier = « { $shortcut } » nécessite Ctrl ou Alt, sinon il gênerait la saisie
shortcut-reserved = « { $shortcut } » est réservé à couper, copier et coller

## Listes de complétion (src/lang_api.rs)
api-file-skipped = Fichier d'API { $name } ignoré : { $error }

## Fichiers de langage (src/highlight.rs)
language-file-skipped = Fichier de langage { $file } ignoré : { $error }

## Panneaux des modules (src/plugins/panel.rs)
close-panel = Fermer le panneau

## Paramètres des raccourcis (src/actions.rs)
shortcut-settings-error = Paramètres des raccourcis : { $error }

## Toolbar (src/ribbon*.rs, src/app/ribbon.rs)
apply = Appliquer
customise-toolbar = Personnaliser la barre d'outils
customise-toolbar-dots = Personnaliser la barre d'outils…
toolbar-show = Barre d'outils
toolbar-collapse-menu = Réduire la barre d'outils
toolbar-collapse = Réduire la barre d'outils
toolbar-expand = Afficher les boutons de la barre d'outils
toolbar-hide = Masquer la barre d'outils
toolbar-empty = Aucun bouton pour l'instant. Clic droit pour personnaliser la barre d'outils.
toolbar-remove-button = Retirer de la barre d'outils
toolbar-read-failed = Impossible de lire la disposition de la barre d'outils { $path } : { $error }. La barre d'outils par défaut est utilisée.
toolbar-save-failed = Impossible d'enregistrer la disposition de la barre d'outils dans { $path } : { $error }
toolbar-commands = Commandes
toolbar-filter-hint = Filtrer les commandes
toolbar-layout = Barre d'outils
toolbar-drag-hint = Glissez des commandes ici, ou glissez onglets, groupes et boutons pour les déplacer.
toolbar-add = Ajouter ›
toolbar-add-tip = Ajouter la commande choisie après la sélection à droite
toolbar-remove = ‹ Retirer
toolbar-move-up = Monter
toolbar-move-down = Descendre
toolbar-new-tab = Nouvel onglet
toolbar-new-group = Nouveau groupe
toolbar-new-tab-name = Nouvel onglet
toolbar-new-group-name = Nouveau groupe
toolbar-no-tabs = Il n'y a aucun onglet. Ajoutez-en un avec Nouvel onglet.
toolbar-tab-visible = Afficher cet onglet
toolbar-unavailable = Cette commande n'est pas disponible pour l'instant (son module ne tourne peut-être pas). Le bouton reviendra quand elle le sera.
toolbar-select-hint = Sélectionnez un onglet, un groupe ou un bouton pour le modifier.
toolbar-tab-name = Nom de l'onglet :
toolbar-group-name = Nom du groupe :
toolbar-label = Libellé :
toolbar-size = Taille :
toolbar-size-large = Grand
toolbar-size-small = Petit
toolbar-icon = Icône :
toolbar-choose-icon = Choisir une icône
toolbar-change-icon = Changer…
toolbar-default-icon = Utiliser l'icône de la commande
toolbar-icon-filter-hint = Filtrer les icônes
toolbar-icon-credit = Icônes : Phosphor (licence MIT)
toolbar-reset = Rétablir les valeurs par défaut
toolbar-tab-home = Accueil
toolbar-tab-edit = Édition
toolbar-tab-view = Affichage
toolbar-tab-tools = Outils
toolbar-group-file = Fichier
toolbar-group-clipboard = Presse-papiers
toolbar-group-history = Annuler
toolbar-group-find = Rechercher
toolbar-group-tabs = Onglets
toolbar-group-lines = Lignes
toolbar-group-sort = Trier
toolbar-group-case = Casse
toolbar-group-comment = Commentaire
toolbar-group-blanks = Blancs
toolbar-group-selection = Sélection
toolbar-group-show = Afficher
toolbar-group-zoom = Zoom
toolbar-group-folding = Pliage
toolbar-group-panels = Panneaux
toolbar-group-window = Fenêtre
toolbar-group-compare = Comparer
toolbar-group-json = JSON
toolbar-group-xml = XML
toolbar-group-data = Données
toolbar-group-macros = Macros
toolbar-group-customise = Personnaliser
