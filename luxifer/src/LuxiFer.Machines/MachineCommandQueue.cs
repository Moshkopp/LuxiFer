using System.Threading.Channels;
using LuxiFer.Core.Machines;

namespace LuxiFer.Machines;

/// <summary>
/// Zentrale Command Queue: serialisiert alle Maschinenbefehle und
/// wendet vor jeder Ausführung den StateGuard an.
/// </summary>
public sealed class MachineCommandQueue(IMachineDriver driver)
{
    private readonly Channel<MachineCommand> _channel =
        Channel.CreateUnbounded<MachineCommand>(new UnboundedChannelOptions { SingleReader = true });

    public bool TryEnqueue(MachineCommand command)
    {
        if (!StateGuard.IsAllowed(command, driver.State)) return false;
        return _channel.Writer.TryWrite(command);
    }

    public async Task RunAsync(CancellationToken ct)
    {
        await foreach (var command in _channel.Reader.ReadAllAsync(ct))
        {
            if (!StateGuard.IsAllowed(command, driver.State)) continue;
            await driver.ExecuteAsync(command, ct);
        }
    }
}
