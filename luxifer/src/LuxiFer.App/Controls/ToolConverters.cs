using Avalonia.Data.Converters;
using Avalonia.Media;
using LuxiFer.Core.Canvas;

namespace LuxiFer.App.Controls;

public static class ToolConverters
{
    /// <summary>true, wenn der gebundene Wert dem ConverterParameter entspricht (für Tool-ToggleButtons).</summary>
    public static readonly IValueConverter Equals = new FuncValueConverter<object?, object?, bool>(
        (value, parameter) => value?.Equals(parameter) ?? false);

    /// <summary>"#RRGGBB" → Brush für Layer-Farbfelder.</summary>
    public static readonly IValueConverter HexToBrush = new FuncValueConverter<string?, IBrush>(
        hex => Color.TryParse(hex ?? "", out var c) ? new SolidColorBrush(c) : Brushes.Gray);

    /// <summary>Layer-Modus als kurzer deutscher Text für die Layer-Tabelle.</summary>
    public static readonly IValueConverter ModeLabel = new FuncValueConverter<LayerMode, string>(
        mode => mode switch
        {
            LayerMode.Cut => "Schneiden",
            LayerMode.Fill => "Füllen",
            LayerMode.Raster => "Raster",
            _ => mode.ToString(),
        });
}
