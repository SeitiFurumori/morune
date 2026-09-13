param([int]$Width = 1440, [int]$Height = 960)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$probe = New-Object System.Windows.Forms.Form
$probe.Text = 'Morune - verificacao de material'
$probe.FormBorderStyle = 'None'
$probe.StartPosition = 'Manual'
$probe.Bounds = New-Object System.Drawing.Rectangle 0, 0, $Width, $Height
$probe.BackColor = [System.Drawing.Color]::RoyalBlue
$stripe = New-Object System.Windows.Forms.Panel
$stripe.Dock = 'Left'
$stripe.Width = [int]($Width / 2)
$stripe.BackColor = [System.Drawing.Color]::White
$probe.Controls.Add($stripe)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 30000
$timer.Add_Tick({
    if ($probe.BackColor -eq [System.Drawing.Color]::RoyalBlue) {
        $probe.BackColor = [System.Drawing.Color]::OrangeRed
    } else {
        $probe.BackColor = [System.Drawing.Color]::RoyalBlue
    }
})
$timer.Start()
try { [System.Windows.Forms.Application]::Run($probe) }
finally { $timer.Dispose(); $probe.Dispose() }
