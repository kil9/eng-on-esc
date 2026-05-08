# tools/generate-icon.ps1
#
# ESC 키 캡 모양의 트레이 아이콘 (.ico) 을 생성한다.
# System.Drawing 으로 16/32/48/256 PNG 비트맵을 그리고 ICO 컨테이너에 패킹한다.
# 출력: <repo>/assets/icon.ico
#
# 실행: pwsh -File tools/generate-icon.ps1

[CmdletBinding()]
param(
    [string]$OutPath
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

function New-EscIcon {
    param([int]$Size)

    $bmp = New-Object System.Drawing.Bitmap $Size, $Size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
        $g.Clear([System.Drawing.Color]::Transparent)

        # 키 캡 사각형 영역 (1px 여백)
        $pad = [Math]::Max(1, [int]($Size * 0.06))
        $rect = New-Object System.Drawing.Rectangle $pad, $pad, ($Size - 2 * $pad - 1), ($Size - 2 * $pad - 1)

        # 둥근 사각형 path
        $radius = [Math]::Max(2, [int]($Size * 0.18))
        $diameter = $radius * 2
        $path = New-Object System.Drawing.Drawing2D.GraphicsPath
        $path.AddArc($rect.X, $rect.Y, $diameter, $diameter, 180, 90)
        $path.AddArc($rect.Right - $diameter, $rect.Y, $diameter, $diameter, 270, 90)
        $path.AddArc($rect.Right - $diameter, $rect.Bottom - $diameter, $diameter, $diameter, 0, 90)
        $path.AddArc($rect.X, $rect.Bottom - $diameter, $diameter, $diameter, 90, 90)
        $path.CloseFigure()

        # 배경: 거의 흰색, 살짝 위쪽이 밝은 그라데이션
        $gradTop = [System.Drawing.Color]::FromArgb(255, 252, 252, 252)
        $gradBot = [System.Drawing.Color]::FromArgb(255, 220, 220, 222)
        $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
            (New-Object System.Drawing.Point $rect.X, $rect.Y),
            (New-Object System.Drawing.Point $rect.X, $rect.Bottom),
            $gradTop, $gradBot)
        $g.FillPath($brush, $path)
        $brush.Dispose()

        # 테두리: 진한 회색
        $penWidth = [Math]::Max(1.0, $Size / 16.0)
        $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 50, 50, 55)), $penWidth
        $g.DrawPath($pen, $path)
        $pen.Dispose()

        # "Esc" 텍스트 (24px 이상에서만; 그 미만은 키 캡만으로 식별)
        if ($Size -ge 24) {
            $fontSize = [int]([Math]::Round($Size * 0.42))
            # Segoe UI Bold 가 안 깔린 환경 대비 fallback
            $fontFamilies = @('Segoe UI', 'Arial', 'Tahoma')
            $font = $null
            foreach ($fam in $fontFamilies) {
                try {
                    $font = New-Object System.Drawing.Font($fam, $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
                    break
                } catch { }
            }
            if ($null -eq $font) {
                $font = New-Object System.Drawing.Font([System.Drawing.SystemFonts]::DefaultFont.FontFamily, $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
            }
            $textBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 30, 30, 35))
            $sf = New-Object System.Drawing.StringFormat
            $sf.Alignment = [System.Drawing.StringAlignment]::Center
            $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
            # 시각적으로 약간 위쪽이 어색해 1px 들여 그린다
            $textRect = New-Object System.Drawing.RectangleF $rect.X, ($rect.Y - $Size * 0.02), $rect.Width, $rect.Height
            $g.DrawString('Esc', $font, $textBrush, $textRect, $sf)
            $font.Dispose()
            $textBrush.Dispose()
            $sf.Dispose()
        } else {
            # 작은 사이즈: 키 캡 안쪽에 작은 점 하나 (식별성 보조)
            $dot = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 50, 50, 55))
            $cx = [int]($Size / 2)
            $cy = [int]($Size / 2)
            $g.FillRectangle($dot, $cx - 2, $cy - 1, 4, 2)
            $dot.Dispose()
        }

        $path.Dispose()
    } finally {
        $g.Dispose()
    }
    return $bmp
}

# 출력 경로 결정
if (-not $OutPath) {
    $repoRoot = Split-Path $PSScriptRoot -Parent
    $assetsDir = Join-Path $repoRoot 'assets'
    if (-not (Test-Path $assetsDir)) {
        New-Item -ItemType Directory -Path $assetsDir | Out-Null
    }
    $OutPath = Join-Path $assetsDir 'icon.ico'
}

$sizes = @(16, 24, 32, 48, 64, 256)

# 비트맵 → PNG 바이트
$pngs = @{}
$bitmaps = @{}
try {
    foreach ($s in $sizes) {
        $bitmaps[$s] = New-EscIcon -Size $s
        $ms = New-Object System.IO.MemoryStream
        $bitmaps[$s].Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $pngs[$s] = $ms.ToArray()
        $ms.Dispose()
    }

    # ICO 컨테이너 작성
    $out = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter $out

    # ICONDIR
    $bw.Write([UInt16]0)                # idReserved
    $bw.Write([UInt16]1)                # idType (1 = icon)
    $bw.Write([UInt16]$sizes.Count)     # idCount

    # ICONDIRENTRYs
    $offset = 6 + 16 * $sizes.Count
    foreach ($s in $sizes) {
        $w = if ($s -ge 256) { 0 } else { $s }
        $h = if ($s -ge 256) { 0 } else { $s }
        $bw.Write([Byte]$w)             # bWidth
        $bw.Write([Byte]$h)             # bHeight
        $bw.Write([Byte]0)              # bColorCount (0 = no palette)
        $bw.Write([Byte]0)              # bReserved
        $bw.Write([UInt16]1)            # wPlanes
        $bw.Write([UInt16]32)           # wBitCount
        $bw.Write([UInt32]$pngs[$s].Length)  # dwBytesInRes
        $bw.Write([UInt32]$offset)      # dwImageOffset
        $offset += $pngs[$s].Length
    }

    # 이미지 데이터 (각 PNG 그대로)
    foreach ($s in $sizes) {
        $bw.Write($pngs[$s])
    }

    [System.IO.File]::WriteAllBytes($OutPath, $out.ToArray())
    $bw.Dispose()
    $out.Dispose()
} finally {
    foreach ($s in $sizes) {
        if ($bitmaps.ContainsKey($s) -and $null -ne $bitmaps[$s]) {
            $bitmaps[$s].Dispose()
        }
    }
}

Write-Host "Generated $OutPath ($($sizes.Count) sizes: $($sizes -join ', '))"
