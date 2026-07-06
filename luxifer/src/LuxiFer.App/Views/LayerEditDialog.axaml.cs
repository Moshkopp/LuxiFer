using Avalonia.Controls;
using Avalonia.Interactivity;
using LuxiFer.Core.Canvas;

namespace LuxiFer.App.Views;

/// <summary>
/// Dialog zum Bearbeiten der Layer-Parameter (per Doppelklick auf einen Layer).
/// Bearbeitet den übergebenen Layer direkt über Datenbindung.
/// </summary>
public partial class LayerEditDialog : Window
{
    public LayerEditDialog()
    {
        InitializeComponent();
    }

    public LayerEditDialog(Layer layer) : this()
    {
        DataContext = layer;
    }

    private void OnCloseClick(object? sender, RoutedEventArgs e) => Close();
}
