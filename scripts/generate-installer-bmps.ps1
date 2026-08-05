Add-Type -AssemblyName System.Drawing

$iconPath = "D:/New folder/NovaVM/src-tauri/icons/novavm_icon.png"
if (-not (Test-Path $iconPath)) {
    Write-Host "Icon not found at $iconPath"
    exit 1
}

$src = [System.Drawing.Bitmap]::FromFile($iconPath)

# 1. Header BMP (150x57)
$hdr = New-Object System.Drawing.Bitmap 150, 57
$g1 = [System.Drawing.Graphics]::FromImage($hdr)
$g1.Clear([System.Drawing.Color]::FromArgb(15, 23, 42))
$g1.DrawImage($src, (150 - 48) / 2, (57 - 48) / 2, 48, 48)
$hdr.Save("D:/New folder/NovaVM/src-tauri/icons/header.bmp", [System.Drawing.Imaging.ImageFormat]::Bmp)
$g1.Dispose()
$hdr.Dispose()

# 2. Sidebar BMP (164x314)
$sdb = New-Object System.Drawing.Bitmap 164, 314
$g2 = [System.Drawing.Graphics]::FromImage($sdb)
$g2.Clear([System.Drawing.Color]::FromArgb(15, 23, 42))
$g2.DrawImage($src, (164 - 128) / 2, (314 - 128) / 2, 128, 128)
$sdb.Save("D:/New folder/NovaVM/src-tauri/icons/sidebar.bmp", [System.Drawing.Imaging.ImageFormat]::Bmp)
$g2.Dispose()
$sdb.Dispose()

$src.Dispose()
Write-Host "Header and Sidebar BMPs generated successfully."
