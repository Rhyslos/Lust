namespace Lust;

/// One .lust file in a program, with everything it can see: its own classes,
/// functions and states, plus what it fetched from other files.
public sealed class FileUnit
{
    public SourceFile Source { get; init; } = null!;
    public Program Ast { get; init; } = null!;
    public string Module => Source.Module;
    public string CsModule { get; set; } = "";
    public bool IsMain { get; init; }

    public Dictionary<string, ClassSym> Classes { get; } = new();   // classes and objects
    public Dictionary<string, FuncSym> Funcs { get; } = new();
    public Dictionary<string, StateSym> States { get; } = new();    // "state", "season", "farm.state", "farm.season"
    public HashSet<string> Modules { get; } = new();                // files this one fetched from

    public List<ClassSym> OwnClasses { get; } = new();
    public List<FuncSym> OwnFuncs { get; } = new();
    public List<StateSym> OwnStates { get; } = new();
    public Dictionary<string, Pos> Sent { get; } = new();
    public HashSet<string> TopVars { get; } = new();
}

/// A whole program: the file you run and every file it fetches from.
public sealed class Compilation
{
    public FileUnit Main { get; private set; } = null!;
    public List<FileUnit> Files { get; } = new();
    public Diagnostics Diags { get; } = new();
    public HashSet<CArrayType> CArrays { get; } = new();

    public FileUnit? FileFor(string module, FileUnit from) =>
        Files.FirstOrDefault(f => f.Module.Equals(module, StringComparison.OrdinalIgnoreCase) && f != from);

    /// Reads the file and everything it fetches, then checks it all.
    /// `display` turns a full path into the name shown in messages.
    public static Compilation Load(string path, Func<string, string> display)
    {
        var c = new Compilation();
        var full = System.IO.Path.GetFullPath(path);
        c.Main = c.Add(full, display, isMain: true);
        var queue = new Queue<FileUnit>();
        queue.Enqueue(c.Main);
        while (queue.Count > 0)
        {
            var unit = queue.Dequeue();
            var dir = System.IO.Path.GetDirectoryName(unit.Source.Path)!;
            foreach (var f in unit.Ast.Fetches)
            {
                if (f.Module.Equals(unit.Module, StringComparison.OrdinalIgnoreCase))
                {
                    c.Diags.Add(f.ModulePos, $"{unit.Source.Display} can't {f.Word} from itself.");
                    continue;
                }
                if (c.FileFor(f.Module, unit) != null) continue;
                var other = System.IO.Path.Combine(dir, f.Module + ".lust");
                if (!File.Exists(other))
                {
                    c.Diags.Add(f.ModulePos, $"there's no file called {f.Module}.lust next to {System.IO.Path.GetFileName(unit.Source.Path)}.");
                    continue;
                }
                queue.Enqueue(c.Add(System.IO.Path.GetFullPath(other), display, isMain: false));
            }
        }
        // Files are numbered in C# (C_F2_Player), so names from different files can never collide.
        for (int i = 0; i < c.Files.Count; i++) c.Files[i].CsModule = $"F{i + 1}";
        Checker.Check(c);
        return c;
    }

    FileUnit Add(string full, Func<string, string> display, bool isMain)
    {
        var file = new SourceFile(full, display(full), File.ReadAllText(full));
        var unit = new FileUnit { Source = file, Ast = Parser.Parse(file, Diags), IsMain = isMain };
        Files.Add(unit);
        return unit;
    }
}
