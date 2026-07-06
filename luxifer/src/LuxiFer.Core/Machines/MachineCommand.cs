namespace LuxiFer.Core.Machines;

/// <summary>
/// Alle Maschinenbefehle laufen als Commands über die zentrale Queue.
/// Die UI erzeugt Commands, greift aber nie direkt auf einen Treiber zu.
/// </summary>
public abstract record MachineCommand
{
    public sealed record Connect(string ConnectionString) : MachineCommand;
    public sealed record Disconnect : MachineCommand;
    public sealed record Home : MachineCommand;
    public sealed record Jog(double Dx, double Dy, double Speed) : MachineCommand;
    public sealed record StartJob(Guid JobId) : MachineCommand;
    public sealed record Pause : MachineCommand;
    public sealed record Resume : MachineCommand;
    public sealed record Stop : MachineCommand;
    public sealed record EmergencyStop : MachineCommand;
}
