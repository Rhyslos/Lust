namespace Lust;

/// The `lust` command.
public static class Cli
{
    const string Version = "0.2.0";

    const string Usage = """
        Lust 0.2.0

          lust run <file.lust>      check the file, then run it
          lust check <file.lust>    only check it, and list any mistakes
          lust build <file.lust>    make a program you can start without Lust, in a build folder next to the file
          lust cs <file.lust>       show the C# that Lust writes
          lust test <folder>        run the test programs in a folder
          lust version              show which Lust this is

        lust <file.lust> is the same as lust run <file.lust>.
        """;

    public static int Main(string[] args)
    {
        Console.OutputEncoding = System.Text.Encoding.UTF8;
        try
        {
            if (args.Length == 0 || args[0] is "help" or "-h" or "--help" or "/?") { Console.WriteLine(Usage); return 0; }
            if (args[0] is "version" or "--version" or "-v") { Console.WriteLine($"lust {Version}"); return 0; }
            if (args[0].EndsWith(".lust", StringComparison.OrdinalIgnoreCase)) return RunFile(args[0]);
            if (args.Length < 2) { Console.Error.WriteLine($"lust {args[0]} needs a file or folder after it.\n\n{Usage}"); return 2; }
            return args[0] switch
            {
                "run" => RunFile(args[1]),
                "check" => CheckFile(args[1]),
                "build" => BuildFile(args[1]),
                "cs" => ShowCs(args[1]),
                "test" => Tests.RunAll(args[1]),
                _ => Unknown(args[0]),
            };
        }
        catch (Exception e) when (e is not OutOfMemoryException)
        {
            Console.Error.WriteLine($"Lust itself hit a problem. That's a bug in Lust.\n{e}");
            return 3;
        }
    }

    static int Unknown(string command)
    {
        Console.Error.WriteLine($"lust doesn't know the command {command}.\n\n{Usage}");
        return 2;
    }

    /// Reads and checks a file. Returns null (after printing the mistakes) when it isn't ready to run.
    public static (Program? program, IReadOnlyList<Diagnostic> mistakes) Compile(string path, string? display = null)
    {
        var full = Path.GetFullPath(path);
        var text = File.ReadAllText(full);
        display ??= DisplayName(full);
        var file = new SourceFile(full, display, text);
        var diags = new Diagnostics();
        var program = Parser.Parse(file, diags);
        // Check types even after a structural mistake, so all the mistakes show at once.
        Checker.Check(program, diags);
        return (diags.Any ? null : program, diags.All);
    }

    static string DisplayName(string full)
    {
        var rel = Path.GetRelativePath(Environment.CurrentDirectory, full);
        return rel.StartsWith("..") ? full : rel;
    }

    static bool FileOk(string path)
    {
        if (!path.EndsWith(".lust", StringComparison.OrdinalIgnoreCase))
        {
            Console.Error.WriteLine($"{path} isn't a .lust file.");
            return false;
        }
        if (!File.Exists(path))
        {
            Console.Error.WriteLine($"there's no file called {path}.");
            return false;
        }
        return true;
    }

    static Program? CompileAndReport(string path)
    {
        if (!FileOk(path)) return null;
        var (program, mistakes) = Compile(path);
        if (program != null) return program;
        foreach (var d in mistakes) Console.Error.WriteLine(d);
        Console.Error.WriteLine(mistakes.Count == 1 ? "\n1 mistake. Nothing was run." : $"\n{mistakes.Count} mistakes{(mistakes.Count >= Diagnostics.Max ? " (stopped counting)" : "")}. Nothing was run.");
        return null;
    }

    static int RunFile(string path)
    {
        var program = CompileAndReport(path);
        if (program == null) return 1;
        var result = Runner.Run(program.File, Emitter.Emit(program), capture: false);
        if (result.Stderr.Length > 0) Console.Error.WriteLine(result.Stderr);
        return result.ExitCode;
    }

    static int CheckFile(string path)
    {
        var program = CompileAndReport(path);
        if (program == null) return 1;
        Console.WriteLine($"{program.File.Display} looks good.");
        return 0;
    }

    static int ShowCs(string path)
    {
        var program = CompileAndReport(path);
        if (program == null) return 1;
        Console.Write(Emitter.Emit(program));
        return 0;
    }

    static int BuildFile(string path)
    {
        var program = CompileAndReport(path);
        if (program == null) return 1;
        var outDir = Path.Combine(Path.GetDirectoryName(program.File.Path)!, "build");
        Console.WriteLine($"Building {program.File.Display}…");
        var result = Runner.Build(program.File, Emitter.Emit(program), outDir);
        if (result.ExitCode != 0) { Console.Error.WriteLine(result.Stderr); return result.ExitCode; }
        var exe = Runner.AppName(program.File) + (OperatingSystem.IsWindows() ? ".exe" : "");
        Console.WriteLine($"Done: {Path.Combine(DisplayName(outDir), exe)}");
        return 0;
    }
}
