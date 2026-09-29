param(
    [Parameter(Mandatory = $true)][int]$ProcId,
    [Parameter(Mandatory = $true)][string]$OutFile
)
# bricscad_console_scraper.ps1 -- the window-lifecycle + text logger.
#
# BricsCAD's console text (AUDIT reports, restorer messages, the
# "General modeling failure" line) is not reachable through the
# classic channels on this build — all TESTED (2026-09-29):
#   - LOGFILENAME is read-only; LOGFILEON writes no file
#   - WM_GETTEXT on BricsCAD windows returns EMPTY (the UI is Qt;
#     all text is painted, not stored in window text slots)
#   - UI Automation exposes NO Text-/Value-pattern controls and an
#     empty Name tree (no accessibility bridge active)
#   - GetWindowText DOES work for window TITLES (managed by the
#     window manager, not Qt)
#
# So this scraper records what is reachable: a chronological
# window-lifecycle transcript. Every 250 ms it enumerates the
# process's top-level windows and logs each window's class + title
# when it first appears (and when its title changes), with
# timestamps — capturing WHICH dialogs appear during a run (the
# modeling-failure dialog is a top-level window), WHEN, and WHAT
# they are titled. Combined with the LOGSEC LISP census and the
# DBMOD/ERRNO record, this is the complete programmatic evidence
# surface on this build; the console text itself needs a build with
# a working log channel or accessibility bridge, or a hand-run
# transcript from the maintainer.
#
# Usage: powershell -File bricscad_console_scraper.ps1 -ProcId <pid> -OutFile <log>
# Exits when the target process exits.

$ErrorActionPreference = 'SilentlyContinue'

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Text;
using System.Runtime.InteropServices;

public static class Win32Dump {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr lp);

    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder sb, int max);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] public static extern int GetClassName(IntPtr hwnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);

    public static List<string> Dump(uint targetPid) {
        var lines = new List<string>();
        EnumWindows(delegate(IntPtr h, IntPtr lp) {
            uint pid;
            GetWindowThreadProcessId(h, out pid);
            if (pid == targetPid) {
                bool visible = IsWindowVisible(h);
                var cls = new StringBuilder(256);
                GetClassName(h, cls, 256);
                var sb = new StringBuilder(1024);
                GetWindowText(h, sb, 1024);
                lines.Add((visible ? "VIS " : "hid ") + "[" + cls + "] " + sb.ToString());
            }
            return true;
        }, IntPtr.Zero);
        return lines;
    }
}
"@

$writer = New-Object System.IO.StreamWriter($OutFile, $true)
$writer.AutoFlush = $true
$writer.WriteLine("")
$writer.WriteLine("=== window scrape start pid=$ProcId $(Get-Date -Format o) ===")

$prevSet = New-Object 'System.Collections.Generic.HashSet[string]'

try {
    while ($true) {
        $p = Get-Process -Id $ProcId -ErrorAction SilentlyContinue
        if (-not $p -or $p.HasExited) { break }
        $lines = [Win32Dump]::Dump([uint32]$ProcId)
        $new = @($lines | Where-Object { -not $prevSet.Contains($_) })
        if ($new.Count -gt 0) {
            $writer.WriteLine("--- poll $(Get-Date -Format HH:mm:ss.fff) ---")
            foreach ($l in $new) { $writer.WriteLine($l) }
        }
        $prevSet = New-Object 'System.Collections.Generic.HashSet[string]'
        foreach ($l in $lines) { $prevSet.Add($l) | Out-Null }
        Start-Sleep -Milliseconds 250
    }
} finally {
    $writer.WriteLine("")
    $writer.WriteLine("=== window scrape end $(Get-Date -Format o) ===")
    $writer.Close()
}
