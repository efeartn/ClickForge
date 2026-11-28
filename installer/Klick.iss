#ifndef AppVersion
  #define AppVersion "1.1.0"
#endif

#define AppName "Klick"
#define AppPublisher "efeartn"
#define AppURL "https://github.com/efeartn/Klick"
#define AppExeName "Klick.exe"

[Setup]
AppId={{B4E9A21F-6C3D-4B8E-9A12-7E5C4D2F8B01}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases
; Per-user install under Programs keeps in-app auto-updates writable without UAC.
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=Klick-Setup
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExeName}
UninstallDisplayName={#AppName}
Compression=lzma2/normal
SolidCompression=yes
WizardStyle=modern
CloseApplications=force
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "cleanlegacy"; Description: "Remove legacy ClickForge installation and shortcuts"; GroupDescription: "Cleanup:"

[Files]
Source: "..\dist\{#AppName}\Klick.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  LegacyDir, LegacyStartMenu, LegacyDesktopLnk: string;
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('cleanlegacy') then
  begin
    LegacyDir := ExpandConstant('{localappdata}\Programs\ClickForge');
    if DirExists(LegacyDir) then
      DelTree(LegacyDir, True, True, True);

    LegacyStartMenu := ExpandConstant('{userprograms}\ClickForge');
    if DirExists(LegacyStartMenu) then
      DelTree(LegacyStartMenu, True, True, True);

    LegacyDesktopLnk := ExpandConstant('{userdesktop}\ClickForge.lnk');
    if FileExists(LegacyDesktopLnk) then
      DeleteFile(LegacyDesktopLnk);

    if RegKeyExists(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\ClickForge') then
      RegDeleteKeyIncludingSubkeys(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\ClickForge');

    if RegKeyExists(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{8F6D7C4A-3A2B-4E1C-9D8A-1F2E3D4C5B6A}_is1') then
      RegDeleteKeyIncludingSubkeys(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{8F6D7C4A-3A2B-4E1C-9D8A-1F2E3D4C5B6A}_is1');
  end;
end;
