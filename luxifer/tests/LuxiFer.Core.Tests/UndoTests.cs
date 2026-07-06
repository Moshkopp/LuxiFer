using LuxiFer.Core.Canvas;
using LuxiFer.Core.Undo;

namespace LuxiFer.Core.Tests;

public class UndoTests
{
    private static (CanvasDocument Doc, Layer Layer) NewDoc()
    {
        var doc = new CanvasDocument();
        var layer = Layer.CreateNext(0);
        doc.Layers.Add(layer);
        return (doc, layer);
    }

    [Fact]
    public void Execute_fuehrt_aus_und_ermoeglicht_Undo()
    {
        var (_, layer) = NewDoc();
        var stack = new UndoStack();
        var rect = new RectangleObject { X = 0, Y = 0, Width = 10, Height = 10 };

        stack.Execute(new AddObjectCommand(layer, rect));

        Assert.Single(layer.Objects);
        Assert.True(stack.CanUndo);
        Assert.False(stack.CanRedo);

        stack.Undo();
        Assert.Empty(layer.Objects);
        Assert.True(stack.CanRedo);

        stack.Redo();
        Assert.Single(layer.Objects);
    }

    [Fact]
    public void Neues_Command_verwirft_Redo_Zweig()
    {
        var (_, layer) = NewDoc();
        var stack = new UndoStack();
        stack.Execute(new AddObjectCommand(layer, new RectangleObject { Width = 1, Height = 1 }));
        stack.Undo();
        Assert.True(stack.CanRedo);

        stack.Execute(new AddObjectCommand(layer, new EllipseObject { Width = 1, Height = 1 }));
        Assert.False(stack.CanRedo);
    }

    [Fact]
    public void RemoveObject_Undo_stellt_Position_wieder_her()
    {
        var (_, layer) = NewDoc();
        var a = new RectangleObject { Width = 1, Height = 1 };
        var b = new EllipseObject { Width = 1, Height = 1 };
        var c = new LineObject { X2 = 1, Y2 = 1 };
        layer.Objects.AddRange([a, b, c]);

        var stack = new UndoStack();
        stack.Execute(new RemoveObjectCommand(layer, b));
        Assert.Equal([a, c], layer.Objects);

        stack.Undo();
        Assert.Equal([a, b, c], layer.Objects); // b wieder an Index 1
    }

    [Fact]
    public void MoveObject_Undo_ist_verlustfrei_fuer_Linie()
    {
        var line = new LineObject { X = 0, Y = 0, X2 = 30, Y2 = 10 };
        var stack = new UndoStack();

        stack.Execute(new MoveObjectCommand(line, 5, -3));
        Assert.Equal((5, -3, 35, 7), (line.X, line.Y, line.X2, line.Y2));

        stack.Undo();
        Assert.Equal((0, 0, 30, 10), (line.X, line.Y, line.X2, line.Y2));
    }

    [Fact]
    public void ResizeObject_Undo_stellt_alte_Bounds_wieder_her()
    {
        var rect = new RectangleObject { X = 0, Y = 0, Width = 10, Height = 10 };
        var stack = new UndoStack();
        var before = rect.Bounds;

        rect.SetBounds(5, 5, 40, 20);
        stack.Push(new ResizeObjectCommand(rect, before, rect.Bounds));

        stack.Undo();
        Assert.Equal((0d, 0d, 10d, 10d), rect.Bounds);

        stack.Redo();
        Assert.Equal((5d, 5d, 40d, 20d), rect.Bounds);
    }

    [Fact]
    public void Labels_beschreiben_naechste_Aktion()
    {
        var (_, layer) = NewDoc();
        var stack = new UndoStack();
        stack.Execute(new AddObjectCommand(layer, new RectangleObject { Width = 1, Height = 1 }));

        Assert.Equal("Rechteck hinzufügen", stack.NextUndoLabel);
        stack.Undo();
        Assert.Equal("Rechteck hinzufügen", stack.NextRedoLabel);
    }

    [Fact]
    public void Changed_Event_feuert_bei_jeder_Aenderung()
    {
        var (_, layer) = NewDoc();
        var stack = new UndoStack();
        var count = 0;
        stack.Changed += (_, _) => count++;

        stack.Execute(new AddObjectCommand(layer, new RectangleObject { Width = 1, Height = 1 }));
        stack.Undo();
        stack.Redo();

        Assert.Equal(3, count);
    }
}
