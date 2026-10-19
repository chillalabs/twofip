; The Windows installer (Inno Setup 6). Built by the "Windows release"
; workflow: iscc /DAppVersion=0.2.14 windows\2fip.iss
; Installs for the current user without admin rights by default, or for all
; users (Program Files) when chosen in the first dialog.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
; Never change the AppId: Windows uses it to find earlier versions to update.
AppId={{28150553-48AB-4A14-9BC3-E481F5F8A394}
AppName=2fip
AppVersion={#AppVersion}
AppVerName=2fip {#AppVersion}
AppPublisher=chillalabs
AppPublisherURL=https://chillalabs.github.io/twofip/
AppSupportURL=https://github.com/chillalabs/twofip/issues
AppUpdatesURL=https://github.com/chillalabs/twofip/releases
DefaultDirName={autopf}\2fip
DefaultGroupName=2fip
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE
SetupIconFile=..\res\windows\2fip.ico
UninstallDisplayIcon={app}\2fip.exe
UninstallDisplayName=2fip
OutputDir=..\dist
OutputBaseFilename=2fip-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Updating while 2fip runs: ask to close it first.
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\Portuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\2fip.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\2fip"; Filename: "{app}\2fip.exe"
Name: "{autodesktop}\2fip"; Filename: "{app}\2fip.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\2fip.exe"; Description: "{cm:LaunchProgram,2fip}"; Flags: nowait postinstall skipifsilent
