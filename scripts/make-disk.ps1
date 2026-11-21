
# scripts/make-disk.ps1
# ---------------------------------------------------------------------------
# Creates a 64 MiB FAT32 disk image with the ESP layout required by UEFI:
#   \EFI\BOOT\BOOTX64.EFI   <- fallback boot path (UEFI Spec §3.5.1.1)
#
# Requires: VirtualBox 7.x (uses VBoxManage.exe for disk operations)
# Run from the workspace root:
#   powershell -ExecutionPolicy Bypass -File scripts\make-disk.ps1
# ---------------------------------------------------------------------------

param(
    [string]$EfiFile   = "target\x86_64-unknown-uefi\debug\bootloader.efi",
    [string]$ImageDir  = "build",
    [string]$ImageName = "esp.img"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$VBoxManage = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"

# ---- 1. Verify source EFI binary exists ------------------------------------
if (-not (Test-Path $EfiFile)) {
    Write-Error "EFI binary not found: $EfiFile`nRun: cargo build -p bootloader --target x86_64-unknown-uefi"
    exit 1
}

# ---- 2. Create build directory ---------------------------------------------
New-Item -ItemType Directory -Force -Path $ImageDir | Out-Null
$ImagePath = Join-Path $ImageDir $ImageName

# ---- 3. Create a raw 64 MiB disk image ------------------------------------
# VBoxManage createmedium creates a VDI; we need a raw image.
# We use PowerShell to write a zeroed file of exactly 64 MiB.
Write-Host "[1/5] Creating blank 64 MiB disk image: $ImagePath"
$sizeBytes = 64 * 1024 * 1024
$fs = [System.IO.File]::Create($ImagePath)
$fs.SetLength($sizeBytes)
$fs.Close()

# ---- 4. Format as FAT32 using VBoxManage formatmedium ----------------------
# We use a temporary VDI as an intermediate because VBoxManage's formatmedium
# only works on VDI/VMDK/VHD. Instead, we write FAT32 structure directly
# using the .NET FileStream + a minimal FAT32 BPB.
#
# SIMPLER APPROACH: Use PowerShell + diskpart for the host Windows machine.
# diskpart can format a raw image attached as a virtual disk.
#
# We use a diskpart script:

Write-Host "[2/5] Creating ESP FAT32 partition structure via diskpart..."

$diskpartScript = @"
select vdisk file="$((Resolve-Path $ImagePath).Path)"
attach vdisk
create partition primary
format fs=fat32 quick label="ESP"
assign
detach vdisk
"@

$scriptFile = "$env:TEMP\myos_diskpart.txt"
$diskpartScript | Set-Content -Encoding ASCII $scriptFile

# diskpart needs elevation. We pass it via Start-Process with -Verb RunAs.
$proc = Start-Process "diskpart.exe" -ArgumentList "/s `"$scriptFile`"" -Verb RunAs -Wait -PassThru
if ($proc.ExitCode -ne 0) {
    Write-Error "diskpart failed with exit code $($proc.ExitCode)"
    exit 1
}

# ---- 5. Mount the VHD and copy EFI files -----------------------------------
Write-Host "[3/5] Mounting image and copying EFI binary..."

# Re-attach to find the drive letter
$attachScript = @"
select vdisk file="$((Resolve-Path $ImagePath).Path)"
attach vdisk
assign
"@
$attachScript | Set-Content -Encoding ASCII $scriptFile
Start-Process "diskpart.exe" -ArgumentList "/s `"$scriptFile`"" -Verb RunAs -Wait

Start-Sleep -Seconds 2  # Wait for Windows to assign drive letter

# Find the newly mounted drive (FAT32 labelled "ESP")
$drive = Get-Volume -FileSystemLabel "ESP" -ErrorAction SilentlyContinue |
         Select-Object -First 1 -ExpandProperty DriveLetter

if (-not $drive) {
    Write-Error "Could not find mounted ESP volume. Check diskpart output."
    exit 1
}

$espRoot = "${drive}:"
Write-Host "   ESP mounted at $espRoot"

# Create UEFI fallback path
New-Item -ItemType Directory -Force -Path "$espRoot\EFI\BOOT" | Out-Null
New-Item -ItemType Directory -Force -Path "$espRoot\EFI\MYOS" | Out-Null

# Copy bootloader to the UEFI fallback location
Copy-Item -Path $EfiFile -Destination "$espRoot\EFI\BOOT\BOOTX64.EFI" -Force
Write-Host "   Copied: $EfiFile -> $espRoot\EFI\BOOT\BOOTX64.EFI"

# ---- 6. Detach the VHD -----------------------------------------------------
Write-Host "[4/5] Detaching image..."
$detachScript = @"
select vdisk file="$((Resolve-Path $ImagePath).Path)"
detach vdisk
"@
$detachScript | Set-Content -Encoding ASCII $scriptFile
Start-Process "diskpart.exe" -ArgumentList "/s `"$scriptFile`"" -Verb RunAs -Wait

Remove-Item $scriptFile -ErrorAction SilentlyContinue

# ---- 7. Convert raw image to VDI for VirtualBox ----------------------------
Write-Host "[5/5] Converting to VDI format for VirtualBox..."
$vdiPath = Join-Path $ImageDir "esp.vdi"

# Remove old VDI if it exists (VBoxManage refuses to overwrite)
if (Test-Path $vdiPath) {
    & $VBoxManage closemedium disk $vdiPath --delete 2>$null
    Remove-Item $vdiPath -Force -ErrorAction SilentlyContinue
}

& $VBoxManage convertfromraw $ImagePath $vdiPath --format VDI
if ($LASTEXITCODE -ne 0) {
    Write-Error "VBoxManage convertfromraw failed."
    exit 1
}

Write-Host ""
Write-Host "SUCCESS: ESP disk image ready."
Write-Host "  Raw image : $ImagePath"
Write-Host "  VDI image : $vdiPath"
Write-Host ""
Write-Host "Next: Run scripts\run-vbox.ps1 to create and start the VM."
