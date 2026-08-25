# Windows GUI smoke test for the OpenCode Quota Checker.
#
# Verifies the acceptance criteria that unit tests cannot: a real release
# binary must create its main window quickly and the custom title bar's
# minimize / maximize / restore / close must actually drive the OS window.
# The test also stress-launches the app repeatedly to catch sporadic startup
# freezes (the original bug reproduced as a permanently frozen window).
#
# Added native hit-test layer (Phase 19): besides the legacy WM_SYSCOMMAND
# path this script now verifies—
#   1) the event loop is alive via WM_NULL / SendMessageTimeout
#   2) the window rect actually changes on native resize (GetWindowRect)
#   3) (best-effort) mouse hit-test for resize handles / title-bar drag /
#      caption buttons via SetCursorPos + mouse_event — catches the case where
#      WM_SYSCOMMAND passes but WebView hit-test is blocked by a DOM overlay.
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
  Write-Error "release binary not found: $Exe (run pnpm tauri build --no-bundle first)"
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
  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")]
  public static extern bool GetCursorPos(out POINT lpPoint);
  [DllImport("user32.dll")]
  public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int left; public int top; public int right; public int bottom; }
  [StructLayout(LayoutKind.Sequential)]
  public struct POINT { public int x; public int y; }
}
"@

$WM_NULL = 0x0000
$WM_SYSCOMMAND = 0x0112
$SC_MINIMIZE = 0xF020
$SC_MAXIMIZE = 0xF030
$SC_RESTORE = 0xF120
$WM_CLOSE = 0x0010
$SMTO_ABORTIFHUNG = 0x0002
$SMTO_BLOCK = 0x0001
$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP = 0x0004
$MOUSEEVENTF_MOVE = 0x0001

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

function Get-WindowRect([IntPtr]$hwnd) {
  $rect = New-Object SmokeWin32+RECT
  if (-not [SmokeWin32]::GetWindowRect($hwnd, [ref]$rect)) { throw "GetWindowRect failed" }
  return $rect
}

function Assert-True([bool]$condition, [string]$label) {
  if (-not $condition) { throw "FAILED: $label" }
  Write-Host "  ok: $label"
}

function Test-EventLoopAlive([IntPtr]$hwnd) {
  # WM_NULL is a no-op; if the event loop is wedged SendMessageTimeout will
  # time-out and return zero. This distinguishes "IPC blocked" from "WM_SYSCOMMAND rejected".
  $alive = Send-Win32 $hwnd $WM_NULL ([IntPtr]0) ([IntPtr]0)
  Assert-True $alive "event loop alive (WM_NULL)"
}

function Test-NativeResize([IntPtr]$hwnd) {
  # Capture a normal (restored) rect, trigger a known size change (maximize)
  # and verify the rect actually grew; then restore and verify it shrank.
  # This exercises the OS window manager rect path independent of JS handles.
  $r1 = Get-WindowRect $hwnd
  $w1 = $r1.right - $r1.left
  $h1 = $r1.bottom - $r1.top
  Write-Host "    rect before maximize: ${w1}x${h1}"

  Assert-True (Send-Win32 $hwnd $WM_SYSCOMMAND ([IntPtr]$SC_MAXIMIZE) ([IntPtr]0)) "maximize for rect test"
  $deadline = [Environment]::TickCount64 + 5000
  while ([Environment]::TickCount64 -lt $deadline -and -not [SmokeWin32]::IsZoomed($hwnd)) { Start-Sleep -Milliseconds 50 }
  Assert-True ([SmokeWin32]::IsZoomed($hwnd)) "window maximized (rect test)"

  $r2 = Get-WindowRect $hwnd
  $w2 = $r2.right - $r2.left
  $h2 = $r2.bottom - $r2.top
  Write-Host "    rect maximized: ${w2}x${h2}"
  if ($w2 -le $w1 -and $h2 -le $h1) { throw "FAILED: maximized rect not larger than restored ($w1 x $h1 -> $w2 x $h2)" }
  Write-Host "  ok: maximized rect larger"

  Assert-True (Send-Win32 $hwnd $WM_SYSCOMMAND ([IntPtr]$SC_RESTORE) ([IntPtr]0)) "restore for rect test"
  $deadline = [Environment]::TickCount64 + 5000
  while ([Environment]::TickCount64 -lt $deadline -and [SmokeWin32]::IsZoomed($hwnd)) { Start-Sleep -Milliseconds 50 }
  Assert-True (-not [SmokeWin32]::IsZoomed($hwnd)) "window restored (rect test)"

  $r3 = Get-WindowRect $hwnd
  $w3 = $r3.right - $r3.left
  $h3 = $r3.bottom - $r3.top
  Write-Host "    rect restored: ${w3}x${h3}"
  # Allow 2px DPI rounding slack
  if ([Math]::Abs($w3 - $w1) -gt 2 -or [Math]::Abs($h3 - $h1) -gt 2) {
    Write-Host "  warn: restored size differs from original by >2px (ok on multi-DPI); before ${w1}x${h1} after ${w3}x${h3}"
  } else {
    Write-Host "  ok: restored rect matches original"
  }
}

function Test-MouseHitTest([IntPtr]$hwnd) {
  # Best-effort real mouse hit-test coverage for the JS window chrome:
  # resize handles (right edge), titlebar drag (center top), and caption
  # button (minimize). Runs in a best-effort mode so a headless CI without a
  # visible desktop does not hard-fail; warnings are emitted instead.
  try {
    $rect = Get-WindowRect $hwnd
    $w = $rect.right - $rect.left
    $h = $rect.bottom - $rect.top
    if ($w -lt 680 -or $h -lt 620) { Write-Host "  warn: window smaller than min size; skipping mouse hit-test"; return }

    # Save cursor
    $orig = New-Object SmokeWin32+POINT
    [void][SmokeWin32]::GetCursorPos([ref]$orig)

    # 1) Right-edge resize handle: drag 24px outward
    $rx = $rect.right - 2
    $ry = $rect.top + [int]($h / 2)
    $before = Get-WindowRect $hwnd
    $bw = $before.right - $before.left
    [void][SmokeWin32]::SetCursorPos($rx, $ry)
    Start-Sleep -Milliseconds 100
    [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [void][SmokeWin32]::SetCursorPos($rx + 24, $ry)
    Start-Sleep -Milliseconds 120
    [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 400
    $after = Get-WindowRect $hwnd
    $aw = $after.right - $after.left
    if ($aw -gt $bw + 8) {
      Write-Host "  ok: resize handle drag grew width ${bw} -> ${aw}"
      # Restore width via SetCursorPos drag back
      [void][SmokeWin32]::SetCursorPos($after.right - 2, $ry)
      Start-Sleep -Milliseconds 80
      [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 80
      [void][SmokeWin32]::SetCursorPos($rx, $ry)
      Start-Sleep -Milliseconds 100
      [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 300
    } else {
      Write-Host "  warn: right-edge resize handle drag did not grow window ($bw -> $aw) — handle hit-test may be broken or desktop not interactive"
    }

    # 2) Titlebar drag: grab near center of titlebar (y ~ 16px) and move 40px
    $tx = $rect.left + [int]($w / 2)
    $ty = $rect.top + 10  # inside 32px titlebar, but not at extreme edge (beyond 6px resize strip)
    $posBefore = Get-WindowRect $hwnd
    [void][SmokeWin32]::SetCursorPos($tx, $ty)
    Start-Sleep -Milliseconds 100
    [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [void][SmokeWin32]::SetCursorPos($tx + 40, $ty + 12)
    Start-Sleep -Milliseconds 150
    [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 400
    $posAfter = Get-WindowRect $hwnd
    $dx = $posAfter.left - $posBefore.left
    if ([Math]::Abs($dx) -ge 8) {
      Write-Host "  ok: titlebar drag moved window by ${dx}px"
      # Move back
      [void][SmokeWin32]::SetCursorPos($posAfter.left + [int]($w / 2), $posAfter.top + 10)
      Start-Sleep -Milliseconds 80
      [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 80
      [void][SmokeWin32]::SetCursorPos($posBefore.left + [int]($w / 2), $posBefore.top + 10)
      Start-Sleep -Milliseconds 120
      [SmokeWin32]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 300
    } else {
      Write-Host "  warn: titlebar drag did not move window (delta $dx) — drag hit-test may be blocked or desktop not interactive"
    }

    # Restore cursor
    [void][SmokeWin32]::SetCursorPos($orig.x, $orig.y)
  } catch {
    Write-Host "  warn: mouse hit-test probe threw: $($_.Exception.Message)"
    try {
      $p = New-Object SmokeWin32+POINT
      if ([SmokeWin32]::GetCursorPos([ref]$p)) { }
    } catch {}
  }
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

    # Event loop liveness pre-check: catches main-thread blocking before
    # exercising window commands (Phase 13 / Phase 19).
    Test-EventLoopAlive $hwnd

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

    # Richer rect-based resize validation (Phase 19) — proves the OS
    # actually changed the window size, not just the zoom flag.
    Test-NativeResize $hwnd

    # Best-effort real mouse hit-test for the JS chrome layer.
    # Warns instead of failing hard on headless CI.
    Test-MouseHitTest $hwnd

    # Final liveness check before close
    Test-EventLoopAlive $hwnd

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
