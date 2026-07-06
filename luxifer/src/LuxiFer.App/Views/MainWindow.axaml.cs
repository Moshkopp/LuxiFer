using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using LuxiFer.App.Controls;
using LuxiFer.App.ViewModels;

namespace LuxiFer.App.Views;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();

        Canvas.PointerMillimeterMoved += (_, mm) => ViewModel?.ReportCursor(mm.X, mm.Y);
        Canvas.DocumentChanged += (_, _) => ViewModel?.MarkDirty();

        DataContextChanged += (_, _) =>
        {
            if (ViewModel is { } vm)
                vm.CanvasInvalidateRequested += (_, _) => Canvas.InvalidateVisual();
        };

        KeyDown += OnWindowKeyDown;
    }

    private MainWindowViewModel? ViewModel => DataContext as MainWindowViewModel;

    private void OnWindowKeyDown(object? sender, KeyEventArgs e)
    {
        // Kürzel nicht auslösen, während in einem Textfeld getippt wird
        if (e.KeyModifiers != KeyModifiers.None || ViewModel is null) return;
        if (FocusManager?.GetFocusedElement() is TextBox) return;

        var tool = e.Key switch
        {
            Key.V => CanvasTool.Select,
            Key.R => CanvasTool.Rectangle,
            Key.E => CanvasTool.Ellipse,
            Key.L => CanvasTool.Line,
            Key.P => CanvasTool.Polyline,
            Key.G => CanvasTool.Polygon,
            _ => (CanvasTool?)null,
        };
        if (tool is { } t)
        {
            ViewModel.SelectToolCommand.Execute(t);
            e.Handled = true;
        }
    }

    private void OnLayerVisibilityClick(object? sender, RoutedEventArgs e) =>
        Canvas.InvalidateVisual();

    private void OnZoomToFitClick(object? sender, RoutedEventArgs e) =>
        Canvas.ZoomToFit();

    private void OnExitClick(object? sender, RoutedEventArgs e) => Close();
}
