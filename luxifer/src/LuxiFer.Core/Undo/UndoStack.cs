namespace LuxiFer.Core.Undo;

/// <summary>
/// Verwaltet die Undo-/Redo-Historie. Ausführen eines neuen Commands
/// verwirft den Redo-Zweig (klassisches lineares Undo).
/// </summary>
public sealed class UndoStack
{
    private readonly Stack<IUndoableCommand> _undo = new();
    private readonly Stack<IUndoableCommand> _redo = new();

    /// <summary>Wird nach jeder Änderung der Historie ausgelöst (für UI-Aktualisierung).</summary>
    public event EventHandler? Changed;

    public bool CanUndo => _undo.Count > 0;
    public bool CanRedo => _redo.Count > 0;

    public string? NextUndoLabel => _undo.Count > 0 ? _undo.Peek().Label : null;
    public string? NextRedoLabel => _redo.Count > 0 ? _redo.Peek().Label : null;

    /// <summary>Führt das Command aus und legt es auf den Undo-Stapel.</summary>
    public void Execute(IUndoableCommand command)
    {
        command.Do();
        _undo.Push(command);
        _redo.Clear();
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>
    /// Legt ein bereits vollzogenes Command auf den Undo-Stapel, ohne es
    /// erneut auszuführen. Für Aktionen, die interaktiv schon passiert sind
    /// (z. B. ein abgeschlossenes Verschieben per Maus).
    /// </summary>
    public void Push(IUndoableCommand command)
    {
        _undo.Push(command);
        _redo.Clear();
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void Undo()
    {
        if (_undo.Count == 0) return;
        var command = _undo.Pop();
        command.Undo();
        _redo.Push(command);
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void Redo()
    {
        if (_redo.Count == 0) return;
        var command = _redo.Pop();
        command.Do();
        _undo.Push(command);
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void Clear()
    {
        _undo.Clear();
        _redo.Clear();
        Changed?.Invoke(this, EventArgs.Empty);
    }
}
