# Ettin Edit's interface text in English: the source of every translation.
#
# Each line is `id = text`. Translations copy the ids and change the text.
# Text in { $braces } is filled in by the app; keep those names as they are.
# Fluent syntax: https://projectfluent.org/fluent/guide/

# The language's own name, shown in Preferences.
language-name = English

## Main window: menu bar, tabs, status bar, dialogs (src/app.rs)
theme-problem = The theme has a problem: { $error }
theme-save-failed = Couldn't save the theme to { $path }: { $error }
open-lossy = { $path } has bytes that aren't valid { $encoding }. They are shown as � and will be replaced if you save. If the text looks wrong, pick its encoding in Encoding > Reopen With.
open-failed = Could not open { $path }: { $error }
dialog-open = Open
dialog-save-as = Save As
save-failed = Could not save { $path }: { $error }
rename-bad-name = The name can't contain / or \.
rename-exists = "{ $name }" already exists in this folder.
rename-failed = Couldn't rename the file: { $error }
not-saved-yet = { $name } (not saved yet)
restore-themes-failed = Couldn't restore the built-in themes: { $error }
reload-failed = Could not reload { $name }: { $error }
reopen-lossy = Some bytes in { $name } aren't valid { $encoding }. They are shown as �.
command-palette-title = Command palette
command-palette-hint = Type part of a command's name
tab-list-title = Open tabs
tab-list-hint = Type part of a tab's name
no-results = No results
selected-count = { $count } selected
marked-count = { $count } marked
dialog-open-workspace = Open Folder as Workspace
distraction-free-notice = Distraction-free mode. Press F11 to leave it.
wrapped-to-top = Wrapped to the top
wrapped-to-bottom = Wrapped to the bottom
not-found = Not found
no-more-matches = No more matches
replaced-count = { $count } replaced
search-start-failed = Could not start the search: { $error }
find-in-files-title = "{ $text }" in { $folder }
invalid-regex = Invalid regex: { $error }
about-ettin-edit = About Ettin Edit
menu-file = File
menu-edit = Edit
menu-search = Search
menu-view = View
menu-encoding = Encoding
menu-theme = Theme
menu-language = Language
menu-settings = Settings
menu-tools = Tools
menu-plugins = Plugins
menu-macro = Macro
menu-help = Help
open-recent = Open Recent
no-recent-files = No recent files
new = New
open-dots = Open…
clear-recent-files = Clear Recent Files
save = Save
save-as = Save As…
save-all = Save All
reload-from-disk = Reload from Disk
rename-dots = Rename…
move-to-trash = Move to Trash…
open-containing-folder = Open Containing Folder
open-terminal-here = Open Terminal Here
print-dots = Print…
export-html = Export as HTML…
open-folder-as-workspace = Open Folder as Workspace…
close-workspace = Close Workspace
close = Close
close-all = Close All
close-others = Close Others
exit = Exit
find-dots = Find…
replace-dots = Replace…
find-next = Find Next
find-previous = Find Previous
find-selected-next = Find Selected Next
find-selected-previous = Find Selected Previous
find-in-files = Find in Files…
find-all-in-document = Find All in Current Document
find-all-in-tabs = Find All in Open Tabs
select-all-matches = Select All Matches
mark-all = Mark All
clear-all-marks = Clear All Marks
go-to-line = Go to Line…
menu-bookmark = Bookmark
toggle-bookmark = Toggle Bookmark
next-bookmark = Next Bookmark
previous-bookmark = Previous Bookmark
clear-all-bookmarks = Clear All Bookmarks
cut-bookmarked-lines = Cut Bookmarked Lines
copy-bookmarked-lines = Copy Bookmarked Lines
remove-bookmarked-lines = Remove Bookmarked Lines
remove-unbookmarked-lines = Remove Unbookmarked Lines
inverse-bookmarks = Inverse Bookmarks
word-wrap = Word Wrap
line-numbers = Line Numbers
highlight-current-line = Highlight Current Line
menu-show-symbol = Show Symbol
show-whitespace = Show Whitespace
show-end-of-line = Show End of Line
show-invisible = Show Invisible Characters
show-all-characters = Show All Characters
indent-guides = Indent Guides
long-line-marker-column = Long-Line Marker (column { $column })
function-list = Function List
document-map = Document Map
character-panel = Character Panel
hex-view = Hex View
folder-as-workspace = Folder as Workspace
distraction-free-mode = Distraction-Free Mode
tab-list = Tab List…
command-palette = Command Palette…
move-to-other-view = Move to Other View
clone-to-other-view = Clone to Other View
focus-other-view = Focus Other View
menu-folding = Folding
fold-all = Fold All
unfold-all = Unfold All
fold-current-level = Fold Current Level
unfold-current-level = Unfold Current Level
fold-level = Fold Level { $level }
zoom-in = Zoom In
zoom-out = Zoom Out
reset-zoom = Reset Zoom
summary-dots = Summary…
themes-dark = Dark
themes-light = Light
themes-custom = Custom
edit-theme-colours = Edit Theme Colours…
open-themes-folder = Open Themes Folder
reload-themes = Reload Themes
restore-built-in-themes = Restore Built-in Themes
preferences-dots = Preferences…
shortcut-mapper-dots = Shortcut Mapper…
auto-close-brackets = Auto-Close Brackets and Quotes
smart-indentation = Smart Indentation
indent-with-spaces = Indent With Spaces
word-completion = Word Completion
keyword-completion = Keyword and Function Completion
parameter-hints = Parameter Hints
single-window-long = Single Window (Open Files in the Running Ettin Edit)
remember-unsaved = Remember Unsaved Changes on Exit
auto-reload = Reload Files Changed on Disk Automatically
long-line-column = Long-line column
user-manual = User Manual
user-manual-web = User Manual on the Web
check-for-updates = Check for Updates…
check-updates-automatically = Check for Updates Automatically
encoding-save-as = Save as
reopen-with = Reopen With
only-saved-files = Only for files that are saved on disk
stop-recording = Stop Recording
start-recording = Start Recording
playback = Playback
run-macro-multiple = Run a Macro Multiple Times…
save-current-macro = Save Current Macro…
delete-saved-macro = Delete Saved Macro
new-language = New Language…
open-languages-folder = Open Languages Folder
open-completion-lists-folder = Open Completion Lists Folder
reload-languages = Reload Languages
language-guide = Language Guide
all-open-tabs = All open tabs
new-tab = New tab
unpin-tab = Unpin Tab
pin-tab = Pin Tab
copy-full-path = Copy Full Path
copy-file-name = Copy File Name
status-length-lines = length: { $length }   lines: { $lines }
click-to-copy = Click to copy
recording-macro = Recording macro
read-only-status = Read-only
column-mode-status = Column mode
column-mode-status-hover = Dragging and Shift+arrows select a rectangle. Click to turn off.
click-to-allow-editing = Click to allow editing
status-position = Ln { $line }, Col { $column }
status-selection = Sel { $count }
status-carets = { $count } carets
go-to-line-hover = Go to line (Ctrl+G)
save-changes-title = Save changes?
save-changes-question = "{ $name }" has unsaved changes. Save them before closing?
cancel = Cancel
dont-save = Don't Save
trash-failed = Couldn't move { $name } to the trash: { $error }
about-version = Version { $version }
about-tagline = A fast, modern text editor for Linux and Windows.
about-built-with = Written in Rust with egui and syntect.
error-title = Something went wrong
ok = OK

## Documents (src/document.rs)
eol-unix = Unix (LF)
eol-windows = Windows (CR LF)
eol-mac = Macintosh (CR)
cant-encode-char = "{ $char }" (U+{ $code }) can't be saved as { $encoding }. Choose an encoding that has it, such as UTF-8, from the Encoding menu.
untitled-name = new { $number }

## Edit and Tools menus (src/app/menus.rs)
duplicate = Duplicate
delete-lines = Delete Lines
move-lines-up = Move Up
move-lines-down = Move Down
join-lines = Join Lines
split-lines = Split Lines at Long-Line Column
insert-blank-above = Insert Blank Line Above
insert-blank-below = Insert Blank Line Below
sort-lexical-asc = Sort Lexicographically Ascending
sort-lexical-desc = Sort Lexicographically Descending
sort-ignore-case-asc = Sort Ignoring Case Ascending
sort-ignore-case-desc = Sort Ignoring Case Descending
sort-integer-asc = Sort As Integers Ascending
sort-integer-desc = Sort As Integers Descending
sort-decimal-asc = Sort As Decimals Ascending
sort-decimal-desc = Sort As Decimals Descending
sort-length-asc = Sort By Length Ascending
sort-length-desc = Sort By Length Descending
reverse-lines = Reverse Line Order
remove-duplicate-lines = Remove Duplicate Lines
remove-consecutive-duplicates = Remove Consecutive Duplicate Lines
remove-empty-lines = Remove Empty Lines
remove-blank-lines = Remove Empty Lines (Including Blank Ones)
undo = Undo
redo = Redo
cut = Cut
copy = Copy
paste = Paste
select-all = Select All
menu-line-operations = Line Operations
menu-convert-case = Convert Case To
case-upper = UPPERCASE
case-lower = lowercase
case-title = Title Case
case-sentence = Sentence case
case-invert = iNVERT cASE
menu-comment = Comment
toggle-line-comment = Toggle Line Comment
toggle-block-comment = Toggle Block Comment
menu-blank-operations = Blank Operations
trim-trailing = Trim Trailing Space
trim-leading = Trim Leading Space
trim-both = Trim Leading and Trailing Space
eol-to-space = EOL to Space
tabs-to-spaces = Tabs to Spaces
leading-spaces-to-tabs = Leading Spaces to Tabs
all-spaces-to-tabs = All Spaces to Tabs
menu-insert = Insert
date-time-short = Date and Time (Short)
date-time-long = Date and Time (Long)
menu-multi-select = Multi-Select
select-next-occurrence = Select Next Occurrence
select-all-occurrences = Select All Occurrences
menu-auto-completion = Auto-Completion
complete-word = Complete Word
show-parameter-hint = Show Parameter Hint
column-editor = Column Editor…
read-only = Read-Only
column-mode = Column Mode
mime-base64-encode = Base64 Encode
mime-base64-encode-wrapped = Base64 Encode with Line Breaks
mime-base64-url-encode = Base64 Encode, URL-Safe
mime-base64-decode = Base64 Decode
mime-url-encode-component = URL Encode (Part of a URL)
mime-url-encode = URL Encode (Whole URL)
mime-url-decode = URL Decode
mime-qp-encode = Quoted-Printable Encode
mime-qp-decode = Quoted-Printable Decode
mime-html-encode = HTML Encode
mime-html-encode-all = HTML Encode, Including Accents and Symbols
mime-html-decode = HTML Decode
mime-saml-decode = SAML Decode
json-format = Format
json-minify = Minify
json-validate = Validate JSON
json-sort-keys = Sort JSON Keys
json-tree = JSON Tree
xml-pretty-print = Pretty Print
xml-pretty-print-attributes = Pretty Print, Attributes on New Lines
xml-linearize = Linearize
xml-check-syntax = Check Syntax
xml-check-on-save = Check Syntax When Saving
xml-validate = Validate
xml-validate-against = Validate Against…
xml-xsl-transform = XSL Transform…
xpath-panel = XPath Panel
copy-current-xpath = Copy Current XPath
xml-escape = Escape Special Characters
xml-unescape = Unescape Special Characters
close-tags-automatically = Close Tags Automatically
csvquery-panel = CsvQuery Panel
run-query = Run Query
copy-results = Copy Results
results-to-new-tab = Results to New Tab
menu-compare = Compare
menu-csvquery = CsvQuery
menu-mime-tools = MIME Tools
menu-json = JSON and JavaScript
menu-xml-tools = XML Tools
compare-two-views = Compare the Two Views
compare-previous-tab = Compare with Previous Tab
compare-with = Compare With
compare-to-saved = Compare to Last Save
compare-to-clipboard = Compare to Clipboard
compare-to-git = Compare to Last Commit (Git)
compare-selections = Compare Selections
next-difference = Next Difference
previous-difference = Previous Difference
show-comparison = Show Comparison
export-as-patch = Export as Patch
close-comparison = Close Comparison
compare-ignore-whitespace = Ignore Spaces and Tabs
compare-ignore-case = Ignore Case
compare-ignore-empty-lines = Ignore Empty Lines
compare-detect-moves = Detect Moved Lines

## Command names in the command palette (src/app/commands.rs)
long-line-marker = Long-Line Marker
next-tab = Next Tab
previous-tab = Previous Tab
single-window = Single Window
start-or-stop-recording = Start or Stop Recording
manage-plugins = Manage Plugins…
plugin-log = Plugin Log
open-plugins-folder = Open Plugins Folder
reload-plugins = Reload Plugins
install-sample-plugins = Install Sample Plugins
plugin-guide = Plugin Guide

## Plugins menu and plugin notices (src/app/plugins.rs)
install-plugin-from-file = Install Plugin from File
plugin-zip-filter = Plugin (.zip)
install-plugin-from-folder = Install Plugin from Folder
no-plugins-folder = There's no config folder to put plugins in.
plugin-removed = Removed { $name }; its folder is in the trash.
plugin-updated = Updated { $name }.
plugin-updated-version = Updated { $name } to { $version }.
plugin-installed = Installed { $name }.
plugin-installed-version = Installed { $name } { $version }.
plugins-running = { $count ->
    [one] { $count } plugin running
   *[other] { $count } plugins running
}
sample-install-failed = Couldn't install the sample plugin { $name } in { $folder }: { $error }
samples-already-installed = The sample plugins are already in the plugins folder.
samples-installed = Installed { $names }. Find them in the Plugins menu.
no-plugin-commands = No plugin commands

## Text tools (src/app/text_tools.rs)
tool-error-at = { $tool }: line { $line }, column { $column }: { $message }
tool-error = { $tool }: { $message }
tool-read-only = the document is read-only.
tool-ok-selection = { $result } (selection)
json-handled-as-js = This isn't strict JSON, so it was handled as JavaScript. Validate JSON shows where it differs.
json-valid = Valid JSON
xml-well-formed = The XML is well-formed
copied-text = Copied { $text }
xpath-no-element = the caret isn't inside an element, or an end tag doesn't match its start tag.
xpath-not-xml = this document isn't XML.
copy-xpath = Copy XPath
saved-not-well-formed = Saved, but { $name } isn't well-formed: line { $line }, column { $column }: { $message }
xpath-type-expression = Type an XPath expression, like //item[@id='1'].
xpath-not-well-formed = The document isn't well-formed XML (line { $line }, column { $column }: { $message })
xpath-error-at = { $message } (at character { $position })

## Compare (src/app/compare.rs)
compare-selection-title = { $name } (selection at line { $line })
compare-same = The two sides are the same.
compare-need-second-tab = Open a second tab to compare with, or pick one in Tools > Compare > Compare With.
compare-not-saved = This tab hasn't been saved yet.
compare-saved-title = { $name } (saved)
compare-read-saved-failed = Couldn't read the saved file: { $error }
compare-commit-title = { $name } (last commit)
compare-need-selections = Select two pieces of text: Ctrl+drag for a second selection, or select in each view.
clipboard = Clipboard
compare-clipboard-empty = The clipboard has no text to compare.
compare-closed-title = { $name } (closed)
compare-tab-title = Compare: { $left } | { $right }

## Languages folder (src/app/languages.rs)
make-folder-failed = Couldn't make { $path }: { $error }
write-failed = Couldn't write { $path }: { $error }

## Printing (src/app/printing.rs)
dialog-export-html = Export as HTML

## XML validation and XSL (src/app/xml_schema.rs)
dialog-validate-against = Validate Against Schema
filter-schema = XML Schema or DTD
filter-all-files = All files
the-doctype = the DOCTYPE
read-failed = Couldn't read { $path }: { $error }
validate-against = Validate against { $schema }
valid-against = The document is valid against { $schema }
xsl-transform = XSL Transform
filter-xsl = XSL stylesheet

## Preferences (src/prefs_ui.rs)
prefs-editing = Editing
prefs-display = Display
prefs-new-documents = New Documents
prefs-general = Files and Startup
preferences = Preferences
theme-colours-dots = Theme Colours…
prefs-font = Font
built-in-font = Built-in (Hack)
editor-font-label = Editor font:
use-built-in = Use Built-in
font-size-label = Size:
font-size-hint = Ctrl+0 returns to this size after zooming
font-find-label = Find:
font-name-hint = Font name
show-proportional-fonts = Show proportional fonts
looking-for-fonts = Looking for installed fonts…
no-fonts-match = No fonts match
proportional = proportional
prefs-show = Show
prefs-word-wrap = Word wrap
prefs-line-numbers = Line numbers
prefs-highlight-line = Highlight the current line
prefs-whitespace = Whitespace symbols
prefs-eol = End-of-line symbols
prefs-invisible = Invisible characters (control characters, unusual spaces, zero-width characters)
prefs-indent-guides = Indent guides
prefs-edge = Long-line marker at column
prefs-indentation = Indentation
prefs-tab-width = Tab width:
prefs-columns = columns
prefs-indent-tabs = Indent with tabs
prefs-indent-spaces = Indent with spaces
prefs-smart-indent = Smart indentation (keep the indent, and indent after an opening bracket)
prefs-typing = Typing
prefs-auto-close = Close brackets and quotes automatically
prefs-close-tags = Close HTML and XML tags automatically
prefs-word-completion = Suggest words from the document while typing
prefs-language-completion = Suggest the language's keywords and functions
prefs-parameter-hints = Show a function's parameters while typing a call
prefs-line-ending = Line ending
prefs-system-line-ending = This system's default ({ $ending })
prefs-startup = Startup and exit
prefs-restore-session = Reopen the files that were open last time
prefs-remember-unsaved = Quit without asking about unsaved changes, and bring them back next time
prefs-single-instance = Open files in the running Ettin Edit window
prefs-files = Files
prefs-auto-reload = Reload files changed on disk automatically when they have no unsaved changes
prefs-xml-check = Check that XML is well-formed each time it's saved
prefs-printing = Printing
prefs-print-line-numbers = Print line numbers
prefs-print-colours = Print syntax colours (darkened to read well on paper)
prefs-updates = Updates
prefs-check-updates = Check for a new version of Ettin Edit once a week
prefs-plugin-programs = Programs that run plugins
prefs-plugin-path-hint = Leave empty to find them on the PATH.
prefs-plugin-reload-hint = Changes apply when plugins are reloaded.
prefs-interface-language = Interface language
prefs-system-language = Same as the system ({ $language })
open-translations-folder = Open Translations Folder
reload-translations = Reload
prefs-translations-hint = Add a language or change a translation by putting a .ftl file in the translations folder. See Translations in the user manual.

## Translations (src/i18n.rs)
no-config-folder = There's no config folder to keep translations in.

## Find bar, results, Go to Line, Find in Files (src/search_ui.rs)
toggle-replace = Toggle replace
find-hint = Find
match-case = Match case
whole-word = Whole word
regular-expression = Regular expression
extended-search = Extended: \n, \r, \t, \0, \xNN and \uNNNN mean those characters
extended-search-short = Extended (\n, \t, \xNN)
match-of-total = { $current } of { $total }
matches-found = { $total } found
find-status = { $count }, { $message }
previous-match = Previous match (Shift+F3)
next-match = Next match (F3)
wrap-around = Wrap around
mark-all-bookmark = Mark All and Bookmark Lines
close-esc = Close (Esc)
replace-hint-regex = Replace  ($1 or \1 for groups)
replace-hint-extended = Replace  (\n, \t, \xNN allowed)
replace = Replace
replace-hover = Replace this match and find the next
replace-all = Replace All
in-selection = In selection
search-stopped = stopped
searched-files = { $count ->
    [one] searched { $count } file
   *[other] searched { $count } files
}
results-hits = { $count ->
    [one] { $count }{ $plus } hit
   *[other] { $count }{ $plus } hits
}
results-in-files = { $count ->
    [one] { $found } in { $count } file
   *[other] { $found } in { $count } files
}
results-in-documents = { $count ->
    [one] { $found } in { $count } document
   *[other] { $found } in { $count } documents
}
stop = Stop
close-results = Close results
collapse-all = Collapse All
expand-all = Expand All
go-to-line-title = Go to Line
go-to-line-prompt = Line number (1–{ $lines }), or line:column
enter-line-number = Enter a line number
go = Go
find-in-files-heading = Find in Files
find-what = Find what
folder = Folder
browse = Browse…
search-in-folder = Search in folder
file-filter-hint = *.rs *.toml   (empty for all files, !name to skip)
file-filter-hover = File name patterns separated by spaces or semicolons. Start one with ! to leave those files out.
include-subfolders = Include subfolders
include-hidden = Include hidden files and folders
find-all = Find All
enter-search-text = Enter something to search for.
folder-missing = That folder doesn't exist.
results-problems = { $count ->
    [one] { $count }{ $plus } problem
   *[other] { $count }{ $plus } problems
}

## Rename, Move to Trash, Summary, file changed on disk (src/file_ui.rs)
rename-file-title = Rename File
new-name = New name
rename = Rename
move-to-trash-title = Move to Trash
move-to-trash-question = { $windows ->
    [yes] Move "{ $name }" to the Recycle Bin and close its tab? You can restore it from the Recycle Bin.
   *[no] Move "{ $name }" to the trash and close its tab? You can restore it from the trash.
}
summary-title = Summary
summary-file = File
summary-created = Created
summary-modified = Modified
summary-characters = Characters (without line endings)
summary-words = Words
summary-lines = Lines
summary-size = Size when saved
summary-bytes = { $count ->
    [one] { $bytes } byte
   *[other] { $bytes } bytes
}
summary-cant-encode = can't be saved in this encoding
summary-selection = Selection
summary-selection-size = { $line_count ->
    [one] { $characters } characters on { $lines } line
   *[other] { $characters } characters on { $lines } lines
}
disk-changed = This file was changed by another program.
disk-changed-unsaved = This file was changed by another program, and you have unsaved changes here. Reloading replaces them (Undo brings them back).
disk-deleted = This file was deleted or moved by another program.
reload = Reload
keep-my-version = Keep My Version
keep-open = Keep Open

## Folder panel (src/folder_ui.rs)
no-folder-open = No folder is open.
close-folder-panel = Close the folder panel
refresh-folder = Read the folder again
collapse-folders = Collapse all folders
show-hidden-files = Show hidden files
open = Open
find-in-files-here = Find in Files Here…
copy-path = Copy Path
folder-truncated = Only the first { $count } entries are shown

## Function list (src/function_list_ui.rs)
show-caret-symbol = Show where the caret is
filter-hint = Filter
reading-document = Reading the document…
nothing-found-in-document = Nothing found in this document.
line-number = Line { $line }

## Command palette (src/palette_ui.rs)
nothing-matches = Nothing matches

## Shortcut mapper (src/shortcut_ui.rs)
shortcut-reserved-clipboard = Ctrl+C, Ctrl+X and Ctrl+V are kept for copy, cut and paste
shortcut-mapper = Shortcut Mapper
command-or-shortcut = Command or shortcut
only-assigned = Only commands with a shortcut
shortcut-in-use = { $shortcut } is already used by "{ $command }".
move-it-here = Move It Here
select-command = Select a command to change its shortcut.
shortcut-fixed = "{ $command }" belongs to the text box and can't be changed.
press-shortcut = Press the new shortcut… (Esc cancels)
change-shortcut = Change…
set-shortcut = Set…
add-another-shortcut = Add Another…
remove = Remove
reset-to-default = Reset to Default
reset-all-shortcuts = Reset All Shortcuts
shortcut-rule = Shortcuts need Ctrl or Alt, unless they use a function key (F1–F12).
command = Command
shortcut = Shortcut

## Theme colours (src/style_ui.rs)
style-background = Background
style-text = Text
style-caret = Caret
style-selection = Selection
style-current-line = Current line
style-line-numbers = Line numbers
style-indent-guides = Indent guides
style-whitespace = Whitespace symbols
style-search-matches = Search matches
style-accent = Accent (tabs, buttons, folds)
unset-colour = Don't set this colour
set-colour = Set
choose-colour = Choose a colour
theme-read-failed = Couldn't read the theme file: { $error }
theme-colours = Theme Colours
theme-label = Theme:
discard-changes = Discard Changes
save-as-new-theme = Save As New Theme…
new-theme-name = New theme name:
new-theme-name-hint = Name for the new theme
theme-name-taken = A theme has this name
theme-colours-hint = Changes show at once. Save keeps them; closing this window without saving drops them.
style-editor = Editor
style-syntax = Syntax
rule-filter-hint = Rule name or scope, like string or keyword
rules-hint = Each rule colours the text whose scope matches. Separate several scopes with commas.
rule-name = Name
rule-scope = Scope
rule-style = Style
no-name = (no name)
delete-rule = Delete this rule
add-rule = Add Rule
new-rule = New rule

## Notices, JSON tree, XPath panel (src/text_tools_ui.rs)
go-to = Go to
dismiss = Dismiss
json-too-large = This document is too large to show as a tree.
close-json-tree = Close the JSON tree
json-filter-hint = Filter keys and values
json-not-valid = Not valid JSON
line-column-message = Line { $line }, column { $column }: { $message }
go-to-error = Go to Error
no-matches = No matches
first-matches = First { $count } matches
copy-key = Copy Key
copy-value = Copy Value
expand-all-inside = Expand All Inside
close-xpath-panel = Close the XPath panel
evaluate = Evaluate
recent = Recent
recent-expressions = Recent expressions
result-label = Result:
xpath-nodes = { $count ->
    [one] { $count } node
   *[other] { $count } nodes
}
xpath-nodes-stale = { $count ->
    [one] { $count } node (the document changed since; evaluate again to refresh)
   *[other] { $count } nodes (the document changed since; evaluate again to refresh)
}

## Column editor and macros (src/tools_ui.rs)
column-editor-title = Column Editor
column-editor-hint = Inserts at every caret, or at the caret's column on each line below it.
hex-offset = Offset 0x{ $hex } ({ $dec })
hex-selected = { $count } bytes selected
hex-size = { $count } bytes in all
hex-read-only = This tab is read-only.
hex-cant-encode = "{ $char }" can't be written in { $encoding }.
hex-paste-hint = Paste hex digits, like 48 65 6C.
hex-invalid = Not changed: those bytes aren't valid { $encoding } text.
hex-round-trip = Not changed: the file would save differently (check the line endings).
char-search-hint = Search: é, U+00E9, 233 or a name
char-panel-hint = Double-click a character to insert it. Right-click to copy.
char-insert-character = Insert the character
char-insert-html = Insert the HTML entity
char-insert-hex = Insert the code (U+…)
char-insert-decimal = Insert the decimal code
char-col-char = Char
char-col-dec = Dec
char-col-hex = Hex
char-col-html = HTML
char-col-name = Name
char-copy-character = Copy Character
char-copy-html = Copy HTML Entity
char-copy-code = Copy Code (U+…)
text-to-insert = Text to insert
numbers-to-insert = Numbers to insert
initial-number = Initial number
increase-by = Increase by
repeat = Repeat
leading-zeros = Leading zeros
format-dec = Dec
format-hex = Hex
format-oct = Oct
format-bin = Bin
insert = Insert
must-be-whole-number = { $field } must be a whole number
run-macro-title = Run a Macro Multiple Times
run = Run
times = times
run-to-end = Run until the end of the file
record-macro-first = Record a macro first.
save-macro-title = Save Current Macro

## Comparison view (src/compare_ui.rs)
no-differences = No differences
lines-changed = { $count } changed
lines-added = { $count } added
lines-removed = { $count } removed
lines-moved = { $count } moved
compare-lines-summary = Lines: { $parts }
first = First
first-difference = First difference
previous-difference-hover = Previous difference (Shift+F7)
next-difference-hover = Next difference (F7)
last = Last
last-difference = Last difference
identical = Identical
difference-of = Difference { $current } of { $total }
differences-count = { $count ->
    [one] { $count } difference
   *[other] { $count } differences
}
compare-approximate = (approximate: the files are very different)
close-comparison-hover = Close the comparison
export-patch = Export Patch
export-patch-hover = Open the differences as a unified diff in a new tab
swap-sides = Swap Sides
options = Options
line-count = { $count ->
    [one] { $count } line
   *[other] { $count } lines
}

## CSV column types and delimiters (src/tools/csv.rs)
column-type-text = Text
column-type-integer = Integer
column-type-decimal = Decimal
delimiter-comma = Comma
delimiter-semicolon = Semicolon
delimiter-tab = Tab
delimiter-pipe = Pipe
delimiter-space = Space

## CsvQuery panel (src/csv_query.rs)
query-engine-stopped = The query engine stopped. Try again.
csv-shape = { $rows } rows × { $columns } columns
csv-this-hover = The current tab is the table `this`; other tabs are tables named after their files.
csv-open-file = Open a CSV or TSV file to query it.
close-csvquery = Close CsvQuery
tables = Tables
click-to-insert-name = Click to insert a name
no-queries-yet = No queries yet
run-query-hover = Run the query (Ctrl+Enter)
results = Results
columns = Columns
csv-tables-list = Tables: this, { $names }
csv-rows-changed = { $count ->
    [one] { $count } row changed in the query copy (the file is unchanged)
   *[other] { $count } rows changed in the query copy (the file is unchanged)
}
csv-first-rows = First { $rows } rows · { $ms } ms
csv-rows = { $count ->
    [one] { $rows } row · { $ms } ms
   *[other] { $rows } rows · { $ms } ms
}
open-in-new-tab = Open in New Tab
open-in-new-tab-hover = Write all result rows to a new tab as CSV
copy-results-hover = Copy the results, tab-separated for pasting into a spreadsheet
click-to-sort = Click to sort
copy-row = Copy Row
csv-run-for-columns = Run a query to see the columns of this tab.
csv-column = Column
csv-type = Type
csv-filled = Filled
csv-empty = Empty
csv-distinct = Distinct
csv-min = Min
csv-max = Max
csv-average = Average
csv-type-hover = How the column is stored and compared. Auto picks the narrowest type that fits every value.
auto-with = Auto ({ $detected })
delimiter = Delimiter
auto-quotes = Auto quotes
quotes-char = Quotes { $char }
no-quotes = No quotes
quote-character = Quote character
header-row = Header row
header-row-hover = The first row names the columns
reset = Reset
reset-format-hover = Go back to the detected format

## Update check (src/update.rs)
update-bad-json = The release list didn't make sense: { $error }
update-no-version = The release list has no version
update-not-found = No published release was found (HTTP 404). Update checks only work while the Ettin Edit repository is public.
update-http-error = GitHub answered with HTTP { $code }.
update-unreachable = Couldn't reach GitHub: { $error }
update-read-failed = Couldn't read GitHub's answer: { $error }
check-for-updates-title = Check for Updates
update-asking = Asking GitHub for the latest release…
update-available = Ettin Edit { $version } is available.
update-current = You have version { $version }.
open-download-page = Open Download Page
dont-check-automatically = Don't Check Automatically
update-latest = You have the latest version, { $version }.
update-newer-local = You have version { $version }; the newest release is { $latest }.
update-failed = Couldn't check for updates.
update-now = Update Now
update-release-notes = Release Notes
later = Later
restart-now = Restart Now
install-and-restart = Install and Restart
self-update-how-arch = Downloads the Arch package and installs it with pacman. Your system asks for your password.
self-update-how-deb = Downloads the Debian package and installs it with apt. Your system asks for your password.
self-update-how-portable = Downloads the new version and replaces this copy's program file.
self-update-how-setup = Downloads the installer. Ettin Edit then closes, installs the update and starts again.
self-update-no-exe = Couldn't tell where Ettin Edit is installed, so it can't update itself.
self-update-no-build = There's no ready-made build for this computer, so it can't update itself.
self-update-source-build = This copy was built from source; update it with git and cargo.
self-update-read-only = Ettin Edit can't write to { $path }, so it can't update itself there.
self-update-no-file = This release has no file for this kind of install; download it from the release page.
self-update-no-checksum = The release lists no checksum for { $file }, so it can't be checked. Download it from the release page instead.
self-update-save-failed = Couldn't save the download: { $error }
self-update-download-failed = Couldn't download the update: { $error }
self-update-cancelled = The download was cancelled.
self-update-checksum-mismatch = { $file } didn't match its checksum, so it wasn't installed.
self-update-install-failed = The install failed: { $error }
self-update-not-authorised = The install needs an administrator password, and none was given.
self-update-unpack-failed = Couldn't unpack the update: { $error }
self-update-missing-file = { $file } isn't in the download
self-update-replace-failed = Couldn't replace the program file: { $error }
self-update-downloading = Downloading { $done } of { $total } MB
self-update-installing = Installing…
self-update-password = Your system asks for an administrator password.
self-update-installed = Ettin Edit { $version } is installed. Restart to start using it.
self-update-ready = Ettin Edit { $version } is downloaded. Ettin Edit closes, installs it and starts again. Choose Later to install it when you next close Ettin Edit.
self-update-run-in-terminal = Run this in a terminal to finish the update:
self-update-failed = Couldn't update Ettin Edit.

## User manual window (src/manual.rs)
back = Back
forward = Forward
search-manual = Search the manual
open-on-web = Open on the Web
web-manual-failed = Couldn't open the web manual: { $error }
open-url-failed = Couldn't open { $url }: { $error }
copy-to-clipboard = Copy this to the clipboard

## Encoding groups (src/encoding.rs)
enc-western-european = Western European
enc-central-european = Central European
enc-cyrillic = Cyrillic
enc-greek = Greek
enc-turkish = Turkish
enc-baltic = Baltic
enc-nordic = Nordic
enc-hebrew = Hebrew
enc-arabic = Arabic
enc-vietnamese = Vietnamese
enc-thai = Thai
enc-romanian = Romanian
enc-chinese = Chinese
enc-japanese = Japanese
enc-korean = Korean

## Files, folders, terminal and Git (src/system.rs)
explorer-failed = Couldn't open Explorer: { $error }
no-parent-folder = { $path } has no folder
terminal-failed = Couldn't open a terminal: { $error }
no-terminal = Couldn't find a terminal. Set the TERMINAL environment variable to your terminal's command.
git-missing = Git isn't installed (or isn't on the PATH).
git-failed = Couldn't run git: { $error }
not-in-git = { $path } isn't in a Git repository.
not-in-repository = { $path } isn't in the repository at { $root }.
git-not-committed = This file isn't in the last commit yet.
git-no-commits = The repository has no commits yet.

## Themes (src/theme.rs)
themes-write-failed = Couldn't write the built-in themes to { $path }: { $error }
theme-file-problem = The theme file { $path } has a problem, so the original { $name } theme is used: { $error }
theme-skipped = Skipped the theme { $path }: { $error }

## Fonts (src/fonts.rs)
font-unusable = { $path } isn't a font Ettin Edit can use.
font-read-failed = Couldn't read the editor font { $path }: { $error }

## Editor (src/editor/view.rs)
completion-keyword = keyword
completion-function = function

## Manage Plugins and plugin log (src/plugins/ui.rs)
plugins-warning = Plugins run as separate programs with your access to files and the network. Only run plugins you trust.
plugins-installed-tab = Installed ({ $count })
plugins-available-count = Available ({ $count })
plugins-available = Available
install-from-folder = Install from Folder…
install-from-folder-hover = Copy a plugin's folder (one with a plugin.json) into the plugins folder
install-from-file = Install from File…
install-from-file-hover = Install a plugin from a .zip file
remove-plugin-title = Remove “{ $name }”?
remove-plugin-hint = Its folder moves to the trash, so you can get it back from there.
no-plugins-yet = No plugins yet.
no-plugins-hint = Install the samples to see how plugins work, find plugins under Available, or put a plugin's folder in the plugins folder.
found-on-path = { $program } (found on PATH)
reload-plugins-hover = Stop every plugin, look for plugins again and start them
plugin-filter-hint = Name, author or description
refresh = Refresh
refresh-index-hover = Read the plugin index again
reading-plugin-index = Reading the plugin index…
try-again = Try Again
index-empty = The plugin index lists no plugins.
no-plugins-match = No plugins match the filter.
plugin-index = Index
plugin-index-hover = The address of the JSON list of plugins. Press Enter to read it.
default = Default
default-index-hover = Use Ettin Edit's own plugin index
needs-version = Needs Ettin Edit { $version }
update-to = Update to { $version }
installed-no-version = Installed: no version
installed-version = Installed: { $version }
installed = Installed
install = Install
by-author = By { $author }
home-page = Home page
plugin-toggle-hover = Turn the plugin on or off
plugin-running = Running
plugin-stopped = Stopped
plugin-off = Off
plugin-needs-permission = Waiting for permission
plugin-not-running = Not running
update-plugin-hover = Download the new version from the plugin index; the old one moves to the trash
restart = Restart
allow-dots = Allow…
start = Start
open-folder = Open Folder
remove-dots = Remove…
remove-plugin-hover = Move the plugin's folder to the trash
trust-changes = Don't ask when it changes
trust-changes-hover = For a plugin you're writing: run it after each change without asking again
clear = Clear
copy-all = Copy All
plugin-log-hint = Messages, print() and console.log() output from plugins
nothing-yet = Nothing yet.
allow-plugin-title = Allow “{ $name }” to run?
plugin-changed = This plugin changed since you allowed it.
allow-plugin-warning = Plugins run as separate programs with your user account. This one could read and change your open documents and your files, and use the network. Only allow plugins you trust.
author = Author
runs = Runs
allow = Allow
dont-allow = Don't Allow
dont-allow-hover = Turn the plugin off. You can turn it on in Manage Plugins.

## Installing plugins (src/plugins/store.rs)
index-bad-json = The plugin index isn't valid JSON: { $error }
index-no-list = The plugin index has no "plugins" list
zip-unsafe-name = The zip has an unsafe file name, so nothing was installed: { $name }
zip-no-manifest = The zip has no { $file }, either at the top or inside a single folder.
zip-unreadable = That isn't a zip file Ettin Edit can read: { $error }
zip-too-many-files = The zip has more than { $count } files.
zip-damaged = The zip is damaged: { $error }
zip-has-link = The zip has a link, which plugins can't contain: { $name }
zip-encrypted = The zip is password-protected.
zip-too-big = The zip unpacks to more than 256 MB.
checksum-mismatch = The download doesn't match its SHA-256 checksum (expected { $expected }, got { $got }). It may be damaged or changed; nothing was installed.
already-in-plugins-folder = That folder is already in the plugins folder.
copy-failed = Couldn't copy { $path }: { $error }
not-a-plugin = This isn't a plugin Ettin Edit can run: { $error }
plugin-trash-old-failed = Couldn't move the installed copy of { $id } to the trash: { $error }
plugin-move-failed = Couldn't move the plugin into place: { $error }
index-unreachable = The plugin index couldn't be reached ({ $error }). Check the address and that it's public:
    { $url }
installing = Installing { $name }…

## Running plugins (src/plugins/mod.rs)
plugin-started = Started with { $program } { $script }
runtime-missing = { $runtime } isn't installed (couldn't find "{ $program }"). Install it or set its path in Plugins > Manage Plugins.
plugin-start-failed = Couldn't start { $program }: { $error }
plugin-exited = It stopped running.
plugin-exit-code = It stopped with exit code { $code }. See the plugin log.
plugin-died = It stopped. See the plugin log.
plugin-stopped-notice = The plugin { $name } stopped. See Plugins > Plugin Log.

## Shortcut names (src/plugins/shortcut.rs)
unknown-modifier = Unknown modifier "{ $modifier }" in "{ $shortcut }"
unknown-key = Unknown key "{ $key }" in "{ $shortcut }"
shortcut-needs-modifier = "{ $shortcut }" needs Ctrl or Alt, or it would get in the way of typing
shortcut-reserved = "{ $shortcut }" is kept for cut, copy and paste

## Completion lists (src/lang_api.rs)
api-file-skipped = Skipped the API file { $name }: { $error }

## Language files (src/highlight.rs)
language-file-skipped = Skipped the language file { $file }: { $error }

## Plugin panels (src/plugins/panel.rs)
close-panel = Close panel

## Shortcut settings (src/actions.rs)
shortcut-settings-error = Shortcut settings: { $error }

## Toolbar (src/ribbon*.rs, src/app/ribbon.rs)
apply = Apply
customise-toolbar = Customise Toolbar
customise-toolbar-dots = Customise Toolbar…
toolbar-show = Toolbar
toolbar-collapse-menu = Collapse Toolbar
toolbar-collapse = Collapse the toolbar
toolbar-expand = Show the toolbar's buttons
toolbar-hide = Hide the toolbar
menu-bar-show = Menu Bar
menu-bar-hide = Hide the menu bar
menu-bar-show-again = Show the menu bar
menu-bar-hidden-notice = The menu bar is hidden. Tap Alt to show it for a moment, or press { $shortcut } to bring it back.
menu-bar-hidden-notice-alt = The menu bar is hidden. Tap Alt to show it, then choose View > Menu Bar to bring it back.
toolbar-empty = No buttons here yet. Right-click to customise the toolbar.
toolbar-remove-button = Remove from Toolbar
toolbar-read-failed = Couldn't read the toolbar layout { $path }: { $error }. Using the default toolbar.
toolbar-save-failed = Couldn't save the toolbar layout to { $path }: { $error }
toolbar-commands = Commands
toolbar-filter-hint = Filter commands
toolbar-layout = Toolbar
toolbar-drag-hint = Drag commands here, or drag tabs, groups and buttons to move them.
toolbar-add = Add ›
toolbar-add-tip = Add the picked command after the selection on the right
toolbar-remove = ‹ Remove
toolbar-move-up = Move up
toolbar-move-down = Move down
toolbar-new-tab = New Tab
toolbar-new-group = New Group
toolbar-new-tab-name = New Tab
toolbar-new-group-name = New Group
toolbar-no-tabs = There are no tabs. Add one with New Tab.
toolbar-tab-visible = Show this tab
toolbar-unavailable = This command isn't available now (its plugin may not be running). The button comes back when it is.
toolbar-select-hint = Select a tab, group or button to change it.
toolbar-tab-name = Tab name:
toolbar-group-name = Group name:
toolbar-label = Label:
toolbar-size = Size:
toolbar-size-large = Large
toolbar-size-small = Small
toolbar-icon = Icon:
toolbar-choose-icon = Choose an icon
toolbar-change-icon = Change…
toolbar-default-icon = Use the command's icon
toolbar-icon-filter-hint = Filter icons
toolbar-icon-credit = Icons: Phosphor (MIT licence)
toolbar-reset = Reset to Defaults
toolbar-tab-home = Home
toolbar-tab-edit = Edit
toolbar-tab-view = View
toolbar-tab-tools = Tools
toolbar-group-file = File
toolbar-group-clipboard = Clipboard
toolbar-group-history = Undo
toolbar-group-find = Find
toolbar-group-tabs = Tabs
toolbar-group-help = Help
toolbar-group-lines = Lines
toolbar-group-sort = Sort
toolbar-group-case = Case
toolbar-group-comment = Comment
toolbar-group-blanks = Blanks
toolbar-group-selection = Selection
toolbar-group-show = Show
toolbar-group-zoom = Zoom
toolbar-group-folding = Folding
toolbar-group-panels = Panels
toolbar-group-window = Window
toolbar-group-compare = Compare
toolbar-group-json = JSON
toolbar-group-xml = XML
toolbar-group-data = Data
toolbar-group-macros = Macros
toolbar-group-customise = Customise
