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
    /// Built into Lust (ui.lust): checked like any file, but its classes come from Lust's runtime.
    public bool IsNative { get; init; }

    public Dictionary<string, ClassSym> Classes { get; } = new();   // classes and objects
    public Dictionary<string, FuncSym> Funcs { get; } = new();
    public Dictionary<string, StateSym> States { get; } = new();    // "state", "season", "farm.state", "farm.season"
    public HashSet<string> Modules { get; } = new();                // files this one fetched from, by their own names
    public Dictionary<string, FileUnit> ModuleFiles { get; } = new();

    public List<ClassSym> OwnClasses { get; } = new();
    public List<FuncSym> OwnFuncs { get; } = new();
    public List<StateSym> OwnStates { get; } = new();
    public Dictionary<string, SaveSym> Saves { get; } = new();
    /// C# types brought in with fetch cs:, by their short name (Math for System.Math).
    public Dictionary<string, System.Type> CsTypes { get; } = new();
    public List<SaveSym> OwnSaves { get; } = new();
    /// The variables declared at the very top of the file (not inside a block), by name.
    public Dictionary<string, Variable> TopScope { get; set; } = new();
    /// save and load lines at the top level, to check they come after the variables they use.
    public List<(Pos Pos, SaveSym Save)> TopSaveUses { get; } = new();
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
    /// NuGet packages the program uses, with their versions.
    public Dictionary<string, string> Packages { get; } = new(StringComparer.OrdinalIgnoreCase);
    /// Whether the program fetches the built-in ui file, so it needs Windows and WinForms.
    public bool UsesUi => Files.Any(f => f.IsNative && f.Module == "ui");

    static readonly StringComparison PathCase = OperatingSystem.IsWindows() ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;

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
                if (f.IsCs) continue;
                var other = System.IO.Path.GetFullPath(System.IO.Path.Combine(dir, f.Path.Replace('/', System.IO.Path.DirectorySeparatorChar) + ".lust"));
                if (other.Equals(unit.Source.Path, PathCase))
                {
                    c.Diags.Add(f.ModulePos, $"{unit.Source.Display} can't {f.Word} from itself.");
                    continue;
                }
                // fetch ui always means Lust's built-in UI. A ui.lust of your own would quietly
                // hide Window, Button and the rest, so it's a mistake instead.
                if (f.Path == "ui")
                {
                    if (File.Exists(other))
                        c.Diags.Add(f.ModulePos, $"fetch ui brings in Lust's built-in windows and buttons, but there's also a ui.lust next to {System.IO.Path.GetFileName(unit.Source.Path)}. Give that file another name.");
                    f.Unit = c.Files.FirstOrDefault(u => u.IsNative && u.Module == "ui");
                    if (f.Unit == null) queue.Enqueue(f.Unit = c.AddBuiltIn("ui"));
                    continue;
                }
                var known = c.Files.FirstOrDefault(u => !u.IsNative && u.Source.Path.Equals(other, PathCase));
                if (known != null) { f.Unit = known; continue; }
                if (!File.Exists(other))
                {
                    var here = System.IO.Path.GetFileName(unit.Source.Path);
                    c.Diags.Add(f.ModulePos, f.Path.Contains('/')
                        ? (Directory.Exists(System.IO.Path.GetDirectoryName(other))
                            ? $"there's no file {f.Path}.lust, looking from the folder {here} is in."
                            : $"there's no folder {f.Path[..f.Path.LastIndexOf('/')]}, looking from the folder {here} is in.")
                        : $"there's no file called {f.Module}.lust next to {here}.");
                    continue;
                }
                queue.Enqueue(f.Unit = c.Add(other, display, isMain: false));
            }
        }
        // Files are numbered in C# (C_F2_Player), so names from different files can never collide.
        for (int i = 0; i < c.Files.Count; i++) c.Files[i].CsModule = c.Files[i].IsNative ? "Ui" : $"F{i + 1}";
        Checker.Check(c);
        return c;
    }

    FileUnit AddBuiltIn(string module)
    {
        using var stream = typeof(Compilation).Assembly.GetManifestResourceStream(module + ".lust")!;
        var text = new StreamReader(stream).ReadToEnd();
        var file = new SourceFile($"<built in>/{module}.lust", $"{module}.lust (built into Lust)", text);
        var unit = new FileUnit { Source = file, Ast = Parser.Parse(file, Diags), IsNative = true };
        Files.Add(unit);
        return unit;
    }

    FileUnit Add(string full, Func<string, string> display, bool isMain)
    {
        var file = new SourceFile(full, display(full), File.ReadAllText(full));
        var unit = new FileUnit { Source = file, Ast = Parser.Parse(file, Diags), IsMain = isMain };
        Files.Add(unit);
        return unit;
    }
}
