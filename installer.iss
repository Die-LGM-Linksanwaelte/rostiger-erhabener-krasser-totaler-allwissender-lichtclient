#ifndef MyAppVersion
#define MyAppVersion "0.2.1"
#endif

#ifndef MyArch
#define MyArch "x64"
#endif

[Setup]
AppName=R.E.K.T.A.L.
AppVersion={#MyAppVersion}
DefaultDirName={autopf}\R.E.K.T.A.L.
DefaultGroupName=R.E.K.T.A.L.
OutputDir=output_installer
OutputBaseFilename=Rektal-Setup-{#MyArch}-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
; Automatically detect and use the Windows system language
ShowLanguageDialog=auto

[Languages]
; Includes multiple standard language files; Inno Setup picks the matching system language automatically
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Files]
; Copies the compiled release binaries from the Windows build into the installation directory
Source: "windows_dist\rektal_client.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "windows_dist\rektal_kernel.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Creates user-friendly shortcuts in the Windows Start Menu
Name: "{group}\R.E.K.T.A.L. Client"; Filename: "{app}\rektal_client.exe"
Name: "{group}\R.E.K.T.A.L. Kernel"; Filename: "{app}\rektal_kernel.exe"
Name: "{group}\Uninstall R.E.K.T.A.L."; Filename: "{uninstallexe}"