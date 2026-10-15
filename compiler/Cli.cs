namespace Lust;

/// The `lust` command.
public static class Cli
{
    const string Version = "0.4.1";

    const string Usage = """
        Lust 0.4.1

          lust run <file.lust>      check the file, then run it
          lust check <file.lust>    only check it, and list any mistakes
          lust build <file.lust>    make a program you can start without Lust, in a build folder next to the file
          lust cs <file.lust>       show the C# that Lust writes
          lust test <folder>        run the test programs in a folder
          lust version              show which Lust this is

        lust <file.lust> is the same as lust run <file.lust>.
        """;

    // Very long expressions nest deeply, so Lust runs on a thread with a big stack.
    public static int Main(string[] args)
    {
        // Started by double-clicking (or by dropping a file on lust.exe)? Then the window
        // belongs to Lust alone and would vanish the moment Lust is done, so keep it open.
        bool ownWindow = OpenedFromExplorer();
        if (ownWindow && args.Length == 0)
        {
            Console.WriteLine("""
                This is Lust. It runs from a terminal, or you can drop a .lust file onto lust.exe.

                To use it from a terminal, open PowerShell or Command Prompt in this folder and type:

                  .\lust run hello.lust

                """);
            args = new[] { "help" };
        }

        int code = 0;
        var t = new Thread(() => code = MainInner(args), 256 * 1024 * 1024);
        t.Start();
        t.Join();

        if (ownWindow)
        {
            Console.WriteLine();
            Console.Write("Press any key to close.");
            try { Console.ReadKey(true); } catch (InvalidOperationException) { }
        }
        return code;
    }

    [System.Runtime.InteropServices.DllImport("kernel32.dll")]
    static extern uint GetConsoleProcessList(uint[] list, uint count);

    /// True when no terminal started Lust: Windows made a fresh console just for it.
    static bool OpenedFromExplorer()
    {
        if (!OperatingSystem.IsWindows() || Console.IsInputRedirected || Console.IsOutputRedirected) return false;
        try { return GetConsoleProcessList(new uint[2], 2) == 1; }
        catch { return false; }
    }

    static int MainInner(string[] args)
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

    /// Reads and checks a file and everything it fetches. The program is null when there are mistakes.
    /// With fileNamesOnly, messages show just file names (for tests).
    public static (Compilation? program, IReadOnlyList<Diagnostic> mistakes) Compile(string path, bool fileNamesOnly = false)
    {
        var comp = Compilation.Load(path, full => fileNamesOnly ? Path.GetFileName(full) : DisplayName(full));
        return (comp.Diags.Any ? null : comp, comp.Diags.All);
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

    static Compilation? CompileAndReport(string path)
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
        if (program.UsesUi && !OperatingSystem.IsWindows())
        {
            Console.Error.WriteLine($"{program.Main.Source.Display} fetches ui, and windows only open on Windows. lust check still works here.");
            return 2;
        }
        var result = Runner.Run(program.Main.Source, Emitter.Emit(program), capture: false, ui: program.UsesUi, packages: program.Packages);
        if (result.Stderr.Length > 0) Console.Error.WriteLine(result.Stderr);
        return result.ExitCode;
    }

    static int CheckFile(string path)
    {
        var program = CompileAndReport(path);
        if (program == null) return 1;
        var others = program.Files.Count(f => !f.IsMain && !f.IsNative);
        Console.WriteLine(others == 0 ? $"{program.Main.Source.Display} looks good." : $"{program.Main.Source.Display} and the {others} file{(others == 1 ? "" : "s")} it fetches from look good.");
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
        var outDir = Path.Combine(Path.GetDirectoryName(program.Main.Source.Path)!, "build");
        Console.WriteLine($"Building {program.Main.Source.Display}…");
        var result = Runner.Build(program.Main.Source, Emitter.Emit(program), outDir, program.UsesUi, program.Packages);
        if (result.ExitCode != 0) { Console.Error.WriteLine(result.Stderr); return result.ExitCode; }
        var exe = Runner.AppName(program.Main.Source) + (OperatingSystem.IsWindows() ? ".exe" : "");
        Console.WriteLine($"Done: {Path.Combine(DisplayName(outDir), exe)}");
        return 0;
    }
}
