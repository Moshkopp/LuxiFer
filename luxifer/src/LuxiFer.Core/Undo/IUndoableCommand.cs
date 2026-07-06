namespace LuxiFer.Core.Undo;

/// <summary>
/// Eine rückgängig-machbare Änderung am Dokument. Jede Benutzeraktion, die
/// den Canvas verändert, wird als solches Command ausgeführt — nur so wird
/// sie undo-fähig. Direkte Mutationen am Dokument umgehen die Historie.
/// </summary>
public interface IUndoableCommand
{
    /// <summary>Kurze Beschreibung für Menü/Statuszeile, z. B. "Rechteck zeichnen".</summary>
    string Label { get; }

    void Do();
    void Undo();
}
