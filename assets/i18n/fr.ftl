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
status-length-lines = Longueur : { $length }   Lignes : { $lines }
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
xml-pretty-print = Mettre en forme le XML
xml-pretty-print-attributes = Mettre en forme le XML, attributs sur des lignes séparées
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
plugin-enabled = Activé
plugin-adds = Ajoute :
plugin-search-installed = Rechercher dans les modules installés
plugins-only-samples = Seuls les modules d'exemple sont listés pour l'instant. Vous avez écrit un module ?
plugins-submit-link = Proposez-le dans l'index
samples-installed = { $count ->
    [one] 1 module installé.
   *[other] { $count } modules installés.
} Retrouvez-les dans le menu Modules.
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
style-bold = Gras
style-italic = Italique
style-underline = Souligné
set-colour = Définir
choose-colour = Choisir une couleur
theme-read-failed = Impossible de lire le fichier de thème : { $error }
theme-colours = Couleurs du thème
theme-label = Thème :
save-as-new-theme = Enregistrer comme nouveau thème…
new-theme-name = Nom du nouveau thème :
new-theme-name-hint = Nom du nouveau thème
theme-name-taken = Un thème porte déjà ce nom
theme-colours-hint = Les changements s'affichent immédiatement et sont enregistrés aussitôt. Annuler les modifications rétablit le thème tel qu'il était à l'ouverture de la fenêtre.
theme-changes-count = { $count ->
    [one] 1 réglage modifié depuis l'ouverture.
   *[other] { $count } réglages modifiés depuis l'ouverture.
}
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
hex-now-binary = Ce n'est plus du texte { $encoding } valide : cet onglet est donc modifié en octets. Annulez pour revenir au texte.
hex-text-again = De nouveau du texte valide : retour à la modification du texte.
hex-binary = Modifié en octets
hex-not-text = Ces octets ne sont pas du texte { $encoding } valide : l'onglet reste en vue hexadécimale.
hex-too-big-for-text = Ce fichier est trop gros pour être affiché en texte : il reste en vue hexadécimale.
hex-copy-too-big = Sélectionnez 16 Mo au plus pour copier.
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
update-now = Mettre à jour
update-release-notes = Notes de version
later = Plus tard
restart-now = Redémarrer
install-and-restart = Installer et redémarrer
self-update-how-arch = Télécharge le paquet Arch et l'installe avec pacman. Le système demande votre mot de passe.
self-update-how-deb = Télécharge le paquet Debian et l'installe avec apt. Le système demande votre mot de passe.
self-update-how-portable = Télécharge la nouvelle version et remplace le programme de cette copie.
self-update-how-setup = Télécharge le programme d'installation. Ettin Edit se ferme, installe la mise à jour et redémarre.
self-update-how-rpm = Télécharge le paquet RPM et l'installe avec dnf (ou zypper). Le système demande votre mot de passe.
self-update-how-appimage = Télécharge le nouvel AppImage et le met à la place de celui-ci.
self-update-flatpak = Cette copie est un Flatpak : votre logithèque ou la commande flatpak update installe les nouvelles versions.
self-update-managed = Cette copie vient de { $store }, qui installe les nouvelles versions.
self-update-no-exe = Impossible de savoir où Ettin Edit est installé ; il ne peut donc pas se mettre à jour.
self-update-no-build = Aucune version prête n'existe pour cet ordinateur ; Ettin Edit ne peut donc pas se mettre à jour.
self-update-source-build = Cette copie a été compilée depuis les sources ; mettez-la à jour avec git et cargo.
self-update-read-only = Ettin Edit ne peut pas écrire dans { $path } et ne peut donc pas s'y mettre à jour.
self-update-no-file = Cette version n'a pas de fichier pour ce type d'installation ; téléchargez-la depuis la page de la version.
self-update-no-checksum = Aucune somme de contrôle n'est indiquée pour { $file } ; il ne peut donc pas être vérifié. Téléchargez-le plutôt depuis la page de la version.
self-update-save-failed = Impossible d'enregistrer le téléchargement : { $error }
self-update-download-failed = Impossible de télécharger la mise à jour : { $error }
self-update-cancelled = Le téléchargement a été annulé.
self-update-checksum-mismatch = { $file } ne correspond pas à sa somme de contrôle ; il n'a pas été installé.
self-update-install-failed = L'installation a échoué : { $error }
self-update-not-authorised = L'installation demande un mot de passe administrateur, et aucun n'a été donné.
self-update-unpack-failed = Impossible de décompresser la mise à jour : { $error }
self-update-missing-file = { $file } est absent du téléchargement
self-update-replace-failed = Impossible de remplacer le programme : { $error }
self-update-downloading = Téléchargement : { $done } sur { $total } Mo
self-update-installing = Installation…
self-update-password = Le système demande un mot de passe administrateur.
self-update-installed = Ettin Edit { $version } est installé. Redémarrez pour l'utiliser.
self-update-ready = Ettin Edit { $version } est téléchargé. Ettin Edit se ferme, l'installe et redémarre. Choisissez Plus tard pour l'installer à la prochaine fermeture d'Ettin Edit.
self-update-run-in-terminal = Exécutez ceci dans un terminal pour terminer la mise à jour :
self-update-failed = Impossible de mettre à jour Ettin Edit.

## Fenêtre du manuel d'utilisation (src/manual.rs)
back = Précédent
forward = Suivant
search-manual = Rechercher dans le manuel
clear-search = Effacer la recherche
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
customise-toolbar = Personnaliser la barre d'outils
customise-toolbar-dots = Personnaliser la barre d'outils…
toolbar-show = Barre d'outils
toolbar-collapse-menu = Réduire la barre d'outils
toolbar-collapse = Réduire la barre d'outils
toolbar-expand = Afficher les boutons de la barre d'outils
toolbar-hide = Masquer la barre d'outils
toolbar-more = Plus de commandes
toolbar-hidden-groups = GROUPES MASQUÉS
toolbar-filter-command = Filtre commande
menu-bar-show = Barre de menus
menu-bar-hide = Masquer la barre de menus
menu-bar-show-again = Afficher la barre de menus
menu-bar-hidden-notice = La barre de menus est masquée. Appuyez sur Alt pour l'afficher un instant, ou sur { $shortcut } pour la rétablir.
menu-bar-hidden-notice-alt = La barre de menus est masquée. Appuyez sur Alt pour l'afficher, puis choisissez Affichage > Barre de menus pour la rétablir.
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
toolbar-group-help = Aide
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

## Transform menu (1.9.5)

menu-transform = Transformer
menu-transform-lines = Lignes
menu-hash = Empreinte
pipe-dots = Filtrer par une commande…
keep-matching-lines = Garder les lignes correspondantes
remove-matching-lines = Supprimer les lignes correspondantes
keep-matching-lines-dots = Garder les lignes correspondantes…
remove-matching-lines-dots = Supprimer les lignes correspondantes…
count-distinct-lines = Compter les lignes distinctes
affix-lines = Ajouter un préfixe et un suffixe
affix-lines-dots = Ajouter un préfixe et un suffixe…
number-lines = Numéroter les lignes
number-lines-dots = Numéroter les lignes…
reflow = Reformater les paragraphes
reflow-dots = Reformater les paragraphes…
convert-to = { $from } vers { $to }
case-camel = camelCase
case-pascal = PascalCase
case-snake = snake_case
case-kebab = kebab-case
case-constant = CONSTANT_CASE
hex-encode = Texte vers hex
hex-decode = Hex vers texte
jwt-decode = Décoder un JWT
hash-copied = { $kind } du document copié : { $hash }
jwt-not-verified = Décodé ; la signature n'a pas été vérifiée.
jwt-issued = Émis le { $time }.
jwt-expires = Expire le { $time }.
jwt-expired = Expiré le { $time }.
pipe-title = Filtrer par une commande
pipe-hint = La sélection (ou tout le document) est envoyée à la commande, et sa sortie va là où l'indique « Résultat vers ». Les commandes s'exécutent dans le dossier du fichier ; $ETTIN_FILE contient son chemin.
pipe-run = Exécuter
pipe-recent = Commandes récentes
pipe-running = { $command } en cours…
pipe-no-command = Saisissez une commande.
pipe-tab-closed = L'onglet a été fermé avant que la commande puisse s'exécuter.
pipe-start-failed = La commande n'a pas pu démarrer : { $error }
pipe-failed = La commande a échoué (code de sortie { $code }) ; le texte n'a pas été modifié.
pipe-text-changed = Le texte a changé pendant l'exécution de la commande ; sa sortie n'a pas été utilisée.
filter-lines-title = Garder ou supprimer les lignes correspondantes
filter-lines-hint = S'applique aux lignes sélectionnées, ou à tout le document.
filter-keep = Garder les lignes correspondantes
filter-remove = Supprimer les lignes correspondantes
filter-no-pattern = Saisissez le texte à chercher.
affix-lines-title = Ajouter un préfixe et un suffixe
affix-lines-hint = Ajoute du texte au début et à la fin de chaque ligne sélectionnée, ou de chaque ligne.
affix-prefix = Avant chaque ligne
affix-suffix = Après chaque ligne
skip-empty-lines = Ignorer les lignes vides
number-lines-title = Numéroter les lignes
number-lines-hint = Place un numéro devant chaque ligne sélectionnée, ou chaque ligne.
number-separator = Puis
number-pad-none = Sans remplissage
number-pad-spaces = Aligner
number-pad-zeros = Zéros devant
reflow-title = Reformater les paragraphes
reflow-hint = Recoupe chaque paragraphe pour qu'aucune ligne ne dépasse la largeur. L'indentation, les marques de commentaire et les puces sont conservées.
reflow-width = Largeur
toolbar-tab-transform = Transformer
toolbar-group-command = Commande
toolbar-group-filter = Lignes
toolbar-group-identifier-case = Identifiants
toolbar-group-convert = Convertir
toolbar-group-encode = Encoder
# Recettes (2.0)
menu-recipes = Recettes
manage-recipes = Gérer les recettes…
record-recipe = Enregistrer une recette
stop-recording-recipe = Arrêter l'enregistrement de la recette
open-recipes-folder = Ouvrir le dossier des recettes
reload-recipes = Relire le dossier des recettes
run-recipe-on-all-tabs = Appliquer à tous les onglets ouverts
toolbar-group-recipes = Recettes
recording-recipe = Enregistrement de recette ({ $count } étapes)
recording-recipe-hover = Chaque commande Transformer, de lignes, de casse, d'encodage et Tout remplacer devient une étape. Cliquez pour arrêter et enregistrer la recette.
recipes-title = Recettes
recipes-pick = Choisissez une recette à gauche, ou créez-en une.
recipes-none = Aucune recette pour l'instant.
recipe-new = Nouvelle recette
recipe-new-name = Nouvelle recette
recipe-recorded-name = Recette enregistrée
recipe-duplicate = Dupliquer
recipe-delete = Supprimer
recipe-delete-dots = Supprimer…
recipe-delete-question = Supprimer « { $name } » ?
recipe-import = Importer des recettes…
recipe-export = Exporter la recette…
recipe-file-filter = Recette Ettin Edit
recipe-name = Nom
recipe-description = Description
recipe-no-name = Donnez un nom à la recette.
recipe-name-taken = Une autre recette porte ce nom ; les menus et raccourcis lancent la première.
recipe-steps = Étapes
recipe-no-steps = Pas encore d'étapes. Ajoutez-en une ci-dessous, ou enregistrez une recette depuis Transformer > Recettes.
recipe-add-step = Ajouter une étape
recipe-move-up = Monter
recipe-move-down = Descendre
recipe-remove-step = Retirer cette étape
recipe-group-replace = Rechercher et remplacer
recipe-group-encode = Encoder et hacher
recipe-group-format = JSON et XML
recipe-group-command = Commande
recipe-step-replace = Remplacer « { $find } » par « { $replace } »
recipe-step-keep = Garder les lignes avec « { $pattern } »
recipe-step-remove = Supprimer les lignes avec « { $pattern } »
recipe-step-affix = Ajouter « { $prefix } » avant et « { $suffix } » après chaque ligne
recipe-step-number = Numéroter les lignes à partir de { $start }, pas de { $step }
recipe-step-reflow = Reformater les paragraphes sur { $width } colonnes
recipe-step-hash = Empreinte { $kind } du texte
recipe-step-join = Joindre les lignes
recipe-step-json = JSON : { $action }
recipe-step-xml = XML : { $action }
recipe-step-command = Lancer la commande : { $command }
recipe-replace-with = Remplacer par
recipe-pattern = Lignes avec
recipe-convert-from = De
recipe-convert-to = Vers
recipe-descending = Décroissant
recipe-only-consecutive = Seulement les répétitions qui se suivent
recipe-include-blank = Aussi les lignes faites seulement d'espaces et de tabulations
recipe-command = Commande
recipe-command-hint = S'exécute dans le dossier du fichier avec le texte en entrée ; sa sortie devient le texte. On vous demande avant de lancer une recette avec une commande.
recipe-hash-hint = Remplace le texte par son empreinte.
recipe-count-hint = Remplace le texte par chaque ligne distincte et son nombre d'occurrences.
recipe-no-settings = Cette étape n'a pas de réglages.
recipe-no-find = Tapez le texte à rechercher.
recipe-step-not-allowed = Cette commande ne peut pas être une étape de recette.
recipe-preview-selection = Sélection
recipe-before = Avant
recipe-after = Après
recipe-step-failed = Étape { $step } : { $message }
recipe-preview-stopped = L'aperçu s'arrête avant l'étape { $step }, qui lance une commande.
recipe-run-all-tabs = Appliquer à tous les onglets
recipe-confirm-title = Lancer des commandes ?
recipe-confirm-text = « { $name } » lance ces commandes sur votre ordinateur :
recipe-confirm-hint = Ne lancez que des commandes de confiance. Elles peuvent lire et modifier vos fichiers.
recipe-confirm-remember = Ne plus demander pour cette recette tant que ses commandes ne changent pas
recipe-confirm-run = Lancer les commandes
recipe-running = « { $name } » en cours…
recipe-stopped = Arrêté.
recipe-nothing-recorded = Rien n'a été enregistré. Seules les commandes qui modifient le texte deviennent des étapes.
recipe-not-found = Aucune recette ne s'appelle « { $name } ».
recipe-failed = « { $name } » s'est arrêtée à l'étape { $step } : { $message } Le texte n'a pas été modifié.
recipe-changed-tabs = « { $name } » a modifié { $count } onglets.
recipe-no-change = « { $name } » n'a rien changé.
recipe-read-failed = Impossible de lire la recette { $path } : { $error }
recipe-save-failed = Impossible d'enregistrer la recette { $path } : { $error }

## 2.0.2 : Convertir en, Lire comme
menu-convert-to = Convertir en
menu-read-as = Lire comme
convert-to-format = Convertir en { $format }
convert-markdown-table = Tableau Markdown
convert-reading = Lecture de l'onglet…
convert-reading-as = Cet onglet est lu comme { $format }
convert-not-data = Ni JSON, ni YAML, ni TOML, ni XML, ni CSV
convert-unknown-format = Impossible de reconnaître le format de ce texte. Choisissez-en un dans Transformer › Convertir en › Lire comme.
converted-to = Converti en { $to } (lu comme { $from })
read-as-auto = Automatique
read-as-auto-format = Automatique ({ $format })
read-as-named = Lire comme { $format }
toolbar-convert-to = En { $format }
## 2.0.2 : palette de commandes
palette-recipe = Recette · { $steps ->
    [one] 1 étape
   *[other] { $steps } étapes
}
palette-plugin = Module · { $name }
palette-not-for-tab = pas pour cet onglet
palette-tag-recent = récent
palette-tag-recipe = recette
palette-tag-plugin = module
## 2.0.2 : fenêtre Recettes
recipe-new-short = Nouvelle
recipe-export-short = Exporter…
recipe-import-short = Importer…
recipe-folder = Dossier
recipe-reload = Recharger
recipe-no-shortcut = Aucun
recipe-clear-shortcut = Effacer
recipe-id = id { $id }
recipe-id-hint = Les raccourcis et les boutons de la barre d'outils utilisent cet id : ils continuent de fonctionner si vous renommez la recette.
recipe-step-search = Rechercher une étape…
json-format-named = Formater le JSON
json-minify-named = Minifier le JSON
## 2.1: Transform first
menu-plugin-transforms = Transformations des modules
menu-sort-lines = Trier les lignes
menu-trim = Supprimer les espaces
menu-macros = Macros
menu-toolbar = Barre d'outils
menu-encode-decode = Encoder / Décoder
more-edit-commands = Autres commandes d'édition
context-selection = Sélection
sort-lines = Trier les lignes
open-in-csvquery = Ouvrir dans CsvQuery
xml-check-syntax-named = Vérifier la syntaxe XML
xml-validate-named = Valider le XML
xml-validate-against-named = Valider le XML avec…
recipes-menu-note = Les recettes rejouent des transformations et peuvent s'exécuter en ligne de commande.
macros-menu-note = Les macros rejouent les frappes et les modifications.
run-last-recipe = Relancer la dernière recette
save-macro-as-recipe = Enregistrer la macro comme recette…
save-macro-as-recipe-hint = Transforme la macro enregistrée en recette modifiable, partageable et utilisable en ligne de commande.
macro-not-recipe = Impossible d'en faire une recette : { $step } n'est pas une transformation.
macro-step-typing = la saisie « { $text } »
macro-step-key = la touche { $key }
recipe-from-macro-name = Recette depuis une macro
toolbar-simplified = Barre d'outils simplifiée
toolbar-mode-full = Complète
toolbar-mode-simplified = Simplifiée
toolbar-mode-full-tip = Afficher le ruban complet
toolbar-mode-simplified-tip = Afficher une rangée de petits boutons
toolbar-help = Aide
toolbar-menus = Menus déroulants
toolbar-menu-format = Formater
toolbar-menu-case = Casse
toolbar-group-transform = Transformer
toolbar-group-quick = Rapide
toolbar-group-search = Recherche
toolbar-keep-matching = Garder les lignes
toolbar-remove-duplicates = Supprimer les doublons

## 2.2: previews, Result goes to, suggestions, Welcome
recipe-preview-active-tab = Onglet actif
recipe-preview-input = Aperçu sur :
recipe-sample-text = Texte d'exemple
recipe-sample-hint = Texte d'exemple (enregistré avec la recette)
recipe-show-after = Afficher le résultat après :
recipe-after-step = Étape { $step }
recipe-after-last-step = La dernière étape
recipe-output-after-step = Après l'étape { $step }
filter-pattern = Motif
filter-keep-button = Garder les lignes
filter-remove-button = Supprimer les lignes
filter-kept-count = { $kept } lignes sur { $lines } gardées
filter-marks-hint = dans l'éditeur, les lignes gardées sont en vert, les lignes supprimées en rouge
filter-counting = Comptage des lignes correspondantes…
pipe-command = Commande
pipe-preview = Aperçu
pipe-preview-tip = Lancer la commande sur le texte sans rien modifier
pipe-input-tab = Entrée (tout l'onglet)
pipe-input-selection = Entrée (sélection)
pipe-output = Sortie
pipe-output-done = Sortie · { $seconds } s · code de sortie { $code }
pipe-output-hint = Appuyez sur Aperçu pour voir ce que la commande fait du texte. Les commandes ne s'exécutent que sur Aperçu ou Exécuter.
pipe-previewing = Exécution de { $command }…
pipe-no-stderr = La commande a échoué sans rien écrire sur la sortie d'erreur.
pipe-preview-stale = La commande ou le texte ont changé depuis cet aperçu.
preview-first-lines = Entrée ({ $lines } premières lignes)
result-goes-to = Résultat vers
result-this-tab = Cet onglet
result-new-tab = Nouvel onglet
result-clipboard = Presse-papiers
result-this-tab-failed = La commande a échoué dans l'aperçu ; sa sortie ne peut donc pas remplacer le texte.
result-tab-unchanged = Cet onglet reste tel quel.
result-copied = Résultat copié dans le presse-papiers ({ $lines } lignes).
size-bytes = { $bytes } octets
suggest-minified-json = Ce JSON tient sur une seule ligne ({ $size }).
suggest-minified-xml = Ce XML tient sur une seule ligne ({ $size }).
suggest-jwt = Cela ressemble à un jeton JWT (JSON Web Token).
suggest-base64 = Cela ressemble à du texte en Base64.
suggest-csv = Cela ressemble à du CSV.
suggest-mixed-line-endings = Ce fichier mélange les fins de ligne (LF et CRLF).
suggest-mixed-indentation = Ce fichier est indenté à la fois avec des tabulations et des espaces.
suggest-format = Formater
suggest-show-tree = Afficher en arbre
suggest-decode = Décoder
suggest-open-csvquery = Ouvrir dans CsvQuery
suggest-show-table = Afficher en tableau
suggest-fix = Corriger
suggest-fix-lf = Utiliser LF partout
suggest-fix-crlf = Utiliser CRLF partout
suggest-fix-spaces = Tabulations en espaces
suggest-fix-tabs = Espaces en tabulations
suggest-never = Ne plus suggérer
suggest-close = Fermer
prefs-suggestions-heading = Suggestions
prefs-suggestions = Suggérer des transformations pour les fichiers ouverts ou collés (comme Formater pour du JSON sur une ligne)
prefs-suggestions-reset = Afficher à nouveau les { $count } suggestions écartées
welcome = Bienvenue
welcome-tab = Bienvenue
welcome-title = Bienvenue dans Ettin Edit
welcome-whats-new-title = Nouveautés d'Ettin Edit { $version }
welcome-intro = Un éditeur de texte fait pour transformer du texte en un autre texte : filtrer des journaux, convertir du JSON en YAML, décoder des jetons, et enregistrer les étapes dans une recette.
welcome-card-transform = filtrer des lignes, convertir JSON/YAML/CSV, changer la casse, encoder et hacher
welcome-card-palette-title = Palette de commandes
welcome-card-palette = tapez ce que vous voulez faire
welcome-card-recipes = enregistrer une suite d'étapes et la rejouer, même en ligne de commande
welcome-card-pipe = faire passer le texte par n'importe quelle commande shell
welcome-try-sample = Essayer sur un texte d'exemple
welcome-open-manual = Ouvrir le manuel
welcome-open-file = Ouvrir un fichier…
welcome-show-at-start = Afficher cet onglet au démarrage d'Ettin Edit
welcome-whats-new = Nouveautés de la { $version }
welcome-full-notes = Notes de version complètes
welcome-whats-new-hint = Après une mise à jour, cet onglet s'ouvre une fois pour montrer les nouveautés. Aide › Bienvenue le rouvre.
tour-step-of = Étape { $step } sur { $steps } :
tour-step-keep = ne garder que les lignes ERROR avec Transformer › Lignes › Garder les lignes correspondantes. Les lignes passent en vert et en rouge avant que vous appuyiez sur Garder les lignes.
tour-step-convert = convertir la ligne JSON en YAML avec Transformer › Convertir en › YAML.
tour-done = C'est tout : deux transformations, chacune annulable avec Ctrl+Z. Enregistrez les étapes dans une recette pour les rejouer sur n'importe quel fichier.
tour-show-me = Montrez-moi
tour-end = Terminer la visite

# Plugins as transforms (2.3)
plugin-error-at = { $name } s'est arrêté sur une erreur : { $file } ligne { $line } : { $error }
open-file-button = Ouvrir
plugin-transform-error-at = { $message } (ligne { $line }, colonne { $column })
plugin-transform-error-line = { $message } (ligne { $line })
recipe-plugin-missing = Le module « { $plugin } » n'est pas installé. Installez-le avec Modules › Gérer les modules.
recipe-plugin-not-allowed = Le module « { $plugin } » n'est pas autorisé. Autorisez-le d'abord dans Ettin Edit (Modules › Gérer les modules).
plugin-no-documents-here = Les onglets ne sont pas disponibles ici : une transformation ne travaille que sur le texte reçu.
plugin-ended-during-transform = Le module « { $plugin } » s'est arrêté avant d'avoir fini.
plugin-transform-timeout = « { $plugin } » a pris plus de { $seconds } secondes ; il a été arrêté.
recipe-plugin-no-transform = Le module « { $plugin } » n'a pas de transformation « { $transform } ».
plugin-transform-no-text = La transformation du module n'a pas renvoyé de texte.
choose-hint = Tapez pour filtrer
copy-command-id = Copier l'identifiant de commande
copy-command-id-hover = Copie l'identifiant que les modules passent à run_action pour lancer cette commande
done = Terminé
markdown-preview = Aperçu Markdown
markdown-preview-title = Aperçu : { $title }
markdown-preview-not-markdown = Cet onglet n'est pas du Markdown. Ouvrez un fichier .md ou passez à l'un d'eux pour l'afficher.
markdown-preview-too-big = Cet onglet est trop gros pour un aperçu pendant la saisie.
markdown-preview-empty = Rien à afficher pour l'instant. Commencez à taper du Markdown.
markdown-preview-sync = Défiler avec le texte
markdown-preview-save-first = Enregistrez d'abord cet onglet pour que les liens vers d'autres fichiers soient trouvés.
markdown-preview-missing = Impossible de trouver { $path }
new-plugin-dots = Nouveau module…
new-plugin-title = Nouveau module
new-plugin-name = Nom
new-plugin-default-name = Mon module
new-plugin-language = Langage
new-plugin-starts-with = Commence par
new-plugin-transform = Une transformation (texte en entrée, texte en sortie)
new-plugin-command = Une commande dans le menu Modules
new-plugin-panel = Un panneau latéral
new-plugin-create = Créer et ouvrir
new-plugin-folder = Il sera placé dans { $folder }
new-plugin-exists = { $folder } existe déjà. Choisissez un autre nom.
new-plugin-failed = Impossible d'écrire le module dans { $folder } : { $error }
plugin-adds-transforms = Transformations :
plugin-options-dots = Options…
plugin-options-hover = Modifier les réglages de ce module
plugin-options-title = Options de { $name }
plugin-options-intro = Les changements s'appliquent tout de suite ; le module en est informé.
plugin-options-reset = Rétablir les valeurs par défaut
plugin-options-escapes = Dans le texte, \n est un saut de ligne et \t une tabulation.
plugin-transform-from = Du module { $plugin }
plugin-previewing = Aperçu en cours…
plugin-preview-first-lines = L'aperçu montre les { $count } premières lignes.
plugin-transform-gone = « { $transform } » n'est pas disponible : son module n'est pas lancé.
plugin-reloaded = Rechargé après la modification de ses fichiers.
recipe-plugin-needs-plugins = Cette étape lance une transformation de module, qui ne peut pas s'exécuter ici.
recipe-plugin-step-empty = Choisissez le module et la transformation de cette étape.
recipe-step-plugin = { $title } ({ $plugin })
recipe-preview-stopped-plugin = L'aperçu s'arrête avant l'étape { $step }, qui lance une transformation de module.
reload-on-change = Recharger quand ses fichiers changent
reload-on-change-hover = Pour les modules que vous écrivez : le relance moins d'une seconde après l'enregistrement d'un de ses fichiers.

## 2.4 Les réglages au même endroit
changes-apply-live = Les changements s'appliquent immédiatement.
undo-changes = Annuler les modifications
undo-changes-tip = Tout remettre comme à l'ouverture de cette fenêtre
reset-section = Réinitialiser la section
reset-section-tip = Remettre les réglages { $section } à leurs valeurs par défaut
theme-reset-tip = Remettre « { $name } » tel qu'Ettin Edit le fournit
theme-reset-custom-tip = Seuls les thèmes fournis peuvent être réinitialisés. Annuler les modifications remet ce thème tel qu'il était à l'ouverture de la fenêtre.
recipes-undo-tip = Remettre les recettes comme à l'ouverture de cette fenêtre (les recettes ajoutées depuis sont supprimées)
prefs-search-hint = Rechercher un réglage
prefs-also = AUSSI
prefs-other-windows = Autres fenêtres
prefs-no-match = Aucun réglage ne correspond à « { $search } ».
prefs-appearance = Apparence
prefs-languages = Langages
prefs-theme = Thème
prefs-follow-system = Suivre le mode clair/sombre du système
prefs-light-dark-pair = Paire claire / sombre
prefs-interface = Interface
prefs-interface-size = Taille de l'interface
prefs-toolbar = Barre d'outils
prefs-toolbar-full = Ruban complet
prefs-toolbar-hidden = Masquée
prefs-menu-bar = Barre de menus
prefs-tab-key = La touche Tab insère
prefs-detect-indent = Détecter l'indentation à l'ouverture d'un fichier (examine ses 1000 premières lignes indentées)
prefs-per-language = Réglages par langage
prefs-per-language-hint = Ils l'emportent sur les réglages ci-dessus. Quand la détection est active, l'indentation du fichier l'emporte sur les deux.
prefs-default = Par défaut
prefs-tabs = Tabulations
prefs-spaces = Espaces
prefs-remove-language = Supprimer les réglages de ce langage
prefs-pick-language = Choisir un langage…
prefs-add = Ajouter
prefs-hash-no-selection = Hachage sans sélection
prefs-hash-clipboard = Copier le hachage dans le presse-papiers
prefs-hash-new-tab = Ouvrir le hachage dans un nouvel onglet
prefs-recipe-commands = Étapes de commande des recettes
prefs-ask-first = Demander avant d'exécuter
prefs-run-without-asking = Exécuter sans demander
prefs-shell = Shell des commandes
prefs-shell-tip = Le programme et ses options qui exécutent Filtrer par une commande et les étapes de commande des recettes, comme bash -c ou pwsh -Command. Vide utilise le shell du système.
prefs-format-indent = Indentation JSON / XML
prefs-indent-document = Comme le document
prefs-indent-n-spaces = { $count } espaces
prefs-settings-files = Fichiers de réglages
prefs-settings-files-hint = Exporter place vos réglages, raccourcis, barre d'outils, thèmes, recettes et langages dans un seul fichier à copier sur un autre ordinateur.
keymap-label = Raccourcis
keymap-ettin = Ettin (style Notepad++)
keymap-vs-code = Style VS Code
keymap-hint = Vos propres changements s'ajoutent par-dessus.
export-settings = Exporter les réglages
export-settings-dots = Exporter les réglages…
import-settings = Importer les réglages
import-settings-dots = Importer les réglages…
open-settings-folder = Ouvrir le dossier des réglages
settings-file-type = Réglages d'Ettin Edit
settings-exported = { $count } fichiers exportés vers { $file }
settings-export-failed = Impossible d'exporter les réglages vers { $file } : { $error }
settings-import-failed = Impossible d'importer { $file } : { $error }
settings-imported = Réglages importés.
settings-imported-backup = Réglages importés. Vos anciens fichiers sont dans { $folder }
import = Importer
import-from = { $file } (depuis Ettin Edit { $version })
import-not-settings = ce fichier n'a pas été créé par Exporter les réglages
import-contents = Fichiers de réglages : { $settings } · Thèmes : { $themes } · Recettes : { $recipes } · Langages : { $languages }
import-replaces-nothing = Aucun de vos fichiers actuels n'est remplacé.
import-replaces = { $count ->
    [one] Il remplace 1 de vos fichiers actuels :
   *[other] Il remplace { $count } de vos fichiers actuels :
}
import-backup-note = Une copie de chaque fichier remplacé est d'abord placée dans le dossier backups du dossier des réglages.
import-plugins-missing = Modules utilisés avec ces réglages et absents ici : { $plugins }. Installez-les depuis Modules > Gérer les modules.
live-reloaded = { $file } rechargé
live-reload-failed = { $file } a changé mais ne peut pas être lu : { $error }. La dernière version valide reste utilisée.
live-reload-place = ligne { $line }, colonne { $column }
