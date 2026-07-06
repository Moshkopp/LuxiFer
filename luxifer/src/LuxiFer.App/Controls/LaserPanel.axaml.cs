using Avalonia.Controls;

namespace LuxiFer.App.Controls;

/// <summary>
/// Maschinen-Steuerpanel des Laser-Modus. Derzeit reines UI-Gerüst — die
/// Aktionen werden später über die MachineCommandQueue und einen IMachineDriver
/// angebunden (siehe ADR 0002).
/// </summary>
public partial class LaserPanel : UserControl
{
    public LaserPanel()
    {
        InitializeComponent();
    }
}
