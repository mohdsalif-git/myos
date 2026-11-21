# scripts/run-vbox.ps1
# ---------------------------------------------------------------------------
# Sets up and runs MyOS in VirtualBox 7.x using the downloaded ISO.
#
# USAGE (PowerShell, run from workspace root):
#   powershell -ExecutionPolicy Bypass -File scripts\run-vbox.ps1
#
# PREREQUISITES:
#   1. Download `myos-b1.iso` from GitHub Actions artifacts.
#   2. Place it at: build\myos-b1.iso  (relative to workspace root)
#   3. VirtualBox 7.x installed at the default path.
# ---------------------------------------------------------------------------

param(
    [string]$IsoPath  = "build\myos-b1.iso",
    [string]$VmName   = "MyOS-B1"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$VBox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"

# ---- Resolve ISO path -------------------------------------------------------
$IsoPath = Resolve-Path $IsoPath -ErrorAction Stop
Write-Host "ISO: $IsoPath"

# ---- Delete old VM if it exists --------------------------------------------
$existing = & $VBox list vms | Select-String $VmName
if ($existing) {
    Write-Host "Removing existing VM: $VmName"
    & $VBox unregistervm $VmName --delete
}

# ---- Create VM --------------------------------------------------------------
Write-Host "[1/7] Creating VM: $VmName"
& $VBox createvm --name $VmName --ostype "Other_64" --register
# "Other_64" = 64-bit unknown OS. VirtualBox uses this for custom OS.

# ---- Configure: EFI64, 2 GiB RAM, 2 vCPUs ---------------------------------
Write-Host "[2/7] Setting firmware to EFI64 and hardware..."
& $VBox modifyvm $VmName `
    --firmware efi64 `
    --memory 512 `
    --cpus 2 `
    --acpi on `
    --ioapic on `
    --rtcuseutc on

# ---- Attach SATA controller and virtual DVD --------------------------------
Write-Host "[3/7] Attaching ISO as DVD drive..."
& $VBox storagectl $VmName --name "SATA" --add sata --controller IntelAHCI
& $VBox storageattach $VmName `
    --storagectl "SATA" `
    --port 0 `
    --device 0 `
    --type dvddrive `
    --medium $IsoPath

# ---- Boot order: DVD first -------------------------------------------------
& $VBox modifyvm $VmName --boot1 dvd --boot2 none --boot3 none --boot4 none

# ---- Display (VMSVGA for GOP framebuffer) -----------------------------------
Write-Host "[4/7] Configuring display..."
& $VBox modifyvm $VmName --graphicscontroller vmsvga --vram 16

# ---- Serial port: COM1 → logfile -------------------------------------------
Write-Host "[5/7] Enabling serial log (COM1 → build\serial.log)..."
$logPath = Join-Path (Get-Location) "build\serial.log"
& $VBox modifyvm $VmName `
    --uart1 0x3F8 4 `
    --uartmode1 file $logPath

# ---- Start VM (GUI mode so you can see the screen) -------------------------
Write-Host "[6/7] Starting VM in GUI mode..."
Write-Host "      You should see the MyOS B1 banner on screen."
Write-Host "      Serial log: $logPath"
& $VBox startvm $VmName --type gui

Write-Host "[7/7] VM started. Close VirtualBox when done."
Write-Host ""
Write-Host "To delete the VM afterwards:"
Write-Host "  VBoxManage unregistervm '$VmName' --delete"
