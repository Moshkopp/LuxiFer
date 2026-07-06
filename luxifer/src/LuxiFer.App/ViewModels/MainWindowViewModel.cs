using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LuxiFer.App.Controls;
using LuxiFer.Core.Canvas;
using LuxiFer.Core.Projects;
using LuxiFer.Core.Undo;

namespace LuxiFer.App.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
    [ObservableProperty]
    private Project _project;

    /// <summary>Undo-/Redo-Historie; alle Canvas-Aktionen laufen hierüber.</summary>
    public UndoStack Undo { get; } = new();

    [ObservableProperty]
    private CanvasTool _activeTool = CanvasTool.Select;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsDesignMode))]
    [NotifyPropertyChangedFor(nameof(IsLaserMode))]
    [NotifyPropertyChangedFor(nameof(ShowNoSelectionHint))]
    private WorkMode _mode = WorkMode.Design;

    public bool IsDesignMode => Mode == WorkMode.Design;
    public bool IsLaserMode => Mode == WorkMode.Laser;

    /// <summary>Hinweis „kein Objekt" nur im Design-Modus ohne Auswahl.</summary>
    public bool ShowNoSelectionHint => IsDesignMode && !HasSelection;

    [RelayCommand]
    private void SetMode(WorkMode mode)
    {
        Mode = mode;
        StatusText = mode == WorkMode.Design
            ? "Design-Modus: Zeichnen und Anordnen"
            : "Laser-Modus: Maschinenparameter und Job";
    }

    [ObservableProperty]
    private Layer? _activeLayer;

    [ObservableProperty]
    private string _statusText = "Bereit";

    [ObservableProperty]
    private string _cursorPosition = "";

    public ObservableCollection<Layer> Layers { get; } = [];

    public static LayerMode[] LayerModes { get; } = Enum.GetValues<LayerMode>();

    /// <summary>Das Canvas soll neu gezeichnet werden (Parameter im Panel geändert).</summary>
    public event EventHandler? CanvasInvalidateRequested;

    public MainWindowViewModel()
    {
        _project = NewProjectInternal();
        SyncLayers();
        Undo.Changed += (_, _) =>
        {
            UndoActionCommand.NotifyCanExecuteChanged();
            RedoActionCommand.NotifyCanExecuteChanged();
            OnPropertyChanged(nameof(UndoHint));
            OnPropertyChanged(nameof(RedoHint));
            RefreshSelectionFields();
            CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
        };
    }

    public string Title => $"LuxiFer — {Project.Name}";

    public string UndoHint => Undo.NextUndoLabel is { } l ? $"Rückgängig: {l}" : "Rückgängig";
    public string RedoHint => Undo.NextRedoLabel is { } l ? $"Wiederholen: {l}" : "Wiederholen";

    [RelayCommand(CanExecute = nameof(CanUndo))]
    private void UndoAction()
    {
        Undo.Undo();
        StatusText = "Rückgängig";
    }

    private bool CanUndo() => Undo.CanUndo;

    [RelayCommand(CanExecute = nameof(CanRedo))]
    private void RedoAction()
    {
        Undo.Redo();
        StatusText = "Wiederholt";
    }

    private bool CanRedo() => Undo.CanRedo;

    private static Project NewProjectInternal()
    {
        var project = new Project { Name = "Unbenannt" };
        project.Canvas.Layers.Add(Layer.CreateNext(0));
        return project;
    }

    private void SyncLayers()
    {
        Layers.Clear();
        foreach (var layer in Project.Canvas.Layers)
            Layers.Add(layer);
        ActiveLayer = Layers.FirstOrDefault();
        OnPropertyChanged(nameof(Title));
    }

    [RelayCommand]
    private void NewProject()
    {
        Project = NewProjectInternal();
        SelectedObject = null;
        Undo.Clear();
        SyncLayers();
        StatusText = "Neues Projekt angelegt";
    }

    [RelayCommand]
    private void AddLayer()
    {
        var layer = Layer.CreateNext(Project.Canvas.Layers.Count);
        Project.Canvas.Layers.Add(layer);
        Layers.Add(layer);
        ActiveLayer = layer;
        StatusText = $"{layer.Name} hinzugefügt";
    }

    /// <summary>
    /// Weist einem Layer eine Palettenfarbe zu. Da <see cref="Layer"/> ein
    /// reines Core-POCO ohne Change-Notification ist, wird die Zeile über einen
    /// Replace derselben Instanz neu gebunden; die Auswahl bleibt erhalten.
    /// </summary>
    [RelayCommand]
    private void SetLayerColor((Layer Layer, string Color) arg)
    {
        arg.Layer.ColorHex = arg.Color;
        var index = Layers.IndexOf(arg.Layer);
        if (index >= 0) Layers[index] = arg.Layer;
        ActiveLayer = arg.Layer;
        CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
    }

    [RelayCommand]
    private void RemoveLayer(Layer? layer)
    {
        if (layer is null || Project.Canvas.Layers.Count <= 1) return;
        Project.Canvas.Layers.Remove(layer);
        Layers.Remove(layer);
        if (ActiveLayer == layer) ActiveLayer = Layers.FirstOrDefault();
        StatusText = $"{layer.Name} entfernt";
        CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
    }

    [RelayCommand]
    private void SelectTool(CanvasTool tool)
    {
        ActiveTool = tool;
        StatusText = tool switch
        {
            CanvasTool.Select => "Auswählen: Klicken wählt, Ziehen verschiebt, Handles skalieren",
            CanvasTool.Rectangle => "Rechteck aufziehen",
            CanvasTool.Ellipse => "Ellipse aufziehen",
            CanvasTool.Line => "Linie ziehen",
            CanvasTool.Polyline => "Polyline: Klick setzt Punkte, Enter/Doppelklick beendet, Esc bricht ab",
            CanvasTool.Polygon => "Polygon: Klick setzt Punkte, Enter/Doppelklick schließt, Esc bricht ab",
            _ => "",
        };
    }

    public void ReportCursor(double xMm, double yMm) =>
        CursorPosition = $"X {xMm:F1} mm   Y {yMm:F1} mm";

    public void MarkDirty()
    {
        Project.ModifiedAt = DateTimeOffset.UtcNow;
        StatusText = "Geändert";
        RefreshSelectionFields();
    }

    // ----- Auswahl / Eigenschaften-Panel -----

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasSelection))]
    [NotifyPropertyChangedFor(nameof(ShowNoSelectionHint))]
    private CanvasObject? _selectedObject;

    public bool HasSelection => SelectedObject is not null;

    [ObservableProperty] private double _selX;
    [ObservableProperty] private double _selY;
    [ObservableProperty] private double _selWidth;
    [ObservableProperty] private double _selHeight;
    [ObservableProperty] private double _selRotation;
    [ObservableProperty] private double _selScalePct = 100;

    /// <summary>Seitenverhältnis von Breite/Höhe sperren.</summary>
    [ObservableProperty] private bool _lockAspect;

    private bool _updatingSelectionFields;

    partial void OnSelectedObjectChanged(CanvasObject? value) => RefreshSelectionFields();

    private void RefreshSelectionFields()
    {
        if (SelectedObject is null) return;
        _updatingSelectionFields = true;
        var (x, y, w, h) = SelectedObject.Bounds;
        SelX = Math.Round(x, 2);
        SelY = Math.Round(y, 2);
        SelWidth = Math.Round(w, 2);
        SelHeight = Math.Round(h, 2);
        SelRotation = Math.Round(SelectedObject.Rotation, 1);
        SelScalePct = 100;
        _updatingSelectionFields = false;
    }

    // Bounds/Rotation vor Beginn einer Panel-Bearbeitung, für je ein Undo-Command
    // über die gesamte Feld-Editiersequenz (bis CommitSelectionEdit).
    private (double X, double Y, double W, double H)? _editStartBounds;
    private double? _editStartRotation;

    partial void OnSelXChanged(double value) => ApplySelectionBounds();
    partial void OnSelYChanged(double value) => ApplySelectionBounds();

    partial void OnSelWidthChanged(double value)
    {
        if (_updatingSelectionFields || !LockAspect) { ApplySelectionBounds(); return; }
        // Seitenverhältnis halten: Höhe proportional zur alten Breite nachziehen.
        var start = _editStartBounds ?? SelectedObject?.Bounds;
        if (start is { W: > 0 } s)
        {
            _updatingSelectionFields = true;
            SelHeight = Math.Round(Math.Max(0.1, value) * s.H / s.W, 2);
            _updatingSelectionFields = false;
        }
        ApplySelectionBounds();
    }

    partial void OnSelHeightChanged(double value)
    {
        if (_updatingSelectionFields || !LockAspect) { ApplySelectionBounds(); return; }
        var start = _editStartBounds ?? SelectedObject?.Bounds;
        if (start is { H: > 0 } s)
        {
            _updatingSelectionFields = true;
            SelWidth = Math.Round(Math.Max(0.1, value) * s.W / s.H, 2);
            _updatingSelectionFields = false;
        }
        ApplySelectionBounds();
    }

    partial void OnSelRotationChanged(double value)
    {
        if (_updatingSelectionFields || SelectedObject is null) return;
        _editStartRotation ??= SelectedObject.Rotation;
        SelectedObject.Rotation = value;
        Project.ModifiedAt = DateTimeOffset.UtcNow;
        CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
    }

    partial void OnSelScalePctChanged(double value)
    {
        if (_updatingSelectionFields || SelectedObject is null || value <= 0) return;
        // Skaliert relativ zu den Bounds bei Editierbeginn, um den Mittelpunkt.
        _editStartBounds ??= SelectedObject.Bounds;
        var s = _editStartBounds.Value;
        var f = value / 100.0;
        var nw = Math.Max(0.1, s.W * f);
        var nh = Math.Max(0.1, s.H * f);
        var nx = s.X + (s.W - nw) / 2;
        var ny = s.Y + (s.H - nh) / 2;
        SelectedObject.SetBounds(nx, ny, nw, nh);
        RefreshBoundsFields();
        Project.ModifiedAt = DateTimeOffset.UtcNow;
        CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
    }

    private void ApplySelectionBounds()
    {
        if (_updatingSelectionFields || SelectedObject is null) return;
        _editStartBounds ??= SelectedObject.Bounds;
        SelectedObject.SetBounds(SelX, SelY, Math.Max(0.1, SelWidth), Math.Max(0.1, SelHeight));
        Project.ModifiedAt = DateTimeOffset.UtcNow;
        CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
    }

    // Aktualisiert nur die X/Y/B/H-Felder aus dem Objekt (z. B. nach Skalierung).
    private void RefreshBoundsFields()
    {
        if (SelectedObject is null) return;
        _updatingSelectionFields = true;
        var (x, y, w, h) = SelectedObject.Bounds;
        SelX = Math.Round(x, 2);
        SelY = Math.Round(y, 2);
        SelWidth = Math.Round(w, 2);
        SelHeight = Math.Round(h, 2);
        _updatingSelectionFields = false;
    }

    /// <summary>
    /// Schließt eine Feld-Bearbeitung ab (Enter/Fokusverlust) und legt die
    /// Änderung(en) als Undo-Command(s) ab. Von der View aufgerufen.
    /// </summary>
    public void CommitSelectionEdit()
    {
        if (SelectedObject is null)
        {
            _editStartBounds = null;
            _editStartRotation = null;
            return;
        }

        if (_editStartBounds is { } before)
        {
            var after = SelectedObject.Bounds;
            if (after != before)
                Undo.Push(new ResizeObjectCommand(SelectedObject, before, after));
        }
        if (_editStartRotation is { } beforeRot && beforeRot != SelectedObject.Rotation)
            Undo.Push(new RotateObjectCommand(SelectedObject, beforeRot, SelectedObject.Rotation));

        _editStartBounds = null;
        _editStartRotation = null;
        _updatingSelectionFields = true;
        SelScalePct = 100;
        _updatingSelectionFields = false;
    }
}
