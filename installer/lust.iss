; Lust's Windows installer, built by Inno Setup (https://jrsoftware.org/isinfo.php) in
; .github/workflows/release.yml:  iscc /DVersion=2.0.0-alpha.38 installer\lust.iss
;
; It installs for the person running it (no administrator needed), in
; %LOCALAPPDATA%\Programs\Lust, and adds that folder to their PATH, so `lust` works in any new
; terminal. There lust update can replace lust.exe by itself. The uninstaller takes both away.

#ifndef Version
  #define Version "0.0.0"
#endif

[Setup]
; Stays the same for every version, so a new installer updates the Lust already there.
AppId={{6F1B7C2A-3D4E-4B8F-9A61-2C5D8E7F1A30}
AppName=Lust
AppVersion={#Version}
AppVerName=Lust {#Version}
AppPublisher=Rhyslos
AppPublisherURL=https://github.com/Rhyslos/Lust
AppSupportURL=https://github.com/Rhyslos/Lust/issues
AppUpdatesURL=https://github.com/Rhyslos/Lust/releases
DefaultDirName={localappdata}\Programs\Lust
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
PrivilegesRequired=lowest
ChangesEnvironment=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=lust-setup
SetupIconFile=..\assets\lust.ico
UninstallDisplayIcon={app}\lust.exe
UninstallDisplayName=Lust
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes

[Files]
Source: "..\target\release\lust.exe"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; What an earlier lust update left behind.
Type: files; Name: "{app}\lust.old.exe"
Type: files; Name: "{app}\lust.new.exe"

[UninstallDelete]
Type: files; Name: "{app}\lust.old.exe"
Type: files; Name: "{app}\lust.new.exe"

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NotOnPath(ExpandConstant('{app}'))

[Messages]
FinishedLabel=Lust is installed. Open a new terminal and type lust to start, or lust run yourfile.lst.%n%nBuilding real programs (lust build) also needs Rust: lust explains how to get it the first time.

[Code]
const
  EnvKey = 'Environment';

{ Whether a folder is already one of the user's PATH folders. }
function NotOnPath(Folder: string): Boolean;
var
  Path: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Folder) + ';', ';' + Uppercase(Path) + ';') = 0;
end;

{ Takes the Lust folder back out of the user's PATH when Lust is uninstalled. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Path, Folder: string;
  P: Integer;
begin
  if CurUninstallStep <> usPostUninstall then
    exit;
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
    exit;
  Folder := ExpandConstant('{app}');
  Path := ';' + Path + ';';
  P := Pos(';' + Uppercase(Folder) + ';', Uppercase(Path));
  if P = 0 then
    exit;
  Delete(Path, P, Length(Folder) + 1);
  { The ; added at each end. }
  Path := Copy(Path, 2, Length(Path) - 2);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path);
end;
