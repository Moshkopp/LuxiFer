using Avalonia.Data.Converters;
using Avalonia.Media;

namespace LuxiFer.App.Controls;

public static class ToolConverters
{
    /// <summary>true, wenn der gebundene Wert dem ConverterParameter entspricht (für Tool-ToggleButtons).</summary>
    public static readonly IValueConverter Equals = new FuncValueConverter<object?, object?, bool>(
        (value, parameter) => value?.Equals(parameter) ?? false);

    /// <summary>"#RRGGBB" → Brush für Layer-Farbfelder.</summary>
    public static readonly IValueConverter HexToBrush = new FuncValueConverter<string?, IBrush>(
        hex => Color.TryParse(hex ?? "", out var c) ? new SolidColorBrush(c) : Brushes.Gray);
}
