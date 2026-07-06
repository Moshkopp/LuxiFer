using LuxiFer.Core.Machines;

namespace LuxiFer.Machines.Ruida;

/// <summary>Ruida-Treiber (UDP-Protokoll). Erste Maschinenanbindung von LuxiFer.</summary>
public sealed class RuidaDriver : IMachineDriver
{
    public string Name => "Ruida";
    public MachineState State { get; private set; } = MachineState.Disconnected;
    public event EventHandler<MachineState>? StateChanged;

    public Task ExecuteAsync(MachineCommand command, CancellationToken ct = default)
    {
        // TODO: Ruida-UDP-Protokoll implementieren.
        throw new NotImplementedException();
    }

    private void SetState(MachineState state)
    {
        State = state;
        StateChanged?.Invoke(this, state);
    }
}
