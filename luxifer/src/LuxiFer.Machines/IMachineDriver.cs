using LuxiFer.Core.Machines;

namespace LuxiFer.Machines;

/// <summary>
/// Einzige Schnittstelle, über die LuxiFer Maschinen kennt.
/// Implementierungen: Ruida (erste), später Simulator, GRBL, ...
/// </summary>
public interface IMachineDriver
{
    string Name { get; }
    MachineState State { get; }
    event EventHandler<MachineState>? StateChanged;

    Task ExecuteAsync(MachineCommand command, CancellationToken ct = default);
}
