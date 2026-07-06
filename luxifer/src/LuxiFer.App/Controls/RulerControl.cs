using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;

namespace LuxiFer.App.Controls;

public enum RulerOrientation
{
    Horizontal,
    Vertical,
}

/// <summary>
/// mm-Lineal am Canvas-Rand. Teilt sich Zoom und Pan-Offset mit dem
/// CanvasControl, damit die Skala deckungsgleich mit dem Grid läuft.
/// </summary>
public sealed class RulerControl : Control
{
    public static readonly StyledProperty<RulerOrientation> OrientationProperty =
        AvaloniaProperty.Register<RulerControl, RulerOrientation>(nameof(Orientation));

    public static readonly StyledProperty<double> ZoomPxPerMmProperty =
        AvaloniaProperty.Register<RulerControl, double>(nameof(ZoomPxPerMm), 1.0);

    /// <summary>Bildschirmposition (Pixel) des mm-Nullpunkts entlang der Achse.</summary>
    public static readonly StyledProperty<double> OriginOffsetProperty =
        AvaloniaProperty.Register<RulerControl, double>(nameof(OriginOffset));

    public RulerOrientation Orientation
    {
        get => GetValue(OrientationProperty);
        set => SetValue(OrientationProperty, value);
    }

    public double ZoomPxPerMm
    {
        get => GetValue(ZoomPxPerMmProperty);
        set => SetValue(ZoomPxPerMmProperty, value);
    }

    public double OriginOffset
    {
        get => GetValue(OriginOffsetProperty);
        set => SetValue(OriginOffsetProperty, value);
    }

    static RulerControl()
    {
        AffectsRender<RulerControl>(OrientationProperty, ZoomPxPerMmProperty, OriginOffsetProperty);
    }

    public override void Render(DrawingContext context)
    {
        var horizontal = Orientation == RulerOrientation.Horizontal;
        var length = horizontal ? Bounds.Width : Bounds.Height;
        var breadth = horizontal ? Bounds.Height : Bounds.Width;
        if (length <= 0 || ZoomPxPerMm <= 0) return;

        context.FillRectangle(new SolidColorBrush(Color.FromRgb(37, 37, 41)), new Rect(Bounds.Size));

        // Sinnvollen Beschriftungsschritt in mm wählen: 1,2,5 × 10^n,
        // sodass ein Schritt mindestens ~60 px breit ist.
        var step = NiceStep(60 / ZoomPxPerMm);
        var minorStep = step / 5;

        var tickPen = new Pen(new SolidColorBrush(Color.FromArgb(90, 255, 255, 255)));
        var labelBrush = new SolidColorBrush(Color.FromArgb(170, 255, 255, 255));
        var typeface = new Typeface(FontFamily.Default);

        // Erste sichtbare mm-Position (kann negativ sein)
        var firstMm = Math.Floor((0 - OriginOffset) / ZoomPxPerMm / minorStep) * minorStep;

        for (var mm = firstMm; ; mm += minorStep)
        {
            var pos = OriginOffset + mm * ZoomPxPerMm;
            if (pos > length) break;
            if (pos < 0) continue;

            var isMajor = Math.Abs(mm % step) < minorStep / 2;
            var tickLen = isMajor ? breadth * 0.55 : breadth * 0.30;

            if (horizontal)
                context.DrawLine(tickPen, new Point(pos, breadth - tickLen), new Point(pos, breadth));
            else
                context.DrawLine(tickPen, new Point(breadth - tickLen, pos), new Point(breadth, pos));

            if (isMajor)
            {
                var text = new FormattedText(
                    ((int)Math.Round(mm)).ToString(),
                    System.Globalization.CultureInfo.InvariantCulture,
                    FlowDirection.LeftToRight, typeface, 10, labelBrush);

                if (horizontal)
                    context.DrawText(text, new Point(pos + 2, 1));
                else
                {
                    // Vertikal: Text um 90° gedreht an der Tick-Position
                    var origin = new Point(1, pos - 2);
                    using (context.PushTransform(
                        Matrix.CreateRotation(-Math.PI / 2) * Matrix.CreateTranslation(origin.X, origin.Y)))
                        context.DrawText(text, new Point(-text.Width, 0));
                }
            }
        }
    }

    /// <summary>Rundet einen mm-Zielabstand auf einen „schönen" Wert 1/2/5·10^n.</summary>
    private static double NiceStep(double target)
    {
        if (target <= 0) return 1;
        var pow = Math.Pow(10, Math.Floor(Math.Log10(target)));
        var f = target / pow;
        var nice = f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10;
        return nice * pow;
    }
}
