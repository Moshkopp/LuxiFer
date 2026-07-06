namespace LuxiFer.Core.Machines;

public enum MachineState
{
    Disconnected,
    Connecting,
    Idle,
    Jogging,
    Running,
    Paused,
    Alarm,
    EmergencyStop,
}
