param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath
)

$ErrorActionPreference = 'Stop'
$resolvedExecutable = (Resolve-Path -LiteralPath $ExecutablePath).Path

if ($null -eq ('NativeIconProbe' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class NativeIconProbe
{
    public delegate bool EnumResNameProc(IntPtr module, IntPtr type, IntPtr name, IntPtr parameter);
    public delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryEx(string fileName, IntPtr file, uint flags);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool FreeLibrary(IntPtr module);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool EnumResourceNames(
        IntPtr module,
        IntPtr type,
        EnumResNameProc callback,
        IntPtr parameter);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr word, IntPtr data);

    [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW", SetLastError = true)]
    public static extern IntPtr GetClassLongPtr(IntPtr window, int index);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowTextLength(IntPtr window);

    public static IntPtr FindVisibleApplicationWindow(uint processId)
    {
        IntPtr result = IntPtr.Zero;
        EnumWindows((window, parameter) => {
            GetWindowThreadProcessId(window, out uint ownerProcessId);
            if (ownerProcessId == processId &&
                IsWindowVisible(window) &&
                GetWindowTextLength(window) > 0)
            {
                result = window;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return result;
    }
}
'@
}

$loadLibraryAsDataFile = 0x00000002
$resourceTypeGroupIcon = [IntPtr]14
$module = [NativeIconProbe]::LoadLibraryEx($resolvedExecutable, [IntPtr]::Zero, $loadLibraryAsDataFile)
if ($module -eq [IntPtr]::Zero) {
    throw "Unable to inspect executable resources: $resolvedExecutable"
}

$groupIconCount = 0
$callback = [NativeIconProbe+EnumResNameProc]{
    param($moduleHandle, $type, $name, $parameter)
    $script:groupIconCount++
    return $true
}
try {
    [void][NativeIconProbe]::EnumResourceNames(
        $module,
        $resourceTypeGroupIcon,
        $callback,
        [IntPtr]::Zero)
}
finally {
    [void][NativeIconProbe]::FreeLibrary($module)
}

$failures = [System.Collections.Generic.List[string]]::new()
if ($groupIconCount -lt 1) {
    $failures.Add('Explorer icon check failed: the executable has no RT_GROUP_ICON resource.')
}

$process = Start-Process -FilePath $resolvedExecutable -PassThru
try {
    $windowHandle = [IntPtr]::Zero
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        Start-Sleep -Milliseconds 100
        $process.Refresh()
        if ($process.HasExited) {
            throw "Application exited before creating its main window (exit code $($process.ExitCode))."
        }
        $windowHandle = [NativeIconProbe]::FindVisibleApplicationWindow($process.Id)
        if ($windowHandle -ne [IntPtr]::Zero) {
            break
        }
    }
    if ($windowHandle -eq [IntPtr]::Zero) {
        throw 'Taskbar icon check failed: the application did not create a main window.'
    }

    $wmGetIcon = 0x007F
    $iconSmall = [NativeIconProbe]::SendMessage($windowHandle, $wmGetIcon, [IntPtr]0, [IntPtr]::Zero)
    $iconBig = [NativeIconProbe]::SendMessage($windowHandle, $wmGetIcon, [IntPtr]1, [IntPtr]::Zero)
    $iconSmall2 = [NativeIconProbe]::SendMessage($windowHandle, $wmGetIcon, [IntPtr]2, [IntPtr]::Zero)
    $classBig = [NativeIconProbe]::GetClassLongPtr($windowHandle, -14)
    $classSmall = [NativeIconProbe]::GetClassLongPtr($windowHandle, -34)
    $windowIcons = @($iconSmall, $iconBig, $iconSmall2, $classBig, $classSmall)
    if (($windowIcons | Where-Object { $_ -ne [IntPtr]::Zero }).Count -eq 0) {
        $failures.Add('Taskbar icon check failed: the window exposes no small or large icon handle.')
    }
}
finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id
        $process.WaitForExit()
    }
}

if ($failures.Count -gt 0) {
    throw ($failures -join [Environment]::NewLine)
}

[pscustomobject]@{
    Executable = $resolvedExecutable
    GroupIconResources = $groupIconCount
    WindowIcon = $true
} | Format-List
