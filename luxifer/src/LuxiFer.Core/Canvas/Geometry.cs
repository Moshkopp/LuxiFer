namespace LuxiFer.Core.Canvas;

/// <summary>
/// Reine Geometrie-Hilfen ohne UI-Bezug (im Core, damit ohne Avalonia testbar).
/// </summary>
public static class Geometry
{
    /// <summary>
    /// Dreht den Punkt (x,y) um das Zentrum (cx,cy) um <paramref name="degrees"/>
    /// (mathematisch positiv = im Uhrzeigersinn bei y-nach-unten-Achse).
    /// </summary>
    public static (double X, double Y) RotatePoint(
        double x, double y, double cx, double cy, double degrees)
    {
        if (degrees == 0) return (x, y);
        var rad = degrees * Math.PI / 180.0;
        var cos = Math.Cos(rad);
        var sin = Math.Sin(rad);
        var dx = x - cx;
        var dy = y - cy;
        return (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos);
    }
}
