# Ettin Edit's interface text in German (Deutsch).
# This translation was made by machine and needs review by a native speaker.
#
# Each line is `id = text`. Translations copy the ids and change the text.
# Text in { $braces } is filled in by the app; keep those names as they are.
# Fluent syntax: https://projectfluent.org/fluent/guide/

# The language's own name, shown in Preferences.
language-name = Deutsch

## Main window: menu bar, tabs, status bar, dialogs (src/app.rs)
theme-problem = Das Design hat einen Fehler: { $error }
theme-save-failed = Das Design konnte nicht in { $path } gespeichert werden: { $error }
open-lossy = { $path } enthält Bytes, die kein gültiges { $encoding } sind. Sie werden als � angezeigt und beim Speichern ersetzt. Sieht der Text falsch aus, wählen Sie seine Kodierung unter Kodierung > Neu öffnen mit.
open-failed = { $path } konnte nicht geöffnet werden: { $error }
dialog-open = Öffnen
dialog-save-as = Speichern unter
save-failed = { $path } konnte nicht gespeichert werden: { $error }
rename-bad-name = Der Name darf weder / noch \ enthalten.
rename-exists = „{ $name }“ ist in diesem Ordner bereits vorhanden.
rename-failed = Die Datei konnte nicht umbenannt werden: { $error }
not-saved-yet = { $name } (noch nicht gespeichert)
restore-themes-failed = Die mitgelieferten Designs konnten nicht wiederhergestellt werden: { $error }
reload-failed = { $name } konnte nicht neu geladen werden: { $error }
reopen-lossy = Einige Bytes in { $name } sind kein gültiges { $encoding }. Sie werden als � angezeigt.
command-palette-title = Befehlspalette
command-palette-hint = Teil eines Befehlsnamens eingeben
tab-list-title = Offene Tabs
tab-list-hint = Teil eines Tab-Namens eingeben
no-results = Keine Ergebnisse
selected-count = { $count } ausgewählt
marked-count = { $count } markiert
dialog-open-workspace = Ordner als Arbeitsbereich öffnen
distraction-free-notice = Ablenkungsfreier Modus. Mit F11 verlassen.
wrapped-to-top = Am Anfang fortgesetzt
wrapped-to-bottom = Am Ende fortgesetzt
not-found = Nicht gefunden
no-more-matches = Keine weiteren Treffer
replaced-count = { $count } ersetzt
search-start-failed = Die Suche konnte nicht gestartet werden: { $error }
find-in-files-title = „{ $text }“ in { $folder }
invalid-regex = Ungültiger regulärer Ausdruck: { $error }
about-ettin-edit = Über Ettin Edit
menu-file = Datei
menu-edit = Bearbeiten
menu-search = Suchen
menu-view = Ansicht
menu-encoding = Kodierung
menu-theme = Design
menu-language = Sprache
menu-settings = Einstellungen
menu-tools = Werkzeuge
menu-plugins = Plugins
menu-help = Hilfe
open-recent = Zuletzt geöffnet
no-recent-files = Keine zuletzt geöffneten Dateien
new = Neu
open-dots = Öffnen…
clear-recent-files = Liste leeren
save = Speichern
save-as = Speichern unter…
save-all = Alle speichern
reload-from-disk = Von Festplatte neu laden
rename-dots = Umbenennen…
move-to-trash = In den Papierkorb…
open-containing-folder = Enthaltenden Ordner öffnen
open-terminal-here = Terminal hier öffnen
print-dots = Drucken…
export-html = Als HTML exportieren…
open-folder-as-workspace = Ordner als Arbeitsbereich öffnen…
close-workspace = Arbeitsbereich schließen
close = Schließen
close-all = Alle schließen
close-others = Andere schließen
exit = Beenden
find-dots = Suchen…
replace-dots = Ersetzen…
find-next = Weitersuchen
find-previous = Rückwärts suchen
find-selected-next = Auswahl weitersuchen
find-selected-previous = Auswahl rückwärts suchen
find-in-files = In Dateien suchen…
find-all-in-document = Alle im aktuellen Dokument suchen
find-all-in-tabs = Alle in offenen Tabs suchen
select-all-matches = Alle Treffer auswählen
mark-all = Alle markieren
clear-all-marks = Alle Markierungen entfernen
go-to-line = Gehe zu Zeile…
menu-bookmark = Lesezeichen
toggle-bookmark = Lesezeichen umschalten
next-bookmark = Nächstes Lesezeichen
previous-bookmark = Vorheriges Lesezeichen
clear-all-bookmarks = Alle Lesezeichen entfernen
cut-bookmarked-lines = Markierte Zeilen ausschneiden
copy-bookmarked-lines = Markierte Zeilen kopieren
remove-bookmarked-lines = Markierte Zeilen löschen
remove-unbookmarked-lines = Nicht markierte Zeilen löschen
inverse-bookmarks = Lesezeichen umkehren
word-wrap = Zeilenumbruch
line-numbers = Zeilennummern
highlight-current-line = Aktuelle Zeile hervorheben
menu-show-symbol = Symbole anzeigen
show-whitespace = Leerzeichen anzeigen
show-end-of-line = Zeilenende anzeigen
show-invisible = Unsichtbare Zeichen anzeigen
show-all-characters = Alle Zeichen anzeigen
indent-guides = Einrückungslinien
long-line-marker-column = Markierung für lange Zeilen (Spalte { $column })
function-list = Funktionsliste
document-map = Dokumentübersicht
character-panel = Zeichentabelle
hex-view = Hex-Ansicht
folder-as-workspace = Ordner als Arbeitsbereich
distraction-free-mode = Ablenkungsfreier Modus
tab-list = Tab-Liste…
command-palette = Befehlspalette…
move-to-other-view = In andere Ansicht verschieben
clone-to-other-view = In andere Ansicht klonen
focus-other-view = Andere Ansicht fokussieren
menu-folding = Faltung
fold-all = Alles einklappen
unfold-all = Alles ausklappen
fold-current-level = Aktuelle Ebene einklappen
unfold-current-level = Aktuelle Ebene ausklappen
fold-level = Ebene { $level } einklappen
zoom-in = Vergrößern
zoom-out = Verkleinern
reset-zoom = Zoom zurücksetzen
summary-dots = Zusammenfassung…
themes-dark = Dunkel
themes-light = Hell
themes-custom = Eigene
edit-theme-colours = Designfarben bearbeiten…
open-themes-folder = Design-Ordner öffnen
reload-themes = Designs neu laden
restore-built-in-themes = Mitgelieferte Designs wiederherstellen
preferences-dots = Optionen…
shortcut-mapper-dots = Tastenkombinationen…
auto-close-brackets = Klammern und Anführungszeichen schließen
smart-indentation = Intelligente Einrückung
indent-with-spaces = Mit Leerzeichen einrücken
word-completion = Wortvervollständigung
keyword-completion = Schlüsselwort- und Funktionsvervollständigung
parameter-hints = Parameterhinweise
single-window-long = Ein Fenster (Dateien im laufenden Ettin Edit öffnen)
remember-unsaved = Ungespeicherte Änderungen beim Beenden merken
auto-reload = Auf Festplatte geänderte Dateien automatisch neu laden
long-line-column = Spalte für lange Zeilen
user-manual = Benutzerhandbuch
user-manual-web = Benutzerhandbuch im Web
check-for-updates = Nach Updates suchen…
check-updates-automatically = Automatisch nach Updates suchen
encoding-save-as = Speichern als
reopen-with = Neu öffnen mit
only-saved-files = Nur für Dateien, die gespeichert sind
stop-recording = Aufnahme beenden
start-recording = Aufnahme starten
playback = Abspielen
run-macro-multiple = Makro mehrfach ausführen…
save-current-macro = Aktuelles Makro speichern…
delete-saved-macro = Gespeichertes Makro löschen
new-language = Neue Sprache…
open-languages-folder = Sprachen-Ordner öffnen
open-completion-lists-folder = Ordner der Vervollständigungslisten öffnen
reload-languages = Sprachen neu laden
language-guide = Sprachen-Anleitung
all-open-tabs = Alle offenen Tabs
new-tab = Neuer Tab
unpin-tab = Tab lösen
pin-tab = Tab anheften
copy-full-path = Vollständigen Pfad kopieren
copy-file-name = Dateinamen kopieren
status-length-lines = Länge: { $length }   Zeilen: { $lines }
click-to-copy = Klicken zum Kopieren
recording-macro = Makro wird aufgenommen
read-only-status = Schreibgeschützt
column-mode-status = Spaltenmodus
column-mode-status-hover = Ziehen und Umschalt+Pfeiltasten wählen ein Rechteck aus. Klicken zum Ausschalten.
click-to-allow-editing = Klicken, um Bearbeiten zu erlauben
status-position = Z { $line }, Sp { $column }
status-selection = Ausw. { $count }
status-carets = { $count } Cursor
go-to-line-hover = Gehe zu Zeile (Ctrl+G)
save-changes-title = Änderungen speichern?
save-changes-question = „{ $name }“ hat ungespeicherte Änderungen. Vor dem Schließen speichern?
cancel = Abbrechen
dont-save = Nicht speichern
trash-failed = { $name } konnte nicht in den Papierkorb verschoben werden: { $error }
about-version = Version { $version }
about-tagline = Ein schneller, moderner Texteditor für Linux und Windows.
about-built-with = Geschrieben in Rust mit egui und syntect.
error-title = Etwas ist schiefgelaufen
ok = OK

## Documents (src/document.rs)
eol-unix = Unix (LF)
eol-windows = Windows (CR LF)
eol-mac = Macintosh (CR)
cant-encode-char = „{ $char }“ (U+{ $code }) kann nicht als { $encoding } gespeichert werden. Wählen Sie im Menü Kodierung eine Kodierung, die es enthält, etwa UTF-8.
untitled-name = neu { $number }

## Edit and Tools menus (src/app/menus.rs)
duplicate = Duplizieren
delete-lines = Zeilen löschen
move-lines-up = Nach oben verschieben
move-lines-down = Nach unten verschieben
join-lines = Zeilen verbinden
split-lines = Zeilen an Spalte für lange Zeilen teilen
insert-blank-above = Leerzeile darüber einfügen
insert-blank-below = Leerzeile darunter einfügen
sort-lexical-asc = Lexikografisch aufsteigend sortieren
sort-lexical-desc = Lexikografisch absteigend sortieren
sort-ignore-case-asc = Ohne Groß-/Kleinschreibung aufsteigend sortieren
sort-ignore-case-desc = Ohne Groß-/Kleinschreibung absteigend sortieren
sort-integer-asc = Als Ganzzahlen aufsteigend sortieren
sort-integer-desc = Als Ganzzahlen absteigend sortieren
sort-decimal-asc = Als Dezimalzahlen aufsteigend sortieren
sort-decimal-desc = Als Dezimalzahlen absteigend sortieren
sort-length-asc = Nach Länge aufsteigend sortieren
sort-length-desc = Nach Länge absteigend sortieren
reverse-lines = Zeilenreihenfolge umkehren
remove-duplicate-lines = Doppelte Zeilen entfernen
remove-consecutive-duplicates = Aufeinanderfolgende doppelte Zeilen entfernen
remove-empty-lines = Leere Zeilen entfernen
remove-blank-lines = Leere Zeilen entfernen (auch mit Leerraum)
undo = Rückgängig
redo = Wiederholen
cut = Ausschneiden
copy = Kopieren
paste = Einfügen
select-all = Alles auswählen
menu-line-operations = Zeilenoperationen
menu-convert-case = Umwandeln in
case-upper = GROSSBUCHSTABEN
case-lower = kleinbuchstaben
case-title = Wörter Groß Schreiben
case-sentence = Satzanfang groß
case-invert = gROSS/kLEIN uMKEHREN
menu-comment = Kommentar
toggle-line-comment = Zeilenkommentar umschalten
toggle-block-comment = Blockkommentar umschalten
trim-trailing = Leerraum am Zeilenende entfernen
trim-leading = Leerraum am Zeilenanfang entfernen
trim-both = Leerraum an Anfang und Ende entfernen
eol-to-space = Zeilenende zu Leerzeichen
tabs-to-spaces = Tabs zu Leerzeichen
leading-spaces-to-tabs = Führende Leerzeichen zu Tabs
all-spaces-to-tabs = Alle Leerzeichen zu Tabs
menu-insert = Einfügen
date-time-short = Datum und Uhrzeit (kurz)
date-time-long = Datum und Uhrzeit (lang)
menu-multi-select = Mehrfachauswahl
select-next-occurrence = Nächstes Vorkommen auswählen
select-all-occurrences = Alle Vorkommen auswählen
menu-auto-completion = Autovervollständigung
complete-word = Wort vervollständigen
show-parameter-hint = Parameterhinweis anzeigen
column-editor = Spalteneditor…
read-only = Schreibgeschützt
column-mode = Spaltenmodus
mime-base64-encode = Base64-kodieren
mime-base64-encode-wrapped = Base64-kodieren mit Zeilenumbrüchen
mime-base64-url-encode = Base64-kodieren, URL-sicher
mime-base64-decode = Base64-dekodieren
mime-url-encode-component = URL-kodieren (Teil einer URL)
mime-url-encode = URL-kodieren (ganze URL)
mime-url-decode = URL-dekodieren
mime-qp-encode = Quoted-Printable-kodieren
mime-qp-decode = Quoted-Printable-dekodieren
mime-html-encode = HTML-kodieren
mime-html-encode-all = HTML-kodieren, auch Akzente und Symbole
mime-html-decode = HTML-dekodieren
mime-saml-decode = SAML-dekodieren
json-format = Formatieren
json-minify = Minimieren
json-validate = JSON validieren
json-sort-keys = JSON-Schlüssel sortieren
json-tree = JSON-Baum
xml-pretty-print = XML formatieren
xml-pretty-print-attributes = XML formatieren, Attribute in neuen Zeilen
xml-linearize = Linearisieren
xml-check-syntax = Syntax prüfen
xml-check-on-save = Syntax beim Speichern prüfen
xml-validate = Validieren
xml-validate-against = Validieren gegen…
xml-xsl-transform = XSL-Transformation…
xpath-panel = XPath-Bereich
copy-current-xpath = Aktuellen XPath kopieren
xml-escape = Sonderzeichen maskieren
xml-unescape = Sonderzeichen demaskieren
close-tags-automatically = Tags automatisch schließen
csvquery-panel = CsvQuery-Bereich
run-query = Abfrage ausführen
copy-results = Ergebnisse kopieren
results-to-new-tab = Ergebnisse in neuen Tab
menu-compare = Vergleichen
menu-csvquery = CsvQuery
menu-mime-tools = MIME-Werkzeuge
compare-two-views = Beide Ansichten vergleichen
compare-previous-tab = Mit vorherigem Tab vergleichen
compare-with = Vergleichen mit
compare-to-saved = Mit letzter Speicherung vergleichen
compare-to-clipboard = Mit Zwischenablage vergleichen
compare-to-git = Mit letztem Commit vergleichen (Git)
compare-selections = Auswahlen vergleichen
next-difference = Nächster Unterschied
previous-difference = Vorheriger Unterschied
show-comparison = Vergleich anzeigen
export-as-patch = Als Patch exportieren
close-comparison = Vergleich schließen
compare-ignore-whitespace = Leerzeichen und Tabs ignorieren
compare-ignore-case = Groß-/Kleinschreibung ignorieren
compare-ignore-empty-lines = Leere Zeilen ignorieren
compare-detect-moves = Verschobene Zeilen erkennen

## Command names in the command palette (src/app/commands.rs)
long-line-marker = Markierung für lange Zeilen
next-tab = Nächster Tab
previous-tab = Vorheriger Tab
single-window = Ein Fenster
start-or-stop-recording = Aufnahme starten oder beenden
manage-plugins = Plugins verwalten…
plugin-log = Plugin-Protokoll
open-plugins-folder = Plugin-Ordner öffnen
reload-plugins = Plugins neu laden
install-sample-plugins = Beispiel-Plugins installieren
plugin-guide = Plugin-Anleitung

## Plugins menu and plugin notices (src/app/plugins.rs)
install-plugin-from-file = Plugin aus Datei installieren
plugin-zip-filter = Plugin (.zip)
install-plugin-from-folder = Plugin aus Ordner installieren
no-plugins-folder = Es gibt keinen Konfigurationsordner für Plugins.
plugin-removed = { $name } entfernt; sein Ordner liegt im Papierkorb.
plugin-updated = { $name } aktualisiert.
plugin-updated-version = { $name } auf { $version } aktualisiert.
plugin-installed = { $name } installiert.
plugin-installed-version = { $name } { $version } installiert.
plugins-running = { $count ->
    [one] { $count } Plugin läuft
   *[other] { $count } Plugins laufen
}
sample-install-failed = Das Beispiel-Plugin { $name } konnte nicht in { $folder } installiert werden: { $error }
samples-already-installed = Die Beispiel-Plugins sind bereits im Plugin-Ordner.
plugin-enabled = Aktiviert
plugin-adds = Fügt hinzu:
plugin-search-installed = Installierte Plugins durchsuchen
plugins-only-samples = Bisher sind nur die Beispiel-Plugins aufgeführt. Selbst ein Plugin geschrieben?
plugins-submit-link = Im Verzeichnis einreichen
samples-installed = { $count ->
    [one] 1 Plugin installiert.
   *[other] { $count } Plugins installiert.
} Sie finden sie im Menü Plugins.
no-plugin-commands = Keine Plugin-Befehle

## Text tools (src/app/text_tools.rs)
tool-error-at = { $tool }: Zeile { $line }, Spalte { $column }: { $message }
tool-error = { $tool }: { $message }
tool-read-only = Das Dokument ist schreibgeschützt.
tool-ok-selection = { $result } (Auswahl)
json-handled-as-js = Dies ist kein striktes JSON und wurde daher als JavaScript behandelt. JSON validieren zeigt die Abweichungen.
json-valid = Gültiges JSON
xml-well-formed = Das XML ist wohlgeformt
copied-text = { $text } kopiert
xpath-no-element = Der Cursor steht in keinem Element, oder ein End-Tag passt nicht zu seinem Start-Tag.
xpath-not-xml = Dieses Dokument ist kein XML.
copy-xpath = XPath kopieren
saved-not-well-formed = Gespeichert, aber { $name } ist nicht wohlgeformt: Zeile { $line }, Spalte { $column }: { $message }
xpath-type-expression = Geben Sie einen XPath-Ausdruck ein, etwa //item[@id='1'].
xpath-not-well-formed = Das Dokument ist kein wohlgeformtes XML (Zeile { $line }, Spalte { $column }: { $message })
xpath-error-at = { $message } (bei Zeichen { $position })

## Compare (src/app/compare.rs)
compare-selection-title = { $name } (Auswahl ab Zeile { $line })
compare-same = Beide Seiten sind gleich.
compare-need-second-tab = Öffnen Sie einen zweiten Tab zum Vergleichen, oder wählen Sie einen unter Werkzeuge > Vergleichen > Vergleichen mit.
compare-not-saved = Dieser Tab wurde noch nicht gespeichert.
compare-saved-title = { $name } (gespeichert)
compare-read-saved-failed = Die gespeicherte Datei konnte nicht gelesen werden: { $error }
compare-commit-title = { $name } (letzter Commit)
compare-need-selections = Wählen Sie zwei Textstellen aus: Ctrl+Ziehen für eine zweite Auswahl, oder in jeder Ansicht eine.
clipboard = Zwischenablage
compare-clipboard-empty = Die Zwischenablage enthält keinen Text zum Vergleichen.
compare-closed-title = { $name } (geschlossen)
compare-tab-title = Vergleich: { $left } | { $right }

## Languages folder (src/app/languages.rs)
make-folder-failed = { $path } konnte nicht erstellt werden: { $error }
write-failed = { $path } konnte nicht geschrieben werden: { $error }

## Printing (src/app/printing.rs)
dialog-export-html = Als HTML exportieren

## XML validation and XSL (src/app/xml_schema.rs)
dialog-validate-against = Gegen Schema validieren
filter-schema = XML-Schema oder DTD
filter-all-files = Alle Dateien
the-doctype = den DOCTYPE
read-failed = { $path } konnte nicht gelesen werden: { $error }
validate-against = Gegen { $schema } validieren
valid-against = Das Dokument ist gültig gegen { $schema }
xsl-transform = XSL-Transformation
filter-xsl = XSL-Stylesheet

## Preferences (src/prefs_ui.rs)
prefs-editing = Bearbeiten
prefs-new-documents = Neue Dokumente
prefs-general = Dateien und Start
preferences = Optionen
theme-colours-dots = Designfarben…
prefs-font = Schrift
built-in-font = Mitgeliefert (Hack)
editor-font-label = Editor-Schrift:
use-built-in = Mitgelieferte verwenden
font-size-label = Größe:
font-size-hint = Ctrl+0 stellt nach dem Zoomen diese Größe wieder her
font-find-label = Suchen:
font-name-hint = Schriftname
show-proportional-fonts = Proportionalschriften anzeigen
looking-for-fonts = Installierte Schriften werden gesucht…
no-fonts-match = Keine passenden Schriften
proportional = proportional
prefs-show = Anzeigen
prefs-word-wrap = Zeilenumbruch
prefs-line-numbers = Zeilennummern
prefs-highlight-line = Aktuelle Zeile hervorheben
prefs-whitespace = Leerraum-Symbole
prefs-eol = Zeilenende-Symbole
prefs-invisible = Unsichtbare Zeichen (Steuerzeichen, besondere Leerzeichen, Zeichen ohne Breite)
prefs-indent-guides = Einrückungslinien
prefs-edge = Markierung für lange Zeilen bei Spalte
prefs-indentation = Einrückung
prefs-tab-width = Tab-Breite:
prefs-columns = Spalten
prefs-indent-tabs = Mit Tabs einrücken
prefs-indent-spaces = Mit Leerzeichen einrücken
prefs-smart-indent = Intelligente Einrückung (Einrückung beibehalten und nach öffnender Klammer einrücken)
prefs-typing = Eingabe
prefs-auto-close = Klammern und Anführungszeichen automatisch schließen
prefs-close-tags = HTML- und XML-Tags automatisch schließen
prefs-word-completion = Beim Tippen Wörter aus dem Dokument vorschlagen
prefs-language-completion = Schlüsselwörter und Funktionen der Sprache vorschlagen
prefs-parameter-hints = Beim Tippen eines Aufrufs die Parameter der Funktion anzeigen
prefs-line-ending = Zeilenende
prefs-system-line-ending = Standard dieses Systems ({ $ending })
prefs-startup = Start und Beenden
prefs-restore-session = Zuletzt geöffnete Dateien wieder öffnen
prefs-remember-unsaved = Ohne Rückfrage zu ungespeicherten Änderungen beenden und sie beim nächsten Mal wiederherstellen
prefs-single-instance = Dateien im laufenden Ettin-Edit-Fenster öffnen
prefs-files = Dateien
prefs-auto-reload = Auf Festplatte geänderte Dateien ohne ungespeicherte Änderungen automatisch neu laden
prefs-xml-check = XML bei jedem Speichern auf Wohlgeformtheit prüfen
prefs-printing = Drucken
prefs-print-line-numbers = Zeilennummern drucken
prefs-print-colours = Syntaxfarben drucken (abgedunkelt für gute Lesbarkeit auf Papier)
prefs-updates = Updates
prefs-check-updates = Einmal pro Woche nach einer neuen Version von Ettin Edit suchen
prefs-plugin-programs = Programme, die Plugins ausführen
prefs-plugin-path-hint = Leer lassen, um sie im PATH zu suchen.
prefs-plugin-reload-hint = Änderungen gelten, wenn Plugins neu geladen werden.
prefs-interface-language = Sprache der Oberfläche
prefs-system-language = Wie das System ({ $language })
open-translations-folder = Übersetzungsordner öffnen
reload-translations = Neu laden
prefs-translations-hint = Fügen Sie eine Sprache hinzu oder ändern Sie eine Übersetzung, indem Sie eine .ftl-Datei in den Übersetzungsordner legen. Siehe „Translations“ im Benutzerhandbuch.

## Translations (src/i18n.rs)
no-config-folder = Es gibt keinen Konfigurationsordner für Übersetzungen.

## Find bar, results, Go to Line, Find in Files (src/search_ui.rs)
toggle-replace = Ersetzen umschalten
find-hint = Suchen
match-case = Groß-/Kleinschreibung
whole-word = Ganzes Wort
regular-expression = Regulärer Ausdruck
extended-search = Erweitert: \n, \r, \t, \0, \xNN und \uNNNN stehen für diese Zeichen
extended-search-short = Erweitert (\n, \t, \xNN)
match-of-total = { $current } von { $total }
matches-found = { $total } gefunden
find-status = { $count }, { $message }
previous-match = Vorheriger Treffer (Shift+F3)
next-match = Nächster Treffer (F3)
wrap-around = Am Ende fortsetzen
mark-all-bookmark = Alle markieren und Zeilen mit Lesezeichen versehen
close-esc = Schließen (Esc)
replace-hint-regex = Ersetzen  ($1 oder \1 für Gruppen)
replace-hint-extended = Ersetzen  (\n, \t, \xNN erlaubt)
replace = Ersetzen
replace-hover = Diesen Treffer ersetzen und den nächsten suchen
replace-all = Alle ersetzen
in-selection = In Auswahl
search-stopped = abgebrochen
searched-files = { $count ->
    [one] { $count } Datei durchsucht
   *[other] { $count } Dateien durchsucht
}
results-hits = { $count ->
    [one] { $count }{ $plus } Treffer
   *[other] { $count }{ $plus } Treffer
}
results-in-files = { $count ->
    [one] { $found } in { $count } Datei
   *[other] { $found } in { $count } Dateien
}
results-in-documents = { $count ->
    [one] { $found } in { $count } Dokument
   *[other] { $found } in { $count } Dokumenten
}
stop = Stopp
close-results = Ergebnisse schließen
collapse-all = Alle einklappen
expand-all = Alle ausklappen
go-to-line-title = Gehe zu Zeile
go-to-line-prompt = Zeilennummer (1–{ $lines }) oder Zeile:Spalte
enter-line-number = Zeilennummer eingeben
go = Los
find-in-files-heading = In Dateien suchen
find-what = Suchen nach
folder = Ordner
browse = Durchsuchen…
search-in-folder = Im Ordner suchen
file-filter-hint = *.rs *.toml   (leer für alle Dateien, !Name zum Auslassen)
file-filter-hover = Dateinamensmuster, getrennt durch Leerzeichen oder Semikolons. Mit ! am Anfang werden diese Dateien ausgelassen.
include-subfolders = Unterordner einbeziehen
include-hidden = Versteckte Dateien und Ordner einbeziehen
find-all = Alle suchen
enter-search-text = Geben Sie einen Suchbegriff ein.
folder-missing = Dieser Ordner existiert nicht.
results-problems = { $count ->
    [one] { $count }{ $plus } Problem
   *[other] { $count }{ $plus } Probleme
}

## Rename, Move to Trash, Summary, file changed on disk (src/file_ui.rs)
rename-file-title = Datei umbenennen
new-name = Neuer Name
rename = Umbenennen
move-to-trash-title = In den Papierkorb
move-to-trash-question = { $windows ->
    [yes] „{ $name }“ in den Papierkorb verschieben und den Tab schließen? Sie können die Datei aus dem Papierkorb wiederherstellen.
   *[no] „{ $name }“ in den Papierkorb verschieben und den Tab schließen? Sie können die Datei aus dem Papierkorb wiederherstellen.
}
summary-title = Zusammenfassung
summary-file = Datei
summary-created = Erstellt
summary-modified = Geändert
summary-characters = Zeichen (ohne Zeilenenden)
summary-words = Wörter
summary-lines = Zeilen
summary-size = Größe beim Speichern
summary-bytes = { $count ->
    [one] { $bytes } Byte
   *[other] { $bytes } Bytes
}
summary-cant-encode = kann in dieser Kodierung nicht gespeichert werden
summary-selection = Auswahl
summary-selection-size = { $line_count ->
    [one] { $characters } Zeichen in { $lines } Zeile
   *[other] { $characters } Zeichen in { $lines } Zeilen
}
disk-changed = Diese Datei wurde von einem anderen Programm geändert.
disk-changed-unsaved = Diese Datei wurde von einem anderen Programm geändert, und Sie haben hier ungespeicherte Änderungen. Neu laden ersetzt sie (Rückgängig holt sie zurück).
disk-deleted = Diese Datei wurde von einem anderen Programm gelöscht oder verschoben.
reload = Neu laden
keep-my-version = Meine Version behalten
keep-open = Geöffnet lassen

## Folder panel (src/folder_ui.rs)
no-folder-open = Kein Ordner geöffnet.
close-folder-panel = Ordnerbereich schließen
refresh-folder = Ordner neu einlesen
collapse-folders = Alle Ordner einklappen
show-hidden-files = Versteckte Dateien anzeigen
open = Öffnen
find-in-files-here = Hier in Dateien suchen…
copy-path = Pfad kopieren
folder-truncated = Nur die ersten { $count } Einträge werden angezeigt

## Function list (src/function_list_ui.rs)
show-caret-symbol = Position des Cursors anzeigen
filter-hint = Filter
reading-document = Dokument wird gelesen…
nothing-found-in-document = In diesem Dokument nichts gefunden.
line-number = Zeile { $line }

## Command palette (src/palette_ui.rs)
nothing-matches = Keine Übereinstimmung

## Shortcut mapper (src/shortcut_ui.rs)
shortcut-reserved-clipboard = Ctrl+C, Ctrl+X und Ctrl+V sind für Kopieren, Ausschneiden und Einfügen reserviert
shortcut-mapper = Tastenkombinationen
command-or-shortcut = Befehl oder Tastenkombination
only-assigned = Nur Befehle mit Tastenkombination
shortcut-in-use = { $shortcut } wird bereits von „{ $command }“ verwendet.
move-it-here = Hierher verschieben
select-command = Wählen Sie einen Befehl, um seine Tastenkombination zu ändern.
shortcut-fixed = „{ $command }“ gehört zum Textfeld und kann nicht geändert werden.
press-shortcut = Neue Tastenkombination drücken… (Esc bricht ab)
change-shortcut = Ändern…
set-shortcut = Festlegen…
add-another-shortcut = Weitere hinzufügen…
remove = Entfernen
reset-to-default = Auf Standard zurücksetzen
reset-all-shortcuts = Alle Tastenkombinationen zurücksetzen
shortcut-rule = Tastenkombinationen brauchen Ctrl oder Alt, außer bei Funktionstasten (F1–F12).
command = Befehl
shortcut = Tastenkombination

## Theme colours (src/style_ui.rs)
style-background = Hintergrund
style-text = Text
style-caret = Cursor
style-selection = Auswahl
style-current-line = Aktuelle Zeile
style-line-numbers = Zeilennummern
style-indent-guides = Einrückungslinien
style-whitespace = Leerraum-Symbole
style-search-matches = Suchtreffer
style-accent = Akzent (Tabs, Schaltflächen, Faltung)
unset-colour = Diese Farbe nicht festlegen
style-bold = Fett
style-italic = Kursiv
style-underline = Unterstrichen
set-colour = Festlegen
choose-colour = Farbe wählen
theme-read-failed = Die Design-Datei konnte nicht gelesen werden: { $error }
theme-colours = Designfarben
theme-label = Design:
save-as-new-theme = Als neues Design speichern…
new-theme-name = Name des neuen Designs:
new-theme-name-hint = Name für das neue Design
theme-name-taken = Ein Design mit diesem Namen gibt es schon
theme-colours-hint = Änderungen wirken sofort und werden gleich gespeichert. „Änderungen rückgängig“ stellt das Design so wieder her, wie es beim Öffnen des Fensters war.
theme-changes-count = { $count ->
    [one] 1 Einstellung seit dem Öffnen geändert.
   *[other] { $count } Einstellungen seit dem Öffnen geändert.
}
style-editor = Editor
style-syntax = Syntax
rule-filter-hint = Regelname oder Scope, etwa string oder keyword
rules-hint = Jede Regel färbt den Text, dessen Scope passt. Mehrere Scopes durch Kommas trennen.
rule-name = Name
rule-scope = Scope
rule-style = Stil
no-name = (kein Name)
delete-rule = Diese Regel löschen
add-rule = Regel hinzufügen
new-rule = Neue Regel

## Notices, JSON tree, XPath panel (src/text_tools_ui.rs)
go-to = Gehe zu
dismiss = Ausblenden
json-too-large = Dieses Dokument ist zu groß für die Baumansicht.
close-json-tree = JSON-Baum schließen
json-filter-hint = Schlüssel und Werte filtern
json-not-valid = Kein gültiges JSON
line-column-message = Zeile { $line }, Spalte { $column }: { $message }
go-to-error = Zum Fehler
no-matches = Keine Treffer
first-matches = Erste { $count } Treffer
copy-key = Schlüssel kopieren
copy-value = Wert kopieren
expand-all-inside = Alles darin ausklappen
close-xpath-panel = XPath-Bereich schließen
evaluate = Auswerten
recent = Zuletzt
recent-expressions = Letzte Ausdrücke
result-label = Ergebnis:
xpath-nodes = { $count ->
    [one] { $count } Knoten
   *[other] { $count } Knoten
}
xpath-nodes-stale = { $count ->
    [one] { $count } Knoten (das Dokument hat sich seitdem geändert; erneut auswerten)
   *[other] { $count } Knoten (das Dokument hat sich seitdem geändert; erneut auswerten)
}

## Column editor and macros (src/tools_ui.rs)
column-editor-title = Spalteneditor
column-editor-hint = Fügt an jedem Cursor ein, oder in der Spalte des Cursors in jeder Zeile darunter.
hex-offset = Position 0x{ $hex } ({ $dec })
hex-selected = { $count } Bytes ausgewählt
hex-size = { $count } Bytes insgesamt
hex-read-only = Dieser Tab ist schreibgeschützt.
hex-cant-encode = „{ $char }“ lässt sich nicht in { $encoding } schreiben.
hex-paste-hint = Hexziffern einfügen, z. B. 48 65 6C.
hex-now-binary = Kein gültiger { $encoding }-Text mehr, daher wird dieser Tab als Bytes bearbeitet. Rückgängig macht ihn wieder zu Text.
hex-text-again = Wieder gültiger Text: Der Text wird wieder bearbeitet.
hex-binary = Als Bytes bearbeitet
hex-not-text = Diese Bytes sind kein gültiger { $encoding }-Text, daher bleibt der Tab in der Hex-Ansicht.
hex-too-big-for-text = Diese Datei ist zu groß, um sie als Text zu zeigen, daher bleibt sie in der Hex-Ansicht.
hex-copy-too-big = Zum Kopieren höchstens 16 MB auswählen.
char-search-hint = Suchen: é, U+00E9, 233 oder ein Name
char-panel-hint = Doppelklick fügt ein Zeichen ein. Rechtsklick zum Kopieren.
char-insert-character = Zeichen einfügen
char-insert-html = HTML-Entität einfügen
char-insert-hex = Code einfügen (U+…)
char-insert-decimal = Dezimalcode einfügen
char-col-char = Zeichen
char-col-dec = Dez
char-col-hex = Hex
char-col-html = HTML
char-col-name = Name
char-copy-character = Zeichen kopieren
char-copy-html = HTML-Entität kopieren
char-copy-code = Code kopieren (U+…)
text-to-insert = Einzufügender Text
numbers-to-insert = Einzufügende Zahlen
initial-number = Startwert
increase-by = Erhöhen um
repeat = Wiederholen
leading-zeros = Führende Nullen
format-dec = Dez
format-hex = Hex
format-oct = Okt
format-bin = Bin
insert = Einfügen
must-be-whole-number = { $field } muss eine ganze Zahl sein
run-macro-title = Makro mehrfach ausführen
run = Ausführen
times = Mal
run-to-end = Bis zum Dateiende ausführen
record-macro-first = Nehmen Sie zuerst ein Makro auf.
save-macro-title = Aktuelles Makro speichern

## Comparison view (src/compare_ui.rs)
no-differences = Keine Unterschiede
lines-changed = { $count } geändert
lines-added = { $count } hinzugefügt
lines-removed = { $count } entfernt
lines-moved = { $count } verschoben
compare-lines-summary = Zeilen: { $parts }
first = Erster
first-difference = Erster Unterschied
previous-difference-hover = Vorheriger Unterschied (Shift+F7)
next-difference-hover = Nächster Unterschied (F7)
last = Letzter
last-difference = Letzter Unterschied
identical = Identisch
difference-of = Unterschied { $current } von { $total }
differences-count = { $count ->
    [one] { $count } Unterschied
   *[other] { $count } Unterschiede
}
compare-approximate = (ungefähr: die Dateien sind sehr verschieden)
close-comparison-hover = Vergleich schließen
export-patch = Patch exportieren
export-patch-hover = Die Unterschiede als Unified Diff in einem neuen Tab öffnen
swap-sides = Seiten tauschen
options = Optionen
line-count = { $count ->
    [one] { $count } Zeile
   *[other] { $count } Zeilen
}

## CSV column types and delimiters (src/tools/csv.rs)
column-type-text = Text
column-type-integer = Ganzzahl
column-type-decimal = Dezimalzahl
delimiter-comma = Komma
delimiter-semicolon = Semikolon
delimiter-tab = Tab
delimiter-pipe = Senkrechter Strich
delimiter-space = Leerzeichen

## CsvQuery panel (src/csv_query.rs)
query-engine-stopped = Die Abfrage-Engine wurde beendet. Bitte erneut versuchen.
csv-shape = { $rows } Zeilen × { $columns } Spalten
csv-this-hover = Der aktuelle Tab ist die Tabelle `this`; andere Tabs sind Tabellen, benannt nach ihren Dateien.
csv-open-file = Öffnen Sie eine CSV- oder TSV-Datei, um sie abzufragen.
close-csvquery = CsvQuery schließen
tables = Tabellen
click-to-insert-name = Klicken, um einen Namen einzufügen
no-queries-yet = Noch keine Abfragen
run-query-hover = Abfrage ausführen (Ctrl+Enter)
results = Ergebnisse
columns = Spalten
csv-tables-list = Tabellen: this, { $names }
csv-rows-changed = { $count ->
    [one] { $count } Zeile in der Abfragekopie geändert (die Datei ist unverändert)
   *[other] { $count } Zeilen in der Abfragekopie geändert (die Datei ist unverändert)
}
csv-first-rows = Erste { $rows } Zeilen · { $ms } ms
csv-rows = { $count ->
    [one] { $rows } Zeile · { $ms } ms
   *[other] { $rows } Zeilen · { $ms } ms
}
open-in-new-tab = In neuem Tab öffnen
open-in-new-tab-hover = Alle Ergebniszeilen als CSV in einen neuen Tab schreiben
copy-results-hover = Ergebnisse kopieren, tabulatorgetrennt zum Einfügen in eine Tabellenkalkulation
click-to-sort = Klicken zum Sortieren
copy-row = Zeile kopieren
csv-run-for-columns = Führen Sie eine Abfrage aus, um die Spalten dieses Tabs zu sehen.
csv-column = Spalte
csv-type = Typ
csv-filled = Gefüllt
csv-empty = Leer
csv-distinct = Eindeutig
csv-min = Min
csv-max = Max
csv-average = Mittelwert
csv-type-hover = Wie die Spalte gespeichert und verglichen wird. Auto wählt den engsten Typ, der zu allen Werten passt.
auto-with = Auto ({ $detected })
delimiter = Trennzeichen
auto-quotes = Auto-Anführungszeichen
quotes-char = Anführungszeichen { $char }
no-quotes = Keine Anführungszeichen
quote-character = Anführungszeichen
header-row = Kopfzeile
header-row-hover = Die erste Zeile benennt die Spalten
reset = Zurücksetzen
reset-format-hover = Zum erkannten Format zurückkehren

## Update check (src/update.rs)
update-bad-json = Die Release-Liste ergab keinen Sinn: { $error }
update-no-version = Die Release-Liste enthält keine Version
update-not-found = Kein veröffentlichtes Release gefunden (HTTP 404). Die Update-Suche funktioniert nur, solange das Ettin-Edit-Repository öffentlich ist.
update-http-error = GitHub antwortete mit HTTP { $code }.
update-unreachable = GitHub nicht erreichbar: { $error }
update-read-failed = Die Antwort von GitHub konnte nicht gelesen werden: { $error }
check-for-updates-title = Nach Updates suchen
update-asking = GitHub wird nach dem neuesten Release gefragt…
update-available = Ettin Edit { $version } ist verfügbar.
update-current = Sie haben Version { $version }.
open-download-page = Download-Seite öffnen
dont-check-automatically = Nicht automatisch suchen
update-latest = Sie haben die neueste Version, { $version }.
update-newer-local = Sie haben Version { $version }; das neueste Release ist { $latest }.
update-failed = Die Suche nach Updates ist fehlgeschlagen.
update-now = Jetzt aktualisieren
update-release-notes = Versionshinweise
later = Später
restart-now = Jetzt neu starten
install-and-restart = Installieren und neu starten
self-update-how-arch = Lädt das Arch-Paket herunter und installiert es mit pacman. Das System fragt nach Ihrem Passwort.
self-update-how-deb = Lädt das Debian-Paket herunter und installiert es mit apt. Das System fragt nach Ihrem Passwort.
self-update-how-portable = Lädt die neue Version herunter und ersetzt die Programmdatei dieser Kopie.
self-update-how-setup = Lädt das Installationsprogramm herunter. Ettin Edit wird dann geschlossen, installiert das Update und startet neu.
self-update-how-rpm = Lädt das RPM-Paket herunter und installiert es mit dnf (oder zypper). Das System fragt nach Ihrem Passwort.
self-update-how-appimage = Lädt das neue AppImage herunter und ersetzt dieses damit.
self-update-flatpak = Diese Kopie ist ein Flatpak: Ihre Softwareverwaltung oder der Befehl flatpak update installiert neue Versionen.
self-update-managed = Diese Kopie stammt aus { $store }, worüber neue Versionen installiert werden.
self-update-no-exe = Es ist unklar, wo Ettin Edit installiert ist, daher kann es sich nicht selbst aktualisieren.
self-update-no-build = Für diesen Computer gibt es keinen fertigen Build, daher kann sich Ettin Edit nicht selbst aktualisieren.
self-update-source-build = Diese Kopie wurde aus dem Quellcode gebaut; aktualisieren Sie sie mit git und cargo.
self-update-read-only = Ettin Edit kann nicht in { $path } schreiben und sich dort daher nicht selbst aktualisieren.
self-update-no-file = Diese Version hat keine Datei für diese Installationsart; laden Sie sie von der Release-Seite herunter.
self-update-no-checksum = Für { $file } ist keine Prüfsumme angegeben, daher kann die Datei nicht geprüft werden. Laden Sie sie stattdessen von der Release-Seite herunter.
self-update-save-failed = Der Download konnte nicht gespeichert werden: { $error }
self-update-download-failed = Das Update konnte nicht heruntergeladen werden: { $error }
self-update-cancelled = Der Download wurde abgebrochen.
self-update-checksum-mismatch = { $file } passt nicht zur Prüfsumme und wurde nicht installiert.
self-update-install-failed = Die Installation ist fehlgeschlagen: { $error }
self-update-not-authorised = Die Installation braucht ein Administratorpasswort, und es wurde keins eingegeben.
self-update-unpack-failed = Das Update konnte nicht entpackt werden: { $error }
self-update-missing-file = { $file } ist nicht im Download enthalten
self-update-replace-failed = Die Programmdatei konnte nicht ersetzt werden: { $error }
self-update-downloading = { $done } von { $total } MB heruntergeladen
self-update-installing = Wird installiert…
self-update-password = Das System fragt nach einem Administratorpasswort.
self-update-installed = Ettin Edit { $version } ist installiert. Starten Sie neu, um es zu verwenden.
self-update-ready = Ettin Edit { $version } ist heruntergeladen. Ettin Edit wird geschlossen, installiert es und startet neu. Mit „Später“ wird es beim nächsten Schließen von Ettin Edit installiert.
self-update-run-in-terminal = Führen Sie dies in einem Terminal aus, um das Update abzuschließen:
self-update-failed = Ettin Edit konnte nicht aktualisiert werden.

## User manual window (src/manual.rs)
back = Zurück
forward = Vor
search-manual = Handbuch durchsuchen
clear-search = Suche leeren
open-on-web = Im Web öffnen
web-manual-failed = Das Web-Handbuch konnte nicht geöffnet werden: { $error }
open-url-failed = { $url } konnte nicht geöffnet werden: { $error }
copy-to-clipboard = In die Zwischenablage kopieren

## Encoding groups (src/encoding.rs)
enc-western-european = Westeuropäisch
enc-central-european = Mitteleuropäisch
enc-cyrillic = Kyrillisch
enc-greek = Griechisch
enc-turkish = Türkisch
enc-baltic = Baltisch
enc-nordic = Nordisch
enc-hebrew = Hebräisch
enc-arabic = Arabisch
enc-vietnamese = Vietnamesisch
enc-thai = Thailändisch
enc-romanian = Rumänisch
enc-chinese = Chinesisch
enc-japanese = Japanisch
enc-korean = Koreanisch

## Files, folders, terminal and Git (src/system.rs)
explorer-failed = Der Explorer konnte nicht geöffnet werden: { $error }
no-parent-folder = { $path } hat keinen Ordner
terminal-failed = Es konnte kein Terminal geöffnet werden: { $error }
no-terminal = Kein Terminal gefunden. Setzen Sie die Umgebungsvariable TERMINAL auf den Befehl Ihres Terminals.
git-missing = Git ist nicht installiert (oder nicht im PATH).
git-failed = git konnte nicht ausgeführt werden: { $error }
not-in-git = { $path } liegt in keinem Git-Repository.
not-in-repository = { $path } liegt nicht im Repository unter { $root }.
git-not-committed = Diese Datei ist noch nicht im letzten Commit.
git-no-commits = Das Repository hat noch keine Commits.

## Themes (src/theme.rs)
themes-write-failed = Die mitgelieferten Designs konnten nicht nach { $path } geschrieben werden: { $error }
theme-file-problem = Die Design-Datei { $path } hat einen Fehler, daher wird das ursprüngliche Design { $name } verwendet: { $error }
theme-skipped = Design { $path } übersprungen: { $error }

## Fonts (src/fonts.rs)
font-unusable = { $path } ist keine Schrift, die Ettin Edit verwenden kann.
font-read-failed = Die Editor-Schrift { $path } konnte nicht gelesen werden: { $error }

## Editor (src/editor/view.rs)
completion-keyword = Schlüsselwort
completion-function = Funktion

## Manage Plugins and plugin log (src/plugins/ui.rs)
plugins-warning = Plugins laufen als eigene Programme mit Ihrem Zugriff auf Dateien und Netzwerk. Führen Sie nur Plugins aus, denen Sie vertrauen.
plugins-installed-tab = Installiert ({ $count })
plugins-available-count = Verfügbar ({ $count })
plugins-available = Verfügbar
install-from-folder = Aus Ordner installieren…
install-from-folder-hover = Den Ordner eines Plugins (mit einer plugin.json) in den Plugin-Ordner kopieren
install-from-file = Aus Datei installieren…
install-from-file-hover = Ein Plugin aus einer .zip-Datei installieren
remove-plugin-title = „{ $name }“ entfernen?
remove-plugin-hint = Sein Ordner kommt in den Papierkorb, von dort können Sie ihn zurückholen.
no-plugins-yet = Noch keine Plugins.
no-plugins-hint = Installieren Sie die Beispiele, um zu sehen, wie Plugins funktionieren, suchen Sie Plugins unter Verfügbar, oder legen Sie den Ordner eines Plugins in den Plugin-Ordner.
found-on-path = { $program } (im PATH gefunden)
reload-plugins-hover = Alle Plugins beenden, erneut nach Plugins suchen und sie starten
plugin-filter-hint = Name, Autor oder Beschreibung
refresh = Aktualisieren
refresh-index-hover = Den Plugin-Index neu einlesen
reading-plugin-index = Plugin-Index wird gelesen…
try-again = Erneut versuchen
index-empty = Der Plugin-Index enthält keine Plugins.
no-plugins-match = Keine Plugins passen zum Filter.
plugin-index = Index
plugin-index-hover = Die Adresse der JSON-Liste der Plugins. Mit Enter einlesen.
default = Standard
default-index-hover = Den eigenen Plugin-Index von Ettin Edit verwenden
needs-version = Benötigt Ettin Edit { $version }
update-to = Auf { $version } aktualisieren
installed-no-version = Installiert: keine Version
installed-version = Installiert: { $version }
installed = Installiert
install = Installieren
by-author = Von { $author }
home-page = Homepage
plugin-toggle-hover = Plugin ein- oder ausschalten
plugin-running = Läuft
plugin-stopped = Beendet
plugin-off = Aus
plugin-needs-permission = Wartet auf Erlaubnis
plugin-not-running = Läuft nicht
update-plugin-hover = Die neue Version aus dem Plugin-Index laden; die alte kommt in den Papierkorb
restart = Neu starten
allow-dots = Erlauben…
start = Starten
open-folder = Ordner öffnen
remove-dots = Entfernen…
remove-plugin-hover = Den Ordner des Plugins in den Papierkorb verschieben
trust-changes = Bei Änderungen nicht fragen
trust-changes-hover = Für ein Plugin, das Sie selbst schreiben: nach jeder Änderung ohne Rückfrage ausführen
clear = Leeren
copy-all = Alles kopieren
plugin-log-hint = Meldungen sowie Ausgaben von print() und console.log() aus Plugins
nothing-yet = Noch nichts.
allow-plugin-title = „{ $name }“ ausführen lassen?
plugin-changed = Dieses Plugin hat sich geändert, seit Sie es erlaubt haben.
allow-plugin-warning = Plugins laufen als eigene Programme mit Ihrem Benutzerkonto. Dieses könnte Ihre offenen Dokumente und Ihre Dateien lesen und ändern und das Netzwerk nutzen. Erlauben Sie nur Plugins, denen Sie vertrauen.
author = Autor
runs = Führt aus
allow = Erlauben
dont-allow = Nicht erlauben
dont-allow-hover = Das Plugin ausschalten. Sie können es unter Plugins verwalten wieder einschalten.

## Installing plugins (src/plugins/store.rs)
index-bad-json = Der Plugin-Index ist kein gültiges JSON: { $error }
index-no-list = Der Plugin-Index hat keine Liste „plugins“
zip-unsafe-name = Die ZIP-Datei enthält einen unsicheren Dateinamen, daher wurde nichts installiert: { $name }
zip-no-manifest = Die ZIP-Datei enthält kein { $file }, weder oben noch in einem einzelnen Ordner.
zip-unreadable = Das ist keine ZIP-Datei, die Ettin Edit lesen kann: { $error }
zip-too-many-files = Die ZIP-Datei enthält mehr als { $count } Dateien.
zip-damaged = Die ZIP-Datei ist beschädigt: { $error }
zip-has-link = Die ZIP-Datei enthält einen Link, den Plugins nicht enthalten dürfen: { $name }
zip-encrypted = Die ZIP-Datei ist passwortgeschützt.
zip-too-big = Die ZIP-Datei entpackt sich auf mehr als 256 MB.
checksum-mismatch = Der Download passt nicht zu seiner SHA-256-Prüfsumme (erwartet { $expected }, erhalten { $got }). Er ist möglicherweise beschädigt oder verändert; nichts wurde installiert.
already-in-plugins-folder = Dieser Ordner ist bereits im Plugin-Ordner.
copy-failed = { $path } konnte nicht kopiert werden: { $error }
not-a-plugin = Dies ist kein Plugin, das Ettin Edit ausführen kann: { $error }
plugin-trash-old-failed = Die installierte Kopie von { $id } konnte nicht in den Papierkorb verschoben werden: { $error }
plugin-move-failed = Das Plugin konnte nicht an seinen Platz verschoben werden: { $error }
index-unreachable = Der Plugin-Index war nicht erreichbar ({ $error }). Prüfen Sie die Adresse und ob sie öffentlich ist:
    { $url }
installing = { $name } wird installiert…

## Running plugins (src/plugins/mod.rs)
plugin-started = Gestartet mit { $program } { $script }
runtime-missing = { $runtime } ist nicht installiert („{ $program }“ nicht gefunden). Installieren Sie es oder legen Sie den Pfad unter Plugins > Plugins verwalten fest.
plugin-start-failed = { $program } konnte nicht gestartet werden: { $error }
plugin-exited = Es wurde beendet.
plugin-exit-code = Es wurde mit Exit-Code { $code } beendet. Siehe Plugin-Protokoll.
plugin-died = Es wurde beendet. Siehe Plugin-Protokoll.
plugin-stopped-notice = Das Plugin { $name } wurde beendet. Siehe Plugins > Plugin-Protokoll.

## Shortcut names (src/plugins/shortcut.rs)
unknown-modifier = Unbekannte Zusatztaste „{ $modifier }“ in „{ $shortcut }“
unknown-key = Unbekannte Taste „{ $key }“ in „{ $shortcut }“
shortcut-needs-modifier = „{ $shortcut }“ braucht Ctrl oder Alt, sonst würde es beim Tippen stören
shortcut-reserved = „{ $shortcut }“ ist für Ausschneiden, Kopieren und Einfügen reserviert

## Completion lists (src/lang_api.rs)
api-file-skipped = API-Datei { $name } übersprungen: { $error }

## Language files (src/highlight.rs)
language-file-skipped = Sprachdatei { $file } übersprungen: { $error }

## Plugin panels (src/plugins/panel.rs)
close-panel = Bereich schließen

## Shortcut settings (src/actions.rs)
shortcut-settings-error = Einstellungen der Tastenkombinationen: { $error }

## Toolbar (src/ribbon*.rs, src/app/ribbon.rs)
customise-toolbar = Symbolleiste anpassen
customise-toolbar-dots = Symbolleiste anpassen…
toolbar-show = Symbolleiste
toolbar-collapse-menu = Symbolleiste einklappen
toolbar-collapse = Symbolleiste einklappen
toolbar-expand = Schaltflächen der Symbolleiste anzeigen
toolbar-hide = Symbolleiste ausblenden
toolbar-more = Weitere Befehle
toolbar-hidden-groups = AUSGEBLENDETE GRUPPEN
toolbar-filter-command = Befehlsfilter
menu-bar-show = Menüleiste
menu-bar-hide = Menüleiste ausblenden
menu-bar-show-again = Menüleiste einblenden
menu-bar-hidden-notice = Die Menüleiste ist ausgeblendet. Tippen Sie auf Alt, um sie kurz anzuzeigen, oder drücken Sie { $shortcut }, um sie zurückzuholen.
menu-bar-hidden-notice-alt = Die Menüleiste ist ausgeblendet. Tippen Sie auf Alt, um sie anzuzeigen, und wählen Sie dann Ansicht > Menüleiste, um sie zurückzuholen.
toolbar-empty = Hier gibt es noch keine Schaltflächen. Rechtsklick, um die Symbolleiste anzupassen.
toolbar-remove-button = Aus der Symbolleiste entfernen
toolbar-read-failed = Das Symbolleisten-Layout { $path } konnte nicht gelesen werden: { $error }. Die Standard-Symbolleiste wird verwendet.
toolbar-save-failed = Das Symbolleisten-Layout konnte nicht in { $path } gespeichert werden: { $error }
toolbar-commands = Befehle
toolbar-filter-hint = Befehle filtern
toolbar-layout = Symbolleiste
toolbar-drag-hint = Befehle hierher ziehen, oder Registerkarten, Gruppen und Schaltflächen ziehen, um sie zu verschieben.
toolbar-add = Hinzufügen ›
toolbar-add-tip = Den gewählten Befehl nach der Auswahl rechts einfügen
toolbar-remove = ‹ Entfernen
toolbar-move-up = Nach oben
toolbar-move-down = Nach unten
toolbar-new-tab = Neue Registerkarte
toolbar-new-group = Neue Gruppe
toolbar-new-tab-name = Neue Registerkarte
toolbar-new-group-name = Neue Gruppe
toolbar-no-tabs = Es gibt keine Registerkarten. Mit „Neue Registerkarte“ eine hinzufügen.
toolbar-tab-visible = Diese Registerkarte anzeigen
toolbar-unavailable = Dieser Befehl ist gerade nicht verfügbar (vielleicht läuft sein Plugin nicht). Die Schaltfläche erscheint wieder, sobald er verfügbar ist.
toolbar-select-hint = Registerkarte, Gruppe oder Schaltfläche auswählen, um sie zu ändern.
toolbar-tab-name = Name der Registerkarte:
toolbar-group-name = Name der Gruppe:
toolbar-label = Beschriftung:
toolbar-size = Größe:
toolbar-size-large = Groß
toolbar-size-small = Klein
toolbar-icon = Symbol:
toolbar-choose-icon = Symbol auswählen
toolbar-change-icon = Ändern…
toolbar-default-icon = Symbol des Befehls verwenden
toolbar-icon-filter-hint = Symbole filtern
toolbar-icon-credit = Symbole: Phosphor (MIT-Lizenz)
toolbar-reset = Standard wiederherstellen
toolbar-tab-home = Start
toolbar-tab-edit = Bearbeiten
toolbar-tab-view = Ansicht
toolbar-tab-tools = Werkzeuge
toolbar-group-file = Datei
toolbar-group-clipboard = Zwischenablage
toolbar-group-history = Rückgängig
toolbar-group-find = Suchen
toolbar-group-tabs = Registerkarten
toolbar-group-help = Hilfe
toolbar-group-lines = Zeilen
toolbar-group-sort = Sortieren
toolbar-group-case = Schreibweise
toolbar-group-comment = Kommentar
toolbar-group-blanks = Leerzeichen
toolbar-group-selection = Auswahl
toolbar-group-show = Anzeigen
toolbar-group-zoom = Zoom
toolbar-group-folding = Faltung
toolbar-group-panels = Bereiche
toolbar-group-window = Fenster
toolbar-group-compare = Vergleichen
toolbar-group-json = JSON
toolbar-group-xml = XML
toolbar-group-data = Daten
toolbar-group-macros = Makros
toolbar-group-customise = Anpassen

## Transform menu (1.9.5)

menu-transform = Umwandeln
menu-transform-lines = Zeilen
menu-hash = Prüfsumme
pipe-dots = Durch Befehl filtern…
keep-matching-lines = Passende Zeilen behalten
remove-matching-lines = Passende Zeilen entfernen
keep-matching-lines-dots = Passende Zeilen behalten…
remove-matching-lines-dots = Passende Zeilen entfernen…
count-distinct-lines = Verschiedene Zeilen zählen
affix-lines = Präfix und Suffix hinzufügen
affix-lines-dots = Präfix und Suffix hinzufügen…
number-lines = Zeilen nummerieren
number-lines-dots = Zeilen nummerieren…
reflow = Absätze neu umbrechen
reflow-dots = Absätze neu umbrechen…
convert-to = { $from } nach { $to }
case-camel = camelCase
case-pascal = PascalCase
case-snake = snake_case
case-kebab = kebab-case
case-constant = CONSTANT_CASE
hex-encode = Text zu Hex
hex-decode = Hex zu Text
jwt-decode = JWT dekodieren
hash-copied = { $kind } des Dokuments kopiert: { $hash }
jwt-not-verified = Dekodiert; die Signatur wurde nicht geprüft.
jwt-issued = Ausgestellt { $time }.
jwt-expires = Läuft ab { $time }.
jwt-expired = Abgelaufen { $time }.
pipe-title = Durch Befehl filtern
pipe-hint = Die Auswahl (oder das ganze Dokument) geht an den Befehl, und seine Ausgabe geht dorthin, wo „Ergebnis nach“ es angibt. Befehle laufen im Ordner der Datei; $ETTIN_FILE ist der Pfad der Datei.
pipe-run = Ausführen
pipe-recent = Letzte Befehle
pipe-running = { $command } läuft…
pipe-no-command = Geben Sie einen Befehl ein.
pipe-tab-closed = Der Tab wurde geschlossen, bevor der Befehl laufen konnte.
pipe-start-failed = Der Befehl konnte nicht starten: { $error }
pipe-failed = Der Befehl ist fehlgeschlagen (Exit-Code { $code }); der Text wurde nicht geändert.
pipe-text-changed = Der Text hat sich geändert, während der Befehl lief; seine Ausgabe wurde nicht übernommen.
filter-lines-title = Passende Zeilen behalten oder entfernen
filter-lines-hint = Wirkt auf die ausgewählten Zeilen oder das ganze Dokument.
filter-keep = Passende Zeilen behalten
filter-remove = Passende Zeilen entfernen
filter-no-pattern = Geben Sie den gesuchten Text ein.
affix-lines-title = Präfix und Suffix hinzufügen
affix-lines-hint = Fügt Text am Anfang und Ende jeder ausgewählten Zeile oder jeder Zeile hinzu.
affix-prefix = Vor jeder Zeile
affix-suffix = Nach jeder Zeile
skip-empty-lines = Leere Zeilen überspringen
number-lines-title = Zeilen nummerieren
number-lines-hint = Setzt eine Nummer vor jede ausgewählte Zeile oder jede Zeile.
number-separator = Danach
number-pad-none = Ohne Auffüllen
number-pad-spaces = Ausrichten
number-pad-zeros = Führende Nullen
reflow-title = Absätze neu umbrechen
reflow-hint = Bricht jeden Absatz so um, dass keine Zeile breiter ist als angegeben. Einrückung, Kommentarzeichen und Aufzählungszeichen bleiben erhalten.
reflow-width = Breite
toolbar-tab-transform = Umwandeln
toolbar-group-command = Befehl
toolbar-group-filter = Zeilen
toolbar-group-identifier-case = Bezeichner
toolbar-group-convert = Konvertieren
toolbar-group-encode = Kodieren
# Rezepte (2.0)
menu-recipes = Rezepte
manage-recipes = Rezepte verwalten…
record-recipe = Rezept aufnehmen
stop-recording-recipe = Rezeptaufnahme beenden
open-recipes-folder = Rezeptordner öffnen
reload-recipes = Rezeptordner neu einlesen
run-recipe-on-all-tabs = Auf alle offenen Tabs anwenden
toolbar-group-recipes = Rezepte
recording-recipe = Rezept wird aufgenommen ({ $count } Schritte)
recording-recipe-hover = Jeder Umwandeln-, Zeilen-, Schreibweisen-, Kodierungs- und „Alle ersetzen“-Befehl wird als Schritt hinzugefügt. Klicken, um aufzuhören und das Rezept zu speichern.
recipes-title = Rezepte
recipes-pick = Links ein Rezept wählen oder ein neues anlegen.
recipes-none = Noch keine Rezepte.
recipe-new = Neues Rezept
recipe-new-name = Neues Rezept
recipe-recorded-name = Aufgenommenes Rezept
recipe-duplicate = Duplizieren
recipe-delete = Löschen
recipe-delete-dots = Löschen…
recipe-delete-question = „{ $name }“ löschen?
recipe-import = Rezepte importieren…
recipe-export = Rezept exportieren…
recipe-file-filter = Ettin-Edit-Rezept
recipe-name = Name
recipe-description = Beschreibung
recipe-no-name = Gib dem Rezept einen Namen.
recipe-name-taken = Ein anderes Rezept hat diesen Namen; Menüs und Tastenkürzel führen das erste aus.
recipe-steps = Schritte
recipe-no-steps = Noch keine Schritte. Unten einen hinzufügen oder unter Umwandeln > Rezepte ein Rezept aufnehmen.
recipe-add-step = Schritt hinzufügen
recipe-move-up = Nach oben
recipe-move-down = Nach unten
recipe-remove-step = Diesen Schritt entfernen
recipe-group-replace = Suchen und Ersetzen
recipe-group-encode = Kodieren und Hashen
recipe-group-format = JSON und XML
recipe-group-command = Befehl
recipe-step-replace = „{ $find }“ durch „{ $replace }“ ersetzen
recipe-step-keep = Zeilen mit „{ $pattern }“ behalten
recipe-step-remove = Zeilen mit „{ $pattern }“ entfernen
recipe-step-affix = „{ $prefix }“ vor und „{ $suffix }“ nach jeder Zeile einfügen
recipe-step-number = Zeilen ab { $start } in { $step }er-Schritten nummerieren
recipe-step-reflow = Absätze auf { $width } Spalten umbrechen
recipe-step-hash = { $kind }-Hash des Textes
recipe-step-join = Zeilen verbinden
recipe-step-json = JSON: { $action }
recipe-step-xml = XML: { $action }
recipe-step-command = Befehl ausführen: { $command }
recipe-replace-with = Ersetzen durch
recipe-pattern = Zeilen mit
recipe-convert-from = Von
recipe-convert-to = Nach
recipe-descending = Absteigend
recipe-only-consecutive = Nur direkt aufeinanderfolgende Wiederholungen
recipe-include-blank = Auch Zeilen nur aus Leerzeichen und Tabs
recipe-command = Befehl
recipe-command-hint = Läuft im Ordner der Datei mit dem Text als Eingabe; seine Ausgabe wird der neue Text. Bevor ein Rezept mit Befehl läuft, wirst du gefragt.
recipe-hash-hint = Ersetzt den Text durch seinen Hash.
recipe-count-hint = Ersetzt den Text durch jede einzelne Zeile und wie oft sie vorkommt.
recipe-no-settings = Dieser Schritt hat keine Einstellungen.
recipe-no-find = Gib den zu suchenden Text ein.
recipe-step-not-allowed = Dieser Befehl kann kein Rezeptschritt sein.
recipe-preview-selection = Auswahl
recipe-before = Vorher
recipe-after = Nachher
recipe-step-failed = Schritt { $step }: { $message }
recipe-preview-stopped = Die Vorschau endet vor Schritt { $step }, der einen Befehl ausführt.
recipe-run-all-tabs = Auf alle Tabs anwenden
recipe-confirm-title = Befehle ausführen?
recipe-confirm-text = „{ $name }“ führt diese Befehle auf deinem Computer aus:
recipe-confirm-hint = Führe nur Befehle aus, denen du vertraust. Sie können deine Dateien lesen und ändern.
recipe-confirm-remember = Für dieses Rezept nicht mehr fragen, bis sich seine Befehle ändern
recipe-confirm-run = Befehle ausführen
recipe-running = „{ $name }“ läuft…
recipe-stopped = Angehalten.
recipe-nothing-recorded = Nichts aufgenommen. Nur Befehle, die Text ändern, werden zu Rezeptschritten.
recipe-not-found = Es gibt kein Rezept namens „{ $name }“.
recipe-failed = „{ $name }“ hat bei Schritt { $step } angehalten: { $message } Der Text wurde nicht geändert.
recipe-changed-tabs = „{ $name }“ hat { $count } Tabs geändert.
recipe-no-change = „{ $name }“ hat nichts geändert.
recipe-read-failed = Das Rezept { $path } konnte nicht gelesen werden: { $error }
recipe-save-failed = Das Rezept { $path } konnte nicht gespeichert werden: { $error }

## 2.0.2: Konvertieren in, Lesen als
menu-convert-to = Konvertieren in
menu-read-as = Lesen als
convert-to-format = In { $format } konvertieren
convert-markdown-table = Markdown-Tabelle
convert-reading = Tab wird gelesen…
convert-reading-as = Dieser Tab wird als { $format } gelesen
convert-not-data = Kein JSON, YAML, TOML, XML oder CSV
convert-unknown-format = Das Format dieses Textes ist nicht erkennbar. Wählen Sie eines unter Umwandeln › Konvertieren in › Lesen als.
converted-to = In { $to } konvertiert (gelesen als { $from })
read-as-auto = Automatisch
read-as-auto-format = Automatisch ({ $format })
read-as-named = Lesen als { $format }
toolbar-convert-to = In { $format }
## 2.0.2: Befehlspalette
palette-recipe = Rezept · { $steps ->
    [one] 1 Schritt
   *[other] { $steps } Schritte
}
palette-plugin = Plugin · { $name }
palette-not-for-tab = nicht für diesen Tab
palette-tag-recent = zuletzt
palette-tag-recipe = Rezept
palette-tag-plugin = Plugin
## 2.0.2: Rezeptfenster
recipe-new-short = Neu
recipe-export-short = Exportieren…
recipe-import-short = Importieren…
recipe-folder = Ordner
recipe-reload = Neu laden
recipe-no-shortcut = Keines
recipe-clear-shortcut = Entfernen
recipe-id = ID { $id }
recipe-id-hint = Tastenkürzel und Symbolleisten-Schaltflächen verwenden diese ID und funktionieren daher auch nach dem Umbenennen des Rezepts.
recipe-step-search = Schritte suchen…
json-format-named = JSON formatieren
json-minify-named = JSON minimieren
## 2.1: Transform first
menu-plugin-transforms = Plugin-Umwandlungen
menu-sort-lines = Zeilen sortieren
menu-trim = Leerraum entfernen
menu-macros = Makros
menu-toolbar = Symbolleiste
menu-encode-decode = Kodieren / Dekodieren
more-edit-commands = Weitere Bearbeiten-Befehle
context-selection = Auswahl
sort-lines = Zeilen sortieren
open-in-csvquery = In CsvQuery öffnen
xml-check-syntax-named = XML-Syntax prüfen
xml-validate-named = XML validieren
xml-validate-against-named = XML validieren gegen…
recipes-menu-note = Rezepte wiederholen Umwandlungen und laufen auch auf der Kommandozeile.
macros-menu-note = Makros wiederholen Tastendrücke und Bearbeitungen.
run-last-recipe = Letztes Rezept ausführen
save-macro-as-recipe = Makro als Rezept speichern…
save-macro-as-recipe-hint = Macht aus dem aufgezeichneten Makro ein Rezept, das Sie bearbeiten, teilen und auf der Kommandozeile ausführen können.
macro-not-recipe = Kann kein Rezept werden: { $step } ist keine Umwandlung.
macro-step-typing = die Eingabe „{ $text }“
macro-step-key = die Taste { $key }
recipe-from-macro-name = Rezept aus Makro
toolbar-simplified = Vereinfachte Symbolleiste
toolbar-mode-full = Voll
toolbar-mode-simplified = Vereinfacht
toolbar-mode-full-tip = Das volle Menüband zeigen
toolbar-mode-simplified-tip = Eine Reihe kleiner Schaltflächen zeigen
toolbar-help = Hilfe
toolbar-menus = Aufklappmenüs
toolbar-menu-format = Formatieren
toolbar-menu-case = Groß/Klein
toolbar-group-transform = Umwandeln
toolbar-group-quick = Schnell
toolbar-group-search = Suchen
toolbar-keep-matching = Passende behalten
toolbar-remove-duplicates = Duplikate entfernen

## 2.2: previews, Result goes to, suggestions, Welcome
recipe-preview-active-tab = Aktiver Tab
recipe-preview-input = Vorschau auf:
recipe-sample-text = Beispieltext
recipe-sample-hint = Beispieltext (wird mit dem Rezept gespeichert)
recipe-show-after = Ausgabe nach:
recipe-after-step = Schritt { $step }
recipe-after-last-step = Dem letzten Schritt
recipe-output-after-step = Nach Schritt { $step }
filter-pattern = Muster
filter-keep-button = Zeilen behalten
filter-remove-button = Zeilen entfernen
filter-kept-count = { $kept } von { $lines } Zeilen bleiben
filter-marks-hint = behaltene Zeilen sind im Editor grün, entfernte rot
filter-counting = Passende Zeilen werden gezählt…
pipe-command = Befehl
pipe-preview = Vorschau
pipe-preview-tip = Den Befehl auf den Text anwenden, ohne etwas zu ändern
pipe-input-tab = Eingabe (ganzer Tab)
pipe-input-selection = Eingabe (Auswahl)
pipe-output = Ausgabe
pipe-output-done = Ausgabe · { $seconds } s · Exit-Code { $code }
pipe-output-hint = Mit Vorschau sehen Sie, was der Befehl aus dem Text macht. Befehle laufen nur bei Vorschau oder Ausführen.
pipe-previewing = { $command } läuft…
pipe-no-stderr = Der Befehl ist fehlgeschlagen und hat nichts auf die Standardfehlerausgabe geschrieben.
pipe-preview-stale = Befehl oder Text haben sich seit dieser Vorschau geändert.
preview-first-lines = Eingabe (erste { $lines } Zeilen)
result-goes-to = Ergebnis nach
result-this-tab = Diesen Tab
result-new-tab = Neuen Tab
result-clipboard = Zwischenablage
result-this-tab-failed = Der Befehl ist in der Vorschau fehlgeschlagen, daher kann seine Ausgabe den Text nicht ersetzen.
result-tab-unchanged = Dieser Tab bleibt unverändert.
result-copied = Ergebnis in die Zwischenablage kopiert ({ $lines } Zeilen).
size-bytes = { $bytes } Bytes
suggest-minified-json = Dieses JSON steht in einer Zeile ({ $size }).
suggest-minified-xml = Dieses XML steht in einer Zeile ({ $size }).
suggest-jwt = Das sieht nach einem JSON Web Token aus.
suggest-base64 = Das sieht nach Base64-Text aus.
suggest-csv = Das sieht nach CSV aus.
suggest-mixed-line-endings = Diese Datei mischt Zeilenenden (LF und CRLF).
suggest-mixed-indentation = Diese Datei ist mit Tabs und Leerzeichen eingerückt.
suggest-format = Formatieren
suggest-show-tree = Als Baum zeigen
suggest-decode = Dekodieren
suggest-open-csvquery = In CsvQuery öffnen
suggest-show-table = Als Tabelle zeigen
suggest-fix = Beheben
suggest-fix-lf = Überall LF verwenden
suggest-fix-crlf = Überall CRLF verwenden
suggest-fix-spaces = Tabs in Leerzeichen
suggest-fix-tabs = Leerzeichen in Tabs
suggest-never = Nicht mehr vorschlagen
suggest-close = Schließen
prefs-suggestions-heading = Vorschläge
prefs-suggestions = Umwandlungen für geöffnete oder eingefügte Texte vorschlagen (etwa Formatieren für einzeiliges JSON)
prefs-suggestions-reset = Die { $count } abgelehnten Vorschläge wieder zeigen
welcome = Willkommen
welcome-tab = Willkommen
welcome-title = Willkommen bei Ettin Edit
welcome-whats-new-title = Neu in Ettin Edit { $version }
welcome-intro = Ein Texteditor, der Text in anderen Text verwandelt: Logdateien filtern, JSON in YAML umwandeln, Tokens dekodieren und die Schritte als Rezept speichern.
welcome-card-transform = Zeilen filtern, JSON/YAML/CSV umwandeln, Schreibweise ändern, kodieren und hashen
welcome-card-palette-title = Befehlspalette
welcome-card-palette = eintippen, was Sie tun möchten
welcome-card-recipes = eine Liste von Schritten speichern und wieder abspielen, auch von der Kommandozeile
welcome-card-pipe = Text durch einen beliebigen Shell-Befehl leiten
welcome-try-sample = Mit Beispieltext ausprobieren
welcome-open-manual = Handbuch öffnen
welcome-open-file = Datei öffnen…
welcome-show-at-start = Diesen Tab beim Start von Ettin Edit zeigen
welcome-whats-new = Neu in { $version }
welcome-full-notes = Vollständige Versionshinweise
welcome-whats-new-hint = Nach einem Update öffnet sich dieser Tab einmal mit den Neuerungen. Hilfe › Willkommen öffnet ihn wieder.
tour-step-of = Schritt { $step } von { $steps }:
tour-step-keep = nur die ERROR-Zeilen behalten mit Umwandeln › Zeilen › Passende Zeilen behalten. Die Zeilen werden grün und rot, bevor Sie Zeilen behalten drücken.
tour-step-convert = die JSON-Zeile mit Umwandeln › Konvertieren in › YAML in YAML verwandeln.
tour-done = Das war's: zwei Umwandlungen, jede mit Strg+Z rückgängig zu machen. Zeichnen Sie die Schritte als Rezept auf, um sie auf jede Datei anzuwenden.
tour-show-me = Zeig es mir
tour-end = Tour beenden

# Plugins as transforms (2.3)
plugin-error-at = { $name } wurde mit einem Fehler beendet: { $file } Zeile { $line }: { $error }
open-file-button = Öffnen
plugin-transform-error-at = { $message } (Zeile { $line }, Spalte { $column })
plugin-transform-error-line = { $message } (Zeile { $line })
recipe-plugin-missing = Das Plugin „{ $plugin }“ ist nicht installiert. Installieren Sie es über Plugins › Plugins verwalten.
recipe-plugin-not-allowed = Das Plugin „{ $plugin }“ darf nicht laufen. Erlauben Sie es zuerst in Ettin Edit (Plugins › Plugins verwalten).
plugin-no-documents-here = Tabs sind hier nicht verfügbar: Eine Umwandlung arbeitet nur mit dem Text, den sie erhält.
plugin-ended-during-transform = Das Plugin „{ $plugin }“ wurde beendet, bevor es fertig war.
plugin-transform-timeout = „{ $plugin }“ hat länger als { $seconds } Sekunden gebraucht und wurde abgebrochen.
recipe-plugin-no-transform = Das Plugin „{ $plugin }“ hat keine Umwandlung „{ $transform }“.
plugin-transform-no-text = Die Umwandlung des Plugins hat keinen Text zurückgegeben.
choose-hint = Tippen zum Filtern
copy-command-id = Befehls-ID kopieren
copy-command-id-hover = Kopiert die ID, die Plugins an run_action übergeben, um diesen Befehl auszuführen
done = Fertig
markdown-preview = Markdown-Vorschau
markdown-preview-title = Vorschau: { $title }
markdown-preview-not-markdown = Dieser Tab ist kein Markdown. Öffnen Sie eine .md-Datei oder wechseln Sie zu einer, um sie anzuzeigen.
markdown-preview-too-big = Dieser Tab ist zu groß für eine Vorschau beim Tippen.
markdown-preview-empty = Noch nichts zu zeigen. Fangen Sie an, Markdown zu schreiben.
markdown-preview-sync = Mit dem Text scrollen
markdown-preview-save-first = Speichern Sie diesen Tab zuerst, damit Links zu anderen Dateien gefunden werden.
markdown-preview-missing = { $path } wurde nicht gefunden
new-plugin-dots = Neues Plugin…
new-plugin-title = Neues Plugin
new-plugin-name = Name
new-plugin-default-name = Mein Plugin
new-plugin-language = Sprache
new-plugin-starts-with = Beginnt mit
new-plugin-transform = Einer Transformation (Text rein, Text raus)
new-plugin-command = Einem Befehl im Plugins-Menü
new-plugin-panel = Einem Seitenbereich
new-plugin-create = Erstellen und öffnen
new-plugin-folder = Es kommt nach { $folder }
new-plugin-exists = { $folder } gibt es schon. Wählen Sie einen anderen Namen.
new-plugin-failed = Das Plugin konnte nicht nach { $folder } geschrieben werden: { $error }
plugin-adds-transforms = Transformationen:
plugin-options-dots = Optionen…
plugin-options-hover = Die Einstellungen dieses Plugins ändern
plugin-options-title = Optionen für { $name }
plugin-options-intro = Änderungen gelten sofort; das Plugin wird benachrichtigt.
plugin-options-reset = Auf Standard zurücksetzen
plugin-options-escapes = Im Text ist \n ein Zeilenumbruch und \t ein Tabulator.
plugin-transform-from = Aus dem Plugin { $plugin }
plugin-previewing = Vorschau wird erstellt…
plugin-preview-first-lines = Die Vorschau zeigt die ersten { $count } Zeilen.
plugin-transform-gone = „{ $transform }“ ist nicht verfügbar: Sein Plugin läuft nicht.
plugin-reloaded = Nach Dateiänderungen neu geladen.
recipe-plugin-needs-plugins = Dieser Schritt führt eine Plugin-Transformation aus, die hier nicht laufen kann.
recipe-plugin-step-empty = Wählen Sie Plugin und Transformation für diesen Schritt.
recipe-step-plugin = { $title } ({ $plugin })
recipe-preview-stopped-plugin = Die Vorschau endet vor Schritt { $step }, der eine Plugin-Transformation ausführt.
reload-on-change = Bei Dateiänderungen neu laden
reload-on-change-hover = Für Plugins, die Sie selbst schreiben: startet es innerhalb einer Sekunde nach dem Speichern einer seiner Dateien neu.

## 2.4 Einstellungen an einem Ort
changes-apply-live = Änderungen wirken sofort.
undo-changes = Änderungen rückgängig
undo-changes-tip = Alles so wiederherstellen, wie es beim Öffnen dieses Fensters war
reset-section = Bereich zurücksetzen
reset-section-tip = Die Einstellungen unter { $section } auf die Standardwerte zurücksetzen
theme-reset-tip = „{ $name }“ so wiederherstellen, wie Ettin Edit es mitbringt
theme-reset-custom-tip = Nur mitgelieferte Designs lassen sich zurücksetzen. „Änderungen rückgängig“ stellt dieses Design so wieder her, wie es beim Öffnen des Fensters war.
recipes-undo-tip = Die Rezepte so wiederherstellen, wie sie beim Öffnen dieses Fensters waren (seitdem hinzugefügte werden entfernt)
prefs-search-hint = Einstellungen suchen
prefs-also = AUSSERDEM
prefs-other-windows = Andere Fenster
prefs-no-match = Keine Einstellung passt zu „{ $search }“.
prefs-appearance = Darstellung
prefs-languages = Sprachen
prefs-theme = Design
prefs-follow-system = Der Hell/Dunkel-Einstellung des Systems folgen
prefs-light-dark-pair = Helles / dunkles Design
prefs-interface = Oberfläche
prefs-interface-size = Größe der Oberfläche
prefs-toolbar = Symbolleiste
prefs-toolbar-full = Volles Menüband
prefs-toolbar-hidden = Ausgeblendet
prefs-menu-bar = Menüleiste
prefs-tab-key = Tab-Taste fügt ein
prefs-detect-indent = Einrückung beim Öffnen einer Datei erkennen (prüft die ersten 1000 eingerückten Zeilen)
prefs-per-language = Einstellungen je Sprache
prefs-per-language-hint = Diese haben Vorrang vor den Einstellungen oben. Ist die Erkennung an, hat die Einrückung der Datei selbst Vorrang vor beiden.
prefs-default = Standard
prefs-tabs = Tabulatoren
prefs-spaces = Leerzeichen
prefs-remove-language = Die Einstellungen dieser Sprache entfernen
prefs-pick-language = Sprache wählen…
prefs-add = Hinzufügen
prefs-hash-no-selection = Hash ohne Auswahl
prefs-hash-clipboard = Hash in die Zwischenablage kopieren
prefs-hash-new-tab = Hash in einem neuen Tab öffnen
prefs-recipe-commands = Befehlsschritte in Rezepten
prefs-ask-first = Vor dem Ausführen fragen
prefs-run-without-asking = Ohne Nachfrage ausführen
prefs-shell = Shell für Befehle
prefs-shell-tip = Das Programm samt Optionen, das „Durch Befehl filtern“ und Befehlsschritte in Rezepten ausführt, etwa bash -c oder pwsh -Command. Leer nimmt die Shell des Systems.
prefs-format-indent = Einrückung für JSON / XML
prefs-indent-document = Wie im Dokument
prefs-indent-n-spaces = { $count } Leerzeichen
prefs-settings-files = Einstellungsdateien
prefs-settings-files-hint = Exportieren legt Einstellungen, Tastenkombinationen, Symbolleiste, Designs, Rezepte und Sprachen in eine Datei, die sich auf einen anderen Computer mitnehmen lässt.
keymap-label = Tastenbelegung
keymap-ettin = Ettin (wie Notepad++)
keymap-vs-code = Wie VS Code
keymap-hint = Ihre eigenen Änderungen gelten zusätzlich zur Tastenbelegung.
export-settings = Einstellungen exportieren
export-settings-dots = Einstellungen exportieren…
import-settings = Einstellungen importieren
import-settings-dots = Einstellungen importieren…
open-settings-folder = Einstellungsordner öffnen
settings-file-type = Ettin-Edit-Einstellungen
settings-exported = { $count } Dateien nach { $file } exportiert
settings-export-failed = Die Einstellungen konnten nicht nach { $file } exportiert werden: { $error }
settings-import-failed = { $file } konnte nicht importiert werden: { $error }
settings-imported = Einstellungen importiert.
settings-imported-backup = Einstellungen importiert. Ihre alten Dateien liegen in { $folder }
import = Importieren
import-from = { $file } (aus Ettin Edit { $version })
import-not-settings = diese Datei stammt nicht aus „Einstellungen exportieren“
import-contents = Einstellungsdateien: { $settings } · Designs: { $themes } · Rezepte: { $recipes } · Sprachen: { $languages }
import-replaces-nothing = Nichts von Ihren jetzigen Dateien wird ersetzt.
import-replaces = { $count ->
    [one] Sie ersetzt 1 Ihrer jetzigen Dateien:
   *[other] Sie ersetzt { $count } Ihrer jetzigen Dateien:
}
import-backup-note = Von jeder ersetzten Datei kommt vorher eine Kopie in den Ordner „backups“ im Einstellungsordner.
import-plugins-missing = Plugins, mit denen sie benutzt wurde und die hier fehlen: { $plugins }. Installieren Sie sie über Plugins > Plugins verwalten.
live-reloaded = { $file } neu geladen
live-reload-failed = { $file } wurde geändert, ist aber nicht lesbar: { $error }. Die letzte funktionierende Fassung bleibt aktiv.
live-reload-place = Zeile { $line }, Spalte { $column }

## 2.5: Einfache Designfarben und Sprache definieren
theme-view-simple = Einfach
theme-view-simple-tip = Etwa zehn benannte Farben. Eine Änderung gilt für jede Regel dieser Textart.
theme-view-advanced = Erweitert (Scopes)
theme-view-advanced-tip = Alle Regeln des Designs mit ihren TextMate-Scopes.
theme-simple-hint = Wählen Sie eine Farbe für jede Textart. Sie gilt für alle Sprachen.
what-colours-this = 🎯 Was färbt das?
what-colours-this-tip = Klicken Sie danach auf ein Wort in der Vorschau oder im Editor, um seine Farbe und Regel zu sehen.
what-colours-this-waiting = Klicken Sie auf ein Wort in der Vorschau oder im Editor…
what-colours-this-hint = Tipp: Mit „Was färbt das?“ und einem Klick auf ein Wort finden Sie seine Farbe.
preview = Vorschau
simple-text = Normaler Text
simple-comments = Kommentare
simple-strings = Zeichenketten
simple-numbers = Zahlen
simple-keywords = Schlüsselwörter
simple-functions = Funktionen
simple-types = Typen
simple-tags = Tags
simple-attributes = Attribute
simple-errors = Fehler
simple-text-tip = Text, den keine andere Farbe abdeckt.
simple-scopes-tip = Scopes: { $scopes }
simple-colour-name = Farbe für { $name }
simple-background-name = Hintergrund für { $name }
style-window = Fenster
style-window-hint = Bei „Automatisch“ wird die Farbe aus den Editorfarben berechnet.
style-window-automatic = Automatisch
style-tab-strip = Tableiste
style-panels = Bereiche und Dialoge
style-status-bar = Statusleiste
style-ribbon = Menüband und Menüleiste
picked-colour = Das ist { $name }
picked-scope = Scope: { $scope }
picked-stack-tip = Alle Scopes an dieser Stelle, äußerster zuerst.
picked-rule = Gefärbt durch die Regel „{ $rule }“.
picked-rule-separate = Diese Stelle hat eine eigene Regel; eine Änderung an { $name } ändert sie nicht. Ändern Sie sie unter „Erweitert“.
picked-rule-shared = Diese Regel färbt auch anderen Text. Eine Änderung an { $name } gibt diesem Text die neue Farbe und lässt den Rest unverändert.
picked-default = Keine Regel färbt das: Es ist normaler Text.
picked-rule-marker = Die Regel, die das Angeklickte färbt
show-in-advanced = In „Erweitert“ zeigen
define-language = Sprache definieren
define-language-dots = Sprache definieren…
lang-editing = Sprache:
lang-new = Neue Sprache
edit-json-instead = Stattdessen JSON bearbeiten
edit-json-instead-tip = Öffnet die .lang.json-Datei in einem Tab, für Einstellungen, die dieses Formular nicht hat (siehe Sprachhandbuch).
lang-cant-show = Diese Sprache nutzt Dinge, die dieses Formular nicht zeigen kann: { $reasons }. Stattdessen das JSON öffnen?
open-json = JSON öffnen
lang-name = Name
lang-name-hint = Gcode
lang-extensions = Dateiendungen
lang-extensions-hint = gcode nc
lang-line-comment = Zeilenkommentar
lang-block-comment = Block
lang-strings = Zeichenketten
lang-escape = Escape \
lang-multiline = Über mehrere Zeilen
lang-numbers = Zahlen
lang-colour-numbers = Zahlen färben
lang-numbers-after-letters = Auch direkt nach einem Buchstaben (X10)
lang-numbers-after-letters-tip = Für Sprachen wie G-Code, in denen Buchstabe und Zahl zusammengehören. Aus bleibt die 10 in X10 ungefärbt, wie ein Teil eines Namens.
lang-case = Groß/klein
lang-ignore-case = Groß-/Kleinschreibung bei Schlüsselwörtern ignorieren (IF, If und if)
lang-operators = Operatoren
lang-folding = Falten
lang-folding-hint = {"{"} {"}"}, if end
lang-folding-tip = Paare, die einen Block öffnen und schließen, durch Kommas getrennt. Code dazwischen lässt sich falten.
lang-keyword-lists = Schlüsselwortlisten
lang-keywords-list = Schlüsselwörter
lang-list-name = Name
lang-list-words = Wörter, durch Leerzeichen getrennt
lang-remove-list = Liste entfernen
lang-add-list = + Liste hinzufügen
lang-preview = Vorschau mit Beispieltext
lang-sample-reset = Eingebautes Beispiel
lang-sample-from-tab = Aktuellen Tab verwenden
lang-sample-from-tab-tip = Vorschau mit den ersten Zeilen des aktiven Tabs. Sie können auch in der Vorschau tippen oder einfügen.
lang-sample-hint = Tippen oder fügen Sie hier eigenen Text ein, um die Sprache auszuprobieren.
lang-sample-comment = ein Kommentar
lang-sample-text = Text
lang-sample-block = Blockkommentar
lang-preview-error = Die Sprache lässt sich noch nicht erstellen: { $error }
lang-saves-to = Speichert in { $path }
save-language = Sprache speichern
save-language-disabled = Geben Sie der Sprache zuerst einen Namen.
language-saved = { $name } in { $path } gespeichert. Dateien mit ihren Endungen verwenden sie jetzt.
lang-not-an-object = Die Datei ist kein JSON-Objekt
lang-unsupported-spaces = { $setting } hat Einträge mit Leerzeichen
lang-unsupported-block = mehr als ein Blockkommentar
lang-unsupported-strings = eigene Zeichenketten-Begrenzer oder Escapes
lang-unsupported-group = die Schlüsselwortgruppe „{ $group }“
lang-unsupported-folding = Faltpaare mit Leerzeichen oder Kommas
lang-unsupported-first-line = ein Muster für die erste Zeile
lang-unsupported-setting = die Einstellung „{ $setting }“

## Spalten und Felder (2.6)
menu-columns = Spalten und Felder
sort-natural = Natürlich sortieren (file2 vor file10)
sort-natural-asc = Natürlich aufsteigend sortieren
sort-natural-desc = Natürlich absteigend sortieren
extract-matches = Treffer herausziehen
extract-matches-dots = Treffer herausziehen…
align-columns = Spalten ausrichten
align-columns-dots = Spalten ausrichten…
select-fields = Felder auswählen
select-fields-dots = Felder auswählen…
sort-by-field = Zeilen nach Feld sortieren
sort-by-field-dots = Zeilen nach Feld sortieren…
split-lines-at = Zeilen teilen bei
split-lines-at-dots = Zeilen teilen bei…
join-lines-with = Zeilen verbinden mit
join-lines-with-dots = Zeilen verbinden mit…
menu-escape-string = Zeichenkette maskieren
menu-unescape-string = Maskierung aufheben
menu-unix-time = Unix-Zeit
unix-to-date-utc = Unix-Zeit in Datum (UTC)
unix-to-date-local = Unix-Zeit in Datum (Ortszeit)
date-to-unix-seconds = Datum in Unix-Zeit (Sekunden)
date-to-unix-millis = Datum in Unix-Zeit (Millisekunden)
field-hint-extract = Jeder Treffer eines regulären Ausdrucks, einer pro Zeile. Wähle eine Gruppe, um nur diesen Teil jedes Treffers zu behalten.
field-hint-align = Füllt die Zellen auf, sodass die Trennzeichen untereinander stehen. Zeilen ohne Trennzeichen bleiben, wie sie sind.
field-hint-select = Behält oder entfernt Felder in der angegebenen Reihenfolge. CSV-Felder in Anführungszeichen mit Kommas bleiben ganz.
field-hint-sort = Sortiert ganze Zeilen nach einem Feld. Zeilen ohne dieses Feld kommen zuerst.
field-hint-split = Setzt jedes Stück zwischen den Trennzeichen in eine eigene Zeile.
field-hint-join = Verbindet die Zeilen zu einer, auf Wunsch jede in Anführungszeichen: aus einer Liste wird 'a', 'b', 'c' für SQL IN (…).
field-pattern = Muster (Regex)
field-group = Gruppe
field-group-hint = 0 behält den ganzen Treffer; 1 die erste ( … )-Gruppe und so weiter.
field-unique = Jeden Treffer nur einmal
field-delimiter = Trennzeichen
field-delimiter-hint = \t für Tab; leer für Leerzeichen
field-spaced = Ein Leerzeichen auf beiden Seiten des Trennzeichens
field-fields = Felder
field-keep = Diese Felder behalten
field-remove = Diese Felder entfernen
field-output-delimiter = Verbinden mit
field-output-same = demselben Trennzeichen
field-field = Feldnummer
field-order-natural = Natürlich
field-order-text = Text
field-order-number = Zahl
field-split-at = Teilen bei
field-trim = Leerzeichen entfernen und leere Stücke weglassen
field-separator = Trennzeichen
field-quote = Jedes einschließen in
field-quote-hint = Ein Anführungszeichen in einer Zeile wird verdoppelt, wie SQL und CSV es erwarten.
field-utc = UTC (endet auf Z)
field-local = Ortszeit mit Versatz
field-seconds = Sekunden
field-millis = Millisekunden
field-spaces = Leerzeichen
recipe-step-extract = Treffer von „{ $pattern }“ herausziehen
recipe-step-align = Spalten an „{ $delimiter }“ ausrichten
recipe-step-select-fields = Felder { $fields } behalten
recipe-step-remove-fields = Felder { $fields } entfernen
recipe-step-sort-field = Zeilen nach Feld { $field } sortieren
recipe-step-split = Zeilen bei „{ $separator }“ teilen
recipe-step-join-with = Zeilen mit „{ $separator }“ verbinden, in { $quote }
recipe-step-escape = Für { $language } maskieren
recipe-step-unescape = { $language }-Maskierung aufheben

## Akkorde (2.6)
chord-two-keys = „{ $shortcut }“ ist kein Akkord: Schreibe zwei Tasten mit Leerzeichen, etwa Strg+K S.
chord-waiting = { $key } wurde gedrückt. Warte auf die zweite Taste… (Esc bricht ab)
chord-unknown = { $chord } ist kein Befehl.
chord-first-in-use = { $shortcut } ist das Tastenkürzel von { $command }, daher kann kein Akkord damit beginnen.
chord-in-use = { $chord } ist bereits der Akkord von { $command }.
shortcut-starts-chord = { $shortcut } beginnt einen Akkord von { $command } und kann daher kein Tastenkürzel sein.
press-chord-first = Drücke die erste Taste des Akkords, etwa Strg+K… (Esc bricht ab)
press-chord-second = { $key }, dann… drücke die zweite Taste.
add-chord = Akkord hinzufügen…
add-chord-tip = Ein Kürzel aus zwei Tasten wie Strg+K S: eine Taste mit Strg oder Alt drücken, loslassen, dann eine zweite Taste drücken.

## Markdown-Webbilder (2.6)
prefs-markdown-preview = Markdown-Vorschau
prefs-markdown-web-images = Bilder aus dem Web laden
prefs-markdown-web-images-tip = Bilder mit http://- oder https://-Adressen werden aus dem Internet geladen, wodurch dieser Server erfährt, dass du die Datei geöffnet hast. Aus: Sie erscheinen als ihre Beschreibung; Bilder aus Dateien werden weiter gezeigt.
web-image-blocked = Nicht geladen: { $address }. Das Laden von Webbildern ist unter Optionen > Darstellung > Markdown-Vorschau ausgeschaltet.

## Dialog windows (2.6)
window-maximize = Fenster ausfüllen
window-restore = Größe wiederherstellen

## Plugin folders (2.6)
plugin-folder-added = Neues Plugin { $name } gefunden.
plugin-folder-removed = { $name } beendet: sein Ordner wurde entfernt.
