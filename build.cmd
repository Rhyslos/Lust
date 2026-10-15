@echo off
rem Builds lust.exe into the bin folder. Needs the .NET SDK (8 or newer): https://dotnet.microsoft.com/download
dotnet publish "%~dp0compiler" -c Release -r win-x64 -p:PublishSingleFile=true --self-contained false -o "%~dp0bin"
if errorlevel 1 (
  echo.
  echo Building lust.exe didn't work. Is the .NET SDK installed? Try: dotnet --version
  pause
  exit /b 1
)
echo.
echo Done: %~dp0bin\lust.exe
echo Try it: bin\lust run examples\hello.lust
pause
