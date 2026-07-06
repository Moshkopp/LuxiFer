using LuxiFer.Core.Canvas;

namespace LuxiFer.Core.Projects;

/// <summary>Ein Projekt: Metadaten + Canvas + Referenzen auf Assets/Fonts/Materialprofile.</summary>
public sealed class Project
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public required string Name { get; set; }
    public DateTimeOffset CreatedAt { get; init; } = DateTimeOffset.UtcNow;
    public DateTimeOffset ModifiedAt { get; set; } = DateTimeOffset.UtcNow;
    public int Version { get; set; } = 1;

    public CanvasDocument Canvas { get; init; } = new();
    public List<Guid> AssetIds { get; } = [];
    public List<string> FontFamilies { get; } = [];
    public List<Guid> MaterialProfileIds { get; } = [];
}
