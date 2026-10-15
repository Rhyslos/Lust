namespace Lust;

/// `lust test <folder>`: runs every test program in a folder.
///
/// Next to each test.lust is what should happen:
///   test.out   what it prints when it runs
///   test.err   the mistakes Lust reports, or the error it stops with while running
/// A .lust file with neither is a helper for other tests and is skipped.
public static class Tests
{
    public static int RunAll(string folder)
    {
        if (!Directory.Exists(folder)) { Console.Error.WriteLine($"there's no folder called {folder}."); return 2; }
        var files = Directory.GetFiles(folder, "*.lust", SearchOption.AllDirectories)
            .Where(f => File.Exists(Path.ChangeExtension(f, ".out")) || File.Exists(Path.ChangeExtension(f, ".err")))
            .OrderBy(f => f, StringComparer.Ordinal).ToList();
        if (files.Count == 0) { Console.Error.WriteLine($"no tests in {folder}."); return 2; }

        var results = new (string name, string? problem)[files.Count];
        Parallel.For(0, files.Count, new ParallelOptions { MaxDegreeOfParallelism = Math.Clamp(Environment.ProcessorCount / 2, 1, 4) }, i =>
        {
            var f = files[i];
            string problem;
            try { problem = RunOne(f); }
            catch (Exception e) { problem = "Lust itself crashed:\n" + e; }
            results[i] = (Path.GetRelativePath(folder, f), problem.Length == 0 ? null : problem);
        });

        int failed = 0;
        foreach (var (name, problem) in results)
        {
            if (problem == null) { Console.WriteLine($"  ok    {name}"); continue; }
            failed++;
            Console.WriteLine($"  FAIL  {name}");
            foreach (var line in problem.Split('\n')) Console.WriteLine("        " + line);
        }
        Console.WriteLine(failed == 0 ? $"\nAll {files.Count} tests passed." : $"\n{failed} of {files.Count} tests failed.");
        return failed == 0 ? 0 : 1;
    }

    static string Clean(string s) => string.Join("\n", s.Replace("\r\n", "\n").TrimEnd().Split('\n').Select(l => l.TrimEnd()));

    static string Compare(string expected, string actual, string what) =>
        Clean(expected) == Clean(actual) ? "" : $"{what} should be:\n{Indent(Clean(expected))}\nbut was:\n{Indent(Clean(actual))}";

    static string Indent(string s) => string.Join("\n", s.Split('\n').Select(l => "  | " + l));

    static string RunOne(string path)
    {
        var outFile = Path.ChangeExtension(path, ".out");
        var errFile = Path.ChangeExtension(path, ".err");
        var (program, mistakes) = Cli.Compile(path, fileNamesOnly: true);

        if (program == null)
        {
            var got = string.Join("\n", mistakes);
            if (!File.Exists(errFile)) return "Lust found mistakes:\n" + Indent(got);
            return Compare(File.ReadAllText(errFile), got, "the mistakes");
        }

        var result = Runner.Run(program.Main.Source, Emitter.Emit(program), capture: true, new Dictionary<string, string> { ["LUST_SEED"] = "1" });
        if (result.ExitCode == 3) return result.Stderr;
        if (File.Exists(errFile))
        {
            if (result.ExitCode == 0) return "it should have stopped with an error, but ran fine. It printed:\n" + Indent(Clean(result.Stdout));
            var problem = Compare(File.ReadAllText(errFile), result.Stderr, "the error");
            if (problem.Length > 0 || !File.Exists(outFile)) return problem;
        }
        else if (result.ExitCode != 0)
        {
            return $"it stopped with an error:\n{Indent(Clean(result.Stderr))}";
        }
        return File.Exists(outFile) ? Compare(File.ReadAllText(outFile), result.Stdout, "the output") : "";
    }
}
