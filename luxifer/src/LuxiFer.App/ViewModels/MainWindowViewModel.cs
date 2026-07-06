using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LuxiFer.App.Controls;
using LuxiFer.Core.Canvas;
using LuxiFer.Core.Projects;

namespace LuxiFer.App.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
    [ObservableProperty]
    private Project _project;

    [ObservableProperty]
    private CanvasTool _activeTool = CanvasTool.Select;

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
    }

    public string Title => $"LuxiFer — {Project.Name}";

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
    private CanvasObject? _selectedObject;

    public bool HasSelection => SelectedObject is not null;

    [ObservableProperty] private double _selX;
    [ObservableProperty] private double _selY;
    [ObservableProperty] private double _selWidth;
    [ObservableProperty] private double _selHeight;

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
        _updatingSelectionFields = false;
    }

    partial void OnSelXChanged(double value) => ApplySelectionBounds();
    partial void OnSelYChanged(double value) => ApplySelectionBounds();
    partial void OnSelWidthChanged(double value) => ApplySelectionBounds();
    partial void OnSelHeightChanged(double value) => ApplySelectionBounds();

    private void ApplySelectionBounds()
    {
        if (_updatingSelectionFields || SelectedObject is null) return;
        SelectedObject.SetBounds(SelX, SelY, Math.Max(0.1, SelWidth), Math.Max(0.1, SelHeight));
        Project.ModifiedAt = DateTimeOffset.UtcNow;
        CanvasInvalidateRequested?.Invoke(this, EventArgs.Empty);
    }
}
