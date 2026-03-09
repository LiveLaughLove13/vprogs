Add-Type -AssemblyName System.Drawing

$iconPath = Join-Path $PSScriptRoot "..\icons\icon.ico"

$bmp = New-Object System.Drawing.Bitmap 32,32
$g = [System.Drawing.Graphics]::FromImage($bmp)
$color = [System.Drawing.Color]::FromArgb(74,158,255)
$g.Clear($color)

$iconHandle = $bmp.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($iconHandle)

$directory = Split-Path $iconPath
if (-not (Test-Path $directory)) {
    New-Item -ItemType Directory -Path $directory | Out-Null
}

$fs = New-Object System.IO.FileStream($iconPath, [System.IO.FileMode]::Create)
$icon.Save($fs)
$fs.Close()
$g.Dispose()
$bmp.Dispose()

