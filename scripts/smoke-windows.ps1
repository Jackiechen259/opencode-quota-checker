# Windows GUI smoke test for the OpenCode Quota Checker.
#
# Verifies the acceptance criteria that unit tests cannot: a real release
# binary must create its main window quickly and the custom title bar's
# minimize / maximize / restore / close must actually drive the OS window.
# The test also stress-launches the app repeatedly to catch sporadic startup
# freezes (the original bug reproduced as a permanently frozen window).
#
# Usage (from the repo root, after `pnpm tauri build --no-bundle`):
#   pwsh ./scripts/smoke-windows.ps1 [-Exe path\to\opencode-quota-checker.exe]
#                                    [-Cycles 20]
#                                    [-WindowTitle "OpenCode Quota Checker"]
#
# Exit code 0 = every cycle passed; 1 = at least one assertion failed.

param(
  [string]$Exe = "",
  [int]$Cycles = 10,
  [string]$WindowTitle = "OpenCode Quota Checker"
)

$ErrorActionPreference = "Stop"

if (-not $Exe) {
  $candidate = Join-Path $PSScriptRoot "..\src-tauri\target\release\opencode-quota-checker.exe"
  if (-not (Test-Path $candidate)) {
    $candidate = Join-Path $PSScriptRoot "..\target\release\opencode-quota-checker.exe"
  }
  $Exe = $candidate
}
if (-not (Test-Path $Exe)) {
  Write-Error "release binary not found: $Exe (run `pnpm tauri build --no-bundle` first)"
}

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class SmokeWin32 {
  // lpClassName as IntPtr: PowerShell converts $null to "" for string
  // parameters, and an empty class name matches nothing.
  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern IntPtr FindWindow(IntPtr lpClassName, string lpWindowName);
  [DllImport("user32.dll")]
  public static extern bool IsWindow(IntPtr hWnd);
  [DllImport("user32.dll")]
  public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")]
  public static extern bool IsZoomed(IntPtr hWnd);
  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")]
  public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam,
    uint fuFlags, uint uTimeout, out IntPtr lpdwResult);
  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
  [DllImport("user32.dll")]
  public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
}
"@

$WM_SYSCOMMAND = 0x0112
$SC_MINIMIZE = 0xF020
$SC_MAXIMIZE = 0xF030
$SC_RESTORE = 0xF120
$WM_CLOSE = 0x0010
$SMTO_ABORTIFHUNG = 0x0002
$SMTO_BLOCK = 0x0001

function Find-MainWindow([int]$timeoutMs) {
  $deadline = [Environment]::TickCount64 + $timeoutMs
  while ([Environment]::TickCount64 -lt $deadline) {
    $hwnd = [SmokeWin32]::FindWindow([IntPtr]::Zero, $WindowTitle)
    if ($hwnd -ne [IntPtr]::Zero -and [SmokeWin32]::IsWindow($hwnd) -and [SmokeWin32]::IsWindowVisible($hwnd)) {
      return $hwnd
    }
    Start-Sleep -Milliseconds 100
  }
  return [IntPtr]::Zero
}

function Send-Win32([IntPtr]$hwnd, [uint32]$msg, [IntPtr]$wParam, [IntPtr]$lParam) {
  # Returns $true only when the target window processed the message within
  # the timeout (i.e. its event loop is alive).
  $result = [IntPtr]::Zero
  return [SmokeWin32]::SendMessageTimeout($hwnd, $msg, $wParam, $lParam,
    $SMTO_ABORTIFHUNG -bor $SMTO_BLOCK, 3000, [ref]$result) -ne [IntPtr]::Zero
}

function Assert-True([bool]$condition, [string]$label) {
  if (-not $condition) { throw "FAILED: $label" }
  Write-Host "  ok: $label"
}

$totalStart = [Environment]::TickCount64
$failures = 0

for ($cycle = 1; $cycle -le $Cycles; $cycle++) {
  Write-Host "=== cycle $cycle/$Cycles ==="
  $cycleStart = [Environment]::TickCount64

  $process = Start-Process -FilePath $Exe -PassThru
  try {
    $hwnd = Find-MainWindow -timeoutMs 15000
    if ($hwnd -eq [IntPtr]::Zero) {
      throw "main window did not appear within 15 s (startup freeze)"
    }
    $windowMs = [Environment]::TickCount64 - $cycleStart
    Write-Host "  window appeared after ${windowMs} ms"
    if ($windowMs -gt 6000) {
      throw "window appeared after ${windowMs} ms (> 6 s acceptance)"
    }

    # The window event loop must be alive: send the minimize command and
    # wait for the OS to actually minimize.
    Assert-True (Send-Win32 $hwnd $WM_SYSCOMMAND ([IntPtr]$SC_MINIMIZE) ([IntPtr]0)) "minimize message processed"
    $deadline = [Environment]::TickCount64 + 5000
    while ([Environment]::TickCount64 -lt $deadline -and -not [SmokeWin32]::IsIconic($hwnd)) {
      Start-Sleep -Milliseconds 50
    }
    Assert-True ([SmokeWin32]::IsIconic($hwnd)) "window minimized"

    Assert-True (Send-Win32 $hwnd $WM_SYSCOMMAND ([IntPtr]$SC_RESTORE) ([IntPtr]0)) "restore message processed"
    $deadline = [Environment]::TickCount64 + 5000
    while ([Environment]::TickCount64 -lt $deadline -and [SmokeWin32]::IsIconic($hwnd)) {
      Start-Sleep -Milliseconds 50
    }
    Assert-True (-not [SmokeWin32]::IsIconic($hwnd)) "window restored"

    Assert-True (Send-Win32 $hwnd $WM_SYSCOMMAND ([IntPtr]$SC_MAXIMIZE) ([IntPtr]0)) "maximize message processed"
    $deadline = [Environment]::TickCount64 + 5000
    while ([Environment]::TickCount64 -lt $deadline -and -not [SmokeWin32]::IsZoomed($hwnd)) {
      Start-Sleep -Milliseconds 50
    }
    Assert-True ([SmokeWin32]::IsZoomed($hwnd)) "window maximized"

    Assert-True (Send-Win32 $hwnd $WM_SYSCOMMAND ([IntPtr]$SC_RESTORE) ([IntPtr]0)) "unmaximize message processed"
    $deadline = [Environment]::TickCount64 + 5000
    while ([Environment]::TickCount64 -lt $deadline -and [SmokeWin32]::IsZoomed($hwnd)) {
      Start-Sleep -Milliseconds 50
    }
    Assert-True (-not [SmokeWin32]::IsZoomed($hwnd)) "window unmaximized"

    # Close through the same path the caption × uses.
    Assert-True (Send-Win32 $hwnd $WM_CLOSE ([IntPtr]0) ([IntPtr]0)) "close message processed"
    $exited = $process.WaitForExit(8000)
    Assert-True $exited "process exited after close"

    Write-Host "  cycle ${cycle} passed in $([Environment]::TickCount64 - $cycleStart) ms"
  } catch {
    $failures++
    Write-Host "  ERROR: $($_.Exception.Message)"
    if (-not $process.HasExited) {
      $process.Kill()
      $process.WaitForExit()
    }
  } finally {
    if (-not $process.HasExited) {
      $process.Kill()
      $process.WaitForExit()
    }
  }
}

$totalMs = [Environment]::TickCount64 - $totalStart
Write-Host ""
Write-Host "smoke test finished: $($Cycles - $failures)/$Cycles cycles passed in ${totalMs} ms"
if ($failures -gt 0) { exit 1 }
exit 0
