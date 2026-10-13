; CraftSuite installer (Inno Setup 6).
; Installs for the current user only, into %LOCALAPPDATA%\Programs\CraftSuite:
; no administrator rights, no UAC prompt, no PIN.
;
; Build: iscc /DAppVersion=0.1.0 installer\CraftSuite.iss   (expects the assembled folder in dist\CraftSuite)

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{4B7E2C1A-93D5-4F1E-8A62-0C5D7E9B3F21}
AppName=CraftSuite
AppVersion={#AppVersion}
AppVerName=CraftSuite {#AppVersion}
AppPublisher=CraftSuite
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\CraftSuite
DefaultGroupName=CraftSuite
DisableProgramGroupPage=yes
DisableDirPage=yes
DisableReadyPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir=..\dist
OutputBaseFilename=CraftSuite-Setup-{#AppVersion}
SetupIconFile=..\assets\craftsuite.ico
UninstallDisplayIcon={app}\CraftSuite.exe
UninstallDisplayName=CraftSuite (FilmCraft + EffectCraft)
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
ChangesAssociations=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "associate"; Description: "Projektdateien (.fcproj, .ecproj) mit CraftSuite öffnen"; GroupDescription: "Dateizuordnung:"

[Files]
Source: "..\dist\CraftSuite\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\CraftSuite"; Filename: "{app}\CraftSuite.exe"
Name: "{autoprograms}\CraftSuite\FilmCraft"; Filename: "{app}\FilmCraft\filmcraft.exe"; WorkingDir: "{app}\FilmCraft"
Name: "{autoprograms}\CraftSuite\EffectCraft"; Filename: "{app}\EffectCraft\effectcraft.exe"; WorkingDir: "{app}\EffectCraft"
Name: "{autodesktop}\CraftSuite"; Filename: "{app}\CraftSuite.exe"; Tasks: desktopicon

; The browser version (GitHub Pages, .github/workflows/web.yml).
[INI]
Filename: "{autoprograms}\CraftSuite\CraftSuite im Browser.url"; Section: "InternetShortcut"; Key: "URL"; String: "https://tonisagmeister11-design.github.io/CraftSuite/"

[UninstallDelete]
Type: files; Name: "{autoprograms}\CraftSuite\CraftSuite im Browser.url"

; Per-user file associations (HKCU, no admin): CraftSuite picks the right app from the extension.
[Registry]
Root: HKCU; Subkey: "Software\Classes\CraftSuite.Project"; ValueType: string; ValueName: ""; ValueData: "CraftSuite-Projekt"; Flags: uninsdeletekey; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\CraftSuite.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\CraftSuite.exe,0"; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\CraftSuite.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\CraftSuite.exe"" ""%1"""; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\.fcproj\OpenWithProgids"; ValueType: string; ValueName: "CraftSuite.Project"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\.ecproj\OpenWithProgids"; ValueType: string; ValueName: "CraftSuite.Project"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\.ecprojx\OpenWithProgids"; ValueType: string; ValueName: "CraftSuite.Project"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\.fcproj"; ValueType: string; ValueName: ""; ValueData: "CraftSuite.Project"; Flags: uninsdeletevalue; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\.ecproj"; ValueType: string; ValueName: ""; ValueData: "CraftSuite.Project"; Flags: uninsdeletevalue; Tasks: associate
Root: HKCU; Subkey: "Software\Classes\.ecprojx"; ValueType: string; ValueName: ""; ValueData: "CraftSuite.Project"; Flags: uninsdeletevalue; Tasks: associate

[Run]
Filename: "{app}\CraftSuite.exe"; Description: "{cm:LaunchProgram,CraftSuite}"; Flags: nowait postinstall skipifsilent
