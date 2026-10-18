using System.Text.Json.Serialization;

namespace DesktopLayoutManager.WinUI;

public enum ItemKind
{
    Shortcut,
    File,
    Folder,
    System,
    Unknown
}

public sealed class IconItem
{
    public int OriginalIndex { get; set; }
    public string Name { get; set; } = "";
    public int X { get; set; }
    public int Y { get; set; }
    public ItemKind Kind { get; set; }
    public bool LegacyIsFileOrFolder { get; set; }
}

public sealed class Snapshot
{
    public uint SchemaVersion { get; set; }
    public string Product { get; set; } = "";
    public decimal CapturedAtUnixMs { get; set; }
    public string Machine { get; set; } = "";
    public string Os { get; set; } = "";
    public int ScreenWidth { get; set; }
    public int ScreenHeight { get; set; }
    public List<IconItem> Icons { get; set; } = [];
}

public sealed class MoveItem
{
    public int Index { get; set; }
    public string Name { get; set; } = "";
    public int FromX { get; set; }
    public int FromY { get; set; }
    public int ToX { get; set; }
    public int ToY { get; set; }
}

public sealed class SnapshotReply
{
    public Snapshot Snapshot { get; set; } = new();
    public string? Path { get; set; }
}

public class FillPlanReply
{
    public Snapshot Snapshot { get; set; } = new();
    public List<MoveItem> Moves { get; set; } = [];
}

public class RestorePlanReply : FillPlanReply
{
    public Snapshot? SavedSnapshot { get; set; }
    public List<string> Skipped { get; set; } = [];
}

public sealed class OperationReply : RestorePlanReply
{
    public string? BackupPath { get; set; }
}

public sealed class ExportReply
{
    public int Count { get; set; }
    public string? Path { get; set; }
}

public sealed record DisplayRow(string Status, string Name, string Current, string Target);
