namespace LuxiFer.Core.Machines;

/// <summary>
/// Prüft vor der Ausführung eines Commands, ob er im aktuellen
/// Maschinenzustand zulässig ist. EmergencyStop ist immer erlaubt.
/// </summary>
public static class StateGuard
{
    public static bool IsAllowed(MachineCommand command, MachineState state) => command switch
    {
        MachineCommand.EmergencyStop => true,
        MachineCommand.Connect => state is MachineState.Disconnected,
        MachineCommand.Disconnect => state is not MachineState.Disconnected and not MachineState.Running,
        MachineCommand.Home => state is MachineState.Idle,
        MachineCommand.Jog => state is MachineState.Idle or MachineState.Jogging,
        MachineCommand.StartJob => state is MachineState.Idle,
        MachineCommand.Pause => state is MachineState.Running,
        MachineCommand.Resume => state is MachineState.Paused,
        MachineCommand.Stop => state is MachineState.Running or MachineState.Paused,
        _ => false,
    };
}
