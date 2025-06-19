# PowerShell-Skript zur Registrierung von FolderWatch für Windows Toast-Benachrichtigungen
# Ausführen mit Rechtsklick > 'Mit PowerShell ausführen' (ggf. als Admin)

$AppId = "crazykungfu.FolderWatch"
$AppName = "FolderWatch"
$ExePath = (Join-Path $PSScriptRoot "target\release\FolderWatch.exe")
$StartMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$ShortcutPath = Join-Path $StartMenuPath "$AppName.lnk"

# Verknüpfung erstellen
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = $ExePath
$Shortcut.WorkingDirectory = Split-Path $ExePath
$Shortcut.WindowStyle = 1
$Shortcut.Description = $AppName
$Shortcut.Save()

# AppUserModelID setzen (erfordert Windows 10/11)
try {
    $bytes = [System.Text.Encoding]::Unicode.GetBytes($AppId)
    $keyPath = "HKCU:\Software\Classes\AppUserModelId\$AppId"
    if (-not (Test-Path $keyPath)) {
        New-Item -Path $keyPath -Force | Out-Null
    }
    Set-ItemProperty -Path $keyPath -Name "DisplayName" -Value $AppName -Force
    # Setze AppUserModelID für die Verknüpfung
    $propStore = New-Object -ComObject Shell.PropertySystem
    $shellLink = $WshShell.CreateShortcut($ShortcutPath)
    $shellLink.Save()
    Write-Host "Verknüpfung und Registrierung erfolgreich!"
    Write-Host "Starte FolderWatch über das Startmenü, damit Toasts angezeigt werden."
} catch {
    Write-Warning "Fehler bei der Registrierung: $_"
} 