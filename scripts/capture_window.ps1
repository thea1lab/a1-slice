param(
    [string]$OutPath = "target/screenshot.png",
    [string[]]$AppArgs = @()
)

$code = @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public class WindowCapture {
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll")]
    public static extern IntPtr GetDC(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("gdi32.dll")]
    public static extern bool BitBlt(IntPtr hObject, int nXDest, int nYDest, int nWidth, int nHeight, IntPtr hObjectSource, int nXSrc, int nYSrc, int dwRop);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left, Top, Right, Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBmp, uint nFlags);

    public static bool Capture(IntPtr hWnd, string filePath) {
        RECT rect;
        if (!GetWindowRect(hWnd, out rect)) return false;
        int width = rect.Right - rect.Left;
        int height = rect.Bottom - rect.Top;
        Console.WriteLine(string.Format("Rect: {0},{1} to {2},{3} -> {4}x{5}", rect.Left, rect.Top, rect.Right, rect.Bottom, width, height));
        if (width <= 0 || height <= 0) return false;

        SetForegroundWindow(hWnd);
        System.Threading.Thread.Sleep(1500);

        using (Bitmap bmp = new Bitmap(width, height)) {
            using (Graphics g = Graphics.FromImage(bmp)) {
                try {
                    g.CopyFromScreen(rect.Left, rect.Top, 0, 0, new Size(width, height), CopyPixelOperation.SourceCopy);
                } catch {
                    IntPtr hdc = g.GetHdc();
                    PrintWindow(hWnd, hdc, 2);
                    g.ReleaseHdc(hdc);
                }
            }
            bmp.Save(filePath, ImageFormat.Png);
        }
        return true;
    }

    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    public static IntPtr FindWindowForProcess(int targetPid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hWnd, lParam) => {
            uint pid;
            GetWindowThreadProcessId(hWnd, out pid);
            if (pid == targetPid && IsWindowVisible(hWnd)) {
                RECT r;
                if (GetWindowRect(hWnd, out r)) {
                    int w = r.Right - r.Left;
                    int h = r.Bottom - r.Top;
                    if (w >= 600 && h >= 400) {
                        found = hWnd;
                        return false;
                    }
                }
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@

if (-not ([System.Management.Automation.PSTypeName]'WindowCapture').Type) {
    Add-Type -TypeDefinition $code -ReferencedAssemblies System.Drawing
}

$p = if ($AppArgs -and $AppArgs.Count -gt 0) {
    Start-Process -FilePath "target\debug\a1slice.exe" -ArgumentList $AppArgs -PassThru
} else {
    Start-Process -FilePath "target\debug\a1slice.exe" -PassThru
}
$hwnd = [IntPtr]::Zero
for ($i = 0; $i -lt 10; $i++) {
    Start-Sleep -Milliseconds 500
    $hwnd = [WindowCapture]::FindWindowForProcess($p.Id)
    if ($hwnd -ne [IntPtr]::Zero) {
        break
    }
}

if ($hwnd -ne [IntPtr]::Zero) {
    Start-Sleep -Milliseconds 1500
    [WindowCapture]::Capture($hwnd, $OutPath)
    Write-Host "Captured window to $OutPath! File size: $((Get-Item $OutPath).Length) bytes"
} else {
    Write-Host "Window handle was Zero!"
}
Stop-Process -Id $p.Id
