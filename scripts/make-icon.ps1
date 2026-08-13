# 生成 Nicemail 应用图标(1024x1024 PNG)
Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic

# 背景:圆角方块 + 蓝色对角渐变
function New-RoundedRectPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    return $p
}

$margin = 48
$round = 230
$bgPath = New-RoundedRectPath $margin $margin ($size - 2*$margin) ($size - 2*$margin) $round
$c1 = [System.Drawing.Color]::FromArgb(255, 0, 120, 212)
$c2 = [System.Drawing.Color]::FromArgb(255, 0, 70, 140)
$bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    (New-Object System.Drawing.Point(0,0)),
    (New-Object System.Drawing.Point($size,$size)),
    $c1, $c2)
$g.FillPath($bgBrush, $bgPath)

# 信封:白色主体(略靠上留出翻盖)
$envW = 600; $envH = 430
$envX = ($size - $envW) / 2
$envY = ($size - $envH) / 2 - 18
$envBrush = [System.Drawing.SolidBrush]::New([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
$envPath = New-RoundedRectPath $envX $envY $envW $envH 24
$g.FillPath($envBrush, $envPath)

# 翻盖:顶部两侧到中心点的 V 字形 + 底部折线(蓝色线条)
$ink = [System.Drawing.Pen]::New([System.Drawing.Color]::FromArgb(255, 0, 120, 212), 26)
$ink.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$ink.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$ink.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

# V 翻盖:左上角 -> 中心点 -> 右上角
$flapY = $envY + $envH * 0.34
$g.DrawLine($ink, $envX + 8, $envY + 12, $size/2, $flapY)
$g.DrawLine($ink, $envX + $envW - 8, $envY + 12, $size/2, $flapY)

# 底部折线:信封中部偏下一条水平线
$foldY = $envY + $envH * 0.62
$g.DrawLine($ink, $envX + 10, $foldY, $envX + $envW - 10, $foldY)

# 左侧封口竖线(更接近真实信封)
$g.DrawLine($ink, $envX + 10, $envY + 12, $envX + 10, $envY + $envH - 12)

$ink.Dispose(); $envBrush.Dispose(); $bgBrush.Dispose(); $bgPath.Dispose(); $envPath.Dispose()

$out = "C:\blog\Nicemail\app-icon.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved: $out"
