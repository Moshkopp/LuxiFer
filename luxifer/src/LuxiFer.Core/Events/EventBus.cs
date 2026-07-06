using System.Collections.Concurrent;

namespace LuxiFer.Core.Events;

/// <summary>Einfacher In-Process-Eventbus für die Desktop-Anwendung.</summary>
public sealed class EventBus : IEventBus
{
    private readonly ConcurrentDictionary<Type, List<Delegate>> _handlers = new();

    public void Publish<TEvent>(TEvent domainEvent) where TEvent : IDomainEvent
    {
        if (!_handlers.TryGetValue(typeof(TEvent), out var handlers)) return;
        Delegate[] snapshot;
        lock (handlers) snapshot = handlers.ToArray();
        foreach (var handler in snapshot)
            ((Action<TEvent>)handler)(domainEvent);
    }

    public IDisposable Subscribe<TEvent>(Action<TEvent> handler) where TEvent : IDomainEvent
    {
        var handlers = _handlers.GetOrAdd(typeof(TEvent), _ => []);
        lock (handlers) handlers.Add(handler);
        return new Subscription(handlers, handler);
    }

    private sealed class Subscription(List<Delegate> handlers, Delegate handler) : IDisposable
    {
        public void Dispose()
        {
            lock (handlers) handlers.Remove(handler);
        }
    }
}
