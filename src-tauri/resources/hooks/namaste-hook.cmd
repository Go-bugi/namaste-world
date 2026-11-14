@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "NAMASTE_PORT="
set "NAMASTE_TOKEN="
set "NAMASTE_ENDPOINT=%~dp0..\endpoint"
set "NAMASTE_CURL=%SystemRoot%\System32\curl.exe"
if not exist "%NAMASTE_ENDPOINT%" goto drain
for /f "usebackq eol=# tokens=1,* delims==" %%a in ("%NAMASTE_ENDPOINT%") do (
  if "%%a"=="NAMASTE_PORT" set "NAMASTE_PORT=%%b"
  if "%%a"=="NAMASTE_TOKEN" set "NAMASTE_TOKEN=%%b"
)
if not defined NAMASTE_PORT goto drain
if not defined NAMASTE_TOKEN goto drain
if not exist "%NAMASTE_CURL%" goto drain
"%NAMASTE_CURL%" -s -o NUL --connect-timeout 0.5 --max-time 1.3 --noproxy 127.0.0.1 -H "Content-Type: application/json" -H "X-Namaste-Token: %NAMASTE_TOKEN%" --data-binary @- "http://127.0.0.1:%NAMASTE_PORT%/hook/%~1/%~2" >NUL 2>NUL
exit /b 0
:drain
more >NUL 2>NUL
exit /b 0
