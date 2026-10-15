; Inno Setup script for the Windows installer.
; Built by .github/workflows/release.yml:  iscc /DAppVersion=1.0.0 /DSourceExe=path\to\ettin-edit.exe ettin-edit.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\release\ettin-edit.exe"
#endif

[Setup]
AppId={{6C1E7A52-3B7D-4E4B-9C55-2F0E4E7A1D10}
AppName=Ettin Edit
AppVersion={#AppVersion}
AppVerName=Ettin Edit {#AppVersion}
AppPublisher=Tharkis
AppPublisherURL=https://github.com/Tharkis/Ettin-Edit
AppSupportURL=https://github.com/Tharkis/Ettin-Edit/issues
AppUpdatesURL=https://github.com/Tharkis/Ettin-Edit/releases
DefaultDirName={autopf}\Ettin Edit
DefaultGroupName=Ettin Edit
DisableProgramGroupPage=yes
; Installs for the current user without asking for admin rights; the first
; page offers installing for everyone instead.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputBaseFilename=ettin-edit-{#AppVersion}-windows-x86_64-setup
OutputDir=.
SetupIconFile=..\..\assets\icons\ettin-edit.ico
UninstallDisplayIcon={app}\ettin-edit.exe
UninstallDisplayName=Ettin Edit
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
ChangesEnvironment=yes

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "contextmenu"; Description: "Add ""Edit with Ettin Edit"" to the right-click menu of every file"; GroupDescription: "Explorer:"
Name: "openwith"; Description: "List Ettin Edit under ""Open with"" for text and code files"; GroupDescription: "Explorer:"
Name: "addtopath"; Description: "Add Ettin Edit to the PATH, so ettin-edit works in a terminal"; GroupDescription: "Command line:"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "ettin-edit.exe"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\icons\ettin-edit.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Ettin Edit"; Filename: "{app}\ettin-edit.exe"
Name: "{autodesktop}\Ettin Edit"; Filename: "{app}\ettin-edit.exe"; Tasks: desktopicon

[Registry]
; The program itself, so "Open with" can list it.
Root: HKA; Subkey: "Software\Classes\Applications\ettin-edit.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "Ettin Edit"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\ettin-edit.exe\DefaultIcon"; ValueType: string; ValueData: "{app}\ettin-edit.exe,0"
Root: HKA; Subkey: "Software\Classes\Applications\ettin-edit.exe\shell\open\command"; ValueType: string; ValueData: """{app}\ettin-edit.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\EttinEdit.Document"; ValueType: string; ValueData: "Text document"; Flags: uninsdeletekey; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\EttinEdit.Document\DefaultIcon"; ValueType: string; ValueData: "{app}\ettin-edit.exe,0"; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\EttinEdit.Document\shell\open\command"; ValueType: string; ValueData: """{app}\ettin-edit.exe"" ""%1"""; Tasks: openwith
; "Edit with Ettin Edit" on every file.
Root: HKA; Subkey: "Software\Classes\*\shell\EttinEdit"; ValueType: string; ValueData: "Edit with Ettin Edit"; Flags: uninsdeletekey; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\*\shell\EttinEdit"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\ettin-edit.exe,0"; Tasks: contextmenu
Root: HKA; Subkey: "Software\Classes\*\shell\EttinEdit\command"; ValueType: string; ValueData: """{app}\ettin-edit.exe"" ""%1"""; Tasks: contextmenu
; Text and code file types offer Ettin Edit under "Open with" (Windows keeps
; the current default app; people pick Ettin Edit there to make it the default).
Root: HKA; Subkey: "Software\Classes\.txt\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.log\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.ini\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.cfg\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.conf\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.toml\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.yaml\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.yml\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.json\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.xml\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.xsd\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.xsl\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.csv\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.tsv\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.hl7\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.html\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.css\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.js\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.py\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.rs\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.c\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.h\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.cpp\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.cs\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.java\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.go\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.sql\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.sh\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.ps1\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.bat\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith
Root: HKA; Subkey: "Software\Classes\.cmd\OpenWithProgids"; ValueType: string; ValueName: "EttinEdit.Document"; ValueData: ""; Flags: uninsdeletevalue; Tasks: openwith

[Run]
Filename: "{app}\ettin-edit.exe"; Description: "Start Ettin Edit"; Flags: nowait postinstall skipifsilent

[Code]
const
  EnvKeyUser = 'Environment';
  EnvKeyMachine = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';

function EnvRoot: Integer;
begin
  if IsAdminInstallMode then Result := HKEY_LOCAL_MACHINE else Result := HKEY_CURRENT_USER;
end;

function EnvKey: String;
begin
  if IsAdminInstallMode then Result := EnvKeyMachine else Result := EnvKeyUser;
end;

procedure AddToPath(Dir: String);
var
  Paths: String;
begin
  if not RegQueryStringValue(EnvRoot, EnvKey, 'Path', Paths) then Paths := '';
  if Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';') > 0 then exit;
  if (Paths <> '') and (Copy(Paths, Length(Paths), 1) <> ';') then Paths := Paths + ';';
  RegWriteExpandStringValue(EnvRoot, EnvKey, 'Path', Paths + Dir);
end;

procedure RemoveFromPath(Dir: String);
var
  Paths, Padded: String;
  P: Integer;
begin
  if not RegQueryStringValue(EnvRoot, EnvKey, 'Path', Paths) then exit;
  Padded := ';' + Paths + ';';
  P := Pos(';' + Uppercase(Dir) + ';', Uppercase(Padded));
  if P = 0 then exit;
  Delete(Padded, P, Length(Dir) + 1);
  while (Length(Padded) > 0) and (Padded[1] = ';') do Delete(Padded, 1, 1);
  while (Length(Padded) > 0) and (Padded[Length(Padded)] = ';') do Delete(Padded, Length(Padded), 1);
  RegWriteExpandStringValue(EnvRoot, EnvKey, 'Path', Padded);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
