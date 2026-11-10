#define AppName "OOTRR"
#define AppVersion GetEnv("OOTRR_VERSION")
#define RepoUrl "https://github.com/realazk/ootrr"

[Setup]
AppId={{6F3B1E0A-52C9-4D3E-9A41-0B7C8D2E5F17}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=OOTRR contributors
AppPublisherURL={#RepoUrl}
AppSupportURL={#RepoUrl}/issues
AppUpdatesURL={#RepoUrl}/releases
DefaultDirName={autopf}\OOTRR
DefaultGroupName=OOTRR
UninstallDisplayIcon={app}\ootrr.exe
UninstallDisplayName={#AppName} {#AppVersion}
LicenseFile=..\..\LICENSE
InfoBeforeFile=before.txt
OutputDir=..\..\release
OutputBaseFilename=OOTRR-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
DisableProgramGroupPage=no
SetupLogging=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "addpath"; Description: "Add OOTRR to the PATH so the ootrr command works in a terminal"; GroupDescription: "Command line:"; Flags: unchecked

[Files]
Source: "..\..\target\release\ootrr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\OOTRR"; Filename: "{app}\ootrr.exe"; Parameters: "serve"; WorkingDir: "{userdocs}"; Comment: "Start OOTRR and open it in your browser"
Name: "{group}\OOTRR on GitHub"; Filename: "{#RepoUrl}"
Name: "{group}\Uninstall OOTRR"; Filename: "{uninstallexe}"
Name: "{autodesktop}\OOTRR"; Filename: "{app}\ootrr.exe"; Parameters: "serve"; WorkingDir: "{userdocs}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addpath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\ootrr.exe"; Parameters: "serve"; WorkingDir: "{userdocs}"; Description: "Start OOTRR now"; Flags: postinstall nowait skipifsilent unchecked

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  Old: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Old) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(Old) + ';') = 0;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Old, App, Next: string;
  P: Integer;
begin
  if CurUninstallStep <> usPostUninstall then
    exit;
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Old) then
    exit;
  App := ExpandConstant('{app}');
  Next := ';' + Old + ';';
  P := Pos(';' + Uppercase(App) + ';', Uppercase(Next));
  if P = 0 then
    exit;
  Delete(Next, P, Length(App) + 1);
  if (Length(Next) > 0) and (Next[1] = ';') then
    Delete(Next, 1, 1);
  if (Length(Next) > 0) and (Next[Length(Next)] = ';') then
    Delete(Next, Length(Next), 1);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Next);
end;
