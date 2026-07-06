namespace LuxiFer.Core.Events;

/// <summary>
/// Basis aller Domain Events. Core, UI, Sync und Machine-Komponenten
/// kommunizieren ausschließlich über Events, nie über direkte Aufrufe.
/// </summary>
public interface IDomainEvent
{
    DateTimeOffset OccurredAt { get; }
}

public interface IEventBus
{
    void Publish<TEvent>(TEvent domainEvent) where TEvent : IDomainEvent;
    IDisposable Subscribe<TEvent>(Action<TEvent> handler) where TEvent : IDomainEvent;
}
