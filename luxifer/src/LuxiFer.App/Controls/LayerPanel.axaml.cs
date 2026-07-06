using System;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using LuxiFer.Core.Canvas;

namespace LuxiFer.App.Controls;

/// <summary>
/// Layer-Liste („Schnitte / Ebenen"). Doppelklick auf einen Layer meldet den
/// Bearbeitungswunsch, ein Umschalten von Sichtbarkeit/Sperre ein Redraw —
/// beides über Events, damit das Control nicht selbst Fenster/Canvas kennt.
/// </summary>
public partial class LayerPanel : UserControl
{
    public LayerPanel()
    {
        InitializeComponent();
    }

    /// <summary>Doppelklick auf einen Layer (Parameter bearbeiten).</summary>
    public event EventHandler<Layer>? LayerEditRequested;

    /// <summary>Sichtbarkeit oder Sperre eines Layers wurde umgeschaltet.</summary>
    public event EventHandler? LayerToggled;

    /// <summary>Für einen Layer wurde eine neue Palettenfarbe gewählt.</summary>
    public event EventHandler<(Layer Layer, string Color)>? LayerColorChangeRequested;

    private void OnLayerDoubleTapped(object? sender, TappedEventArgs e)
    {
        if ((sender as ListBox)?.SelectedItem is Layer layer)
            LayerEditRequested?.Invoke(this, layer);
    }

    private void OnLayerToggle(object? sender, RoutedEventArgs e) =>
        LayerToggled?.Invoke(this, EventArgs.Empty);

    // Layer, dessen Farbpalette gerade geöffnet ist.
    private Layer? _swatchTarget;

    /// <summary>Öffnen der Palette merkt sich den betroffenen Layer.</summary>
    private void OnSwatchButtonClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Control { DataContext: Layer layer })
            _swatchTarget = layer;
    }

    /// <summary>Klick auf ein Palettenfeld: gemerkten Layer auf die Farbe (Tag) setzen.</summary>
    private void OnSwatchClick(object? sender, RoutedEventArgs e)
    {
        if (_swatchTarget is { } layer && sender is Control { Tag: string color })
            LayerColorChangeRequested?.Invoke(this, (layer, color));
    }
}
