namespace Lust;

/// One .lust file: its text, and the name to show in messages.
public sealed class SourceFile
{
    public string Path { get; }      // full path on disk
    public string Display { get; }   // what messages show, like examples/player.lust
    public string Text { get; }

    public SourceFile(string path, string display, string text)
    {
        Path = path;
        Display = display;
        Text = text.Replace("\r\n", "\n");
    }

    /// The file's name without .lust: what other files call it.
    public string Module => System.IO.Path.GetFileNameWithoutExtension(Path);
}

/// A place in a file. Lines and columns count from 1.
public readonly record struct Pos(SourceFile File, int Line, int Col)
{
    public override string ToString() => $"{File.Display}:{Line}:{Col}";
}

public sealed record Diagnostic(Pos Pos, string Message)
{
    public override string ToString() => $"{Pos}  {Message}";
}

/// Collects mistakes. Lust keeps going after the first one, so you see several at once.
public sealed class Diagnostics
{
    readonly List<Diagnostic> list = new();
    public const int Max = 30;

    public IReadOnlyList<Diagnostic> All =>
        list.OrderBy(d => d.Pos.File.Display, StringComparer.Ordinal).ThenBy(d => d.Pos.Line).ThenBy(d => d.Pos.Col).ToList();

    public bool Any => list.Count > 0;
    public bool Full => list.Count >= Max;

    // Lines where the lexer already found a bad symbol or text. The parser's
    // complaints on those lines are knock-on effects, so they're left out.
    readonly HashSet<(SourceFile, int)> symbolLines = new();
    public void AddSymbol(Pos pos, string message)
    {
        symbolLines.Add((pos.File, pos.Line));
        Add(pos, message);
    }
    public bool SymbolMistakeOn(Pos pos) => symbolLines.Contains((pos.File, pos.Line));

    public void Add(Pos pos, string message, bool always = false)
    {
        // One message per spot is enough; a second one at the same place is usually a knock-on effect.
        if (!always && list.Any(d => d.Pos == pos)) return;
        if (!Full) list.Add(new Diagnostic(pos, message));
    }
}

/// Thrown to stop parsing a statement; the parser then skips ahead and carries on.
public sealed class ParseStop : Exception { }
