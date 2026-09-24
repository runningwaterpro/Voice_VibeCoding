[CmdletBinding()]
param(
  # Install modes are retained for IPC compatibility. The application uses the
  # official VB-CABLE installer; this script is now read-only.
  [ValidateSet("Install", "InstallElevated", "Finish", "Repair", "Restore", "Audit")]
  [string] $Mode = "Audit"
)

$ErrorActionPreference = "Stop"

function Get-VBCableEndpoint([string] $Flow, [string] $Prefix, [string] $Pattern) {
  $root = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\$Flow"
  foreach ($key in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
    $state = (Get-ItemProperty -LiteralPath $key.PSPath -Name DeviceState -ErrorAction SilentlyContinue).DeviceState
    if ($null -ne $state -and [int] $state -ne 1) { continue }
    $props = Get-ItemProperty -LiteralPath (Join-Path $key.PSPath "Properties") -ErrorAction SilentlyContinue
    $name = "$($props.'{a45c254e-df1c-4efd-8020-67d146a850e0},2') $($props.'{b3f8fa53-0004-438e-9003-51a46e139bfc},6')".Trim()
    if ($name -match $Pattern) {
      return [pscustomobject]@{ Id = "$Prefix.$($key.PSChildName)"; Name = $name }
    }
  }
  return $null
}

function Get-VBCableCapture {
  Get-VBCableEndpoint "Capture" "{0.0.1.00000000}" "(?i)(^|\s)CABLE Output(\s|$)"
}

function Get-VBCableRender {
  Get-VBCableEndpoint "Render" "{0.0.0.00000000}" "(?i)(^|\s)CABLE Input(\s|$)"
}

function Test-VBCableReady {
  return [bool](Get-VBCableCapture) -and [bool](Get-VBCableRender)
}

$ready = Test-VBCableReady
if ($ready) {
  $result = "OK"
} else {
  $result = "WARNING: VB-CABLE endpoints not available (read-only check)"
}

# This script never changes the Windows default capture endpoint, microphone
# privacy settings, RunOnce entries, or any other system preference. Users
# select CABLE Output in their external input method.
@(
  "Xiaomi Remote Bridge audio check",
  "Time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')",
  "Mode: $Mode",
  "Result: $result",
  "VB-CABLE render: $([bool](Get-VBCableRender))",
  "VB-CABLE capture: $([bool](Get-VBCableCapture))",
  "System audio preferences: unchanged",
  "Input method or speech recognition: not included"
) | ForEach-Object { Write-Output $_ }

exit 0
