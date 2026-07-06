using LuxiFer.Core.Machines;

namespace LuxiFer.Core.Tests;

public class StateGuardTests
{
    [Fact]
    public void EmergencyStop_ist_in_jedem_Zustand_erlaubt()
    {
        foreach (var state in Enum.GetValues<MachineState>())
            Assert.True(StateGuard.IsAllowed(new MachineCommand.EmergencyStop(), state));
    }

    [Fact]
    public void StartJob_nur_im_Idle_Zustand()
    {
        Assert.True(StateGuard.IsAllowed(new MachineCommand.StartJob(Guid.NewGuid()), MachineState.Idle));
        Assert.False(StateGuard.IsAllowed(new MachineCommand.StartJob(Guid.NewGuid()), MachineState.Running));
        Assert.False(StateGuard.IsAllowed(new MachineCommand.StartJob(Guid.NewGuid()), MachineState.Disconnected));
    }

    [Fact]
    public void Disconnect_nicht_waehrend_laufendem_Job()
    {
        Assert.False(StateGuard.IsAllowed(new MachineCommand.Disconnect(), MachineState.Running));
        Assert.True(StateGuard.IsAllowed(new MachineCommand.Disconnect(), MachineState.Idle));
    }
}
