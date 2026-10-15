using System.Diagnostics;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;

namespace Lust;

/// Builds the C# that Lust wrote with .NET, then runs it.
///
/// Each Lust program gets its own small project in a temp folder. It's kept
/// between runs, so .NET only rebuilds what changed.
public static class Runner
{
    public sealed record Result(int ExitCode, string Stdout, string Stderr);

    /// Target the installed SDK's own .NET version: its files come with the SDK, so nothing is downloaded.
    static string Tfm => tfm ??= FindTfm();
    static string? tfm;

    static string FindTfm()
    {
        try
        {
            var (code, output) = Capture(Dotnet, new[] { "--version" }, Environment.CurrentDirectory);
            var major = output.Trim().Split('.')[0];
            if (code == 0 && int.TryParse(major, out var m) && m >= 6) return $"net{m}.0";
        }
        catch { }
        return $"net{Environment.Version.Major}.0";
    }

    /// Lust programs never use NuGet packages, so restore looks in an empty folder instead of
    /// asking nuget.org. That keeps runs quick and lets them work offline.
    static string EmptyFeed
    {
        get
        {
            var dir = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "lust", "no-packages");
            Directory.CreateDirectory(dir);
            return dir;
        }
    }

    static string Dotnet
    {
        get
        {
            var host = Environment.GetEnvironmentVariable("DOTNET_HOST_PATH");
            if (!string.IsNullOrEmpty(host) && File.Exists(host)) return host;
            var self = Environment.ProcessPath;
            if (self != null && System.IO.Path.GetFileNameWithoutExtension(self).Equals("dotnet", StringComparison.OrdinalIgnoreCase)) return self;
            return "dotnet";
        }
    }

    public static string RuntimeSource()
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("LustRt.cs")
            ?? throw new InvalidOperationException("the Lust runtime is missing from lust itself.");
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }

    /// The program's own name, safe to use for a .exe.
    public static string AppName(SourceFile file)
    {
        var name = new string(file.Module.Where(c => char.IsLetterOrDigit(c) || c is '_' or '-').ToArray());
        return name.Length == 0 ? "program" : name;
    }

    static string WorkDir(SourceFile file)
    {
        var hash = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(file.Path))).Substring(0, 16).ToLowerInvariant();
        return System.IO.Path.Combine(System.IO.Path.GetTempPath(), "lust", $"{AppName(file)}-{hash}");
    }

    static void WriteIfChanged(string path, string text)
    {
        if (File.Exists(path) && File.ReadAllText(path) == text) return;
        File.WriteAllText(path, text);
    }

    /// Writes the project. Returns its folder.
    static string Prepare(SourceFile file, string csharp)
    {
        var dir = WorkDir(file);
        Directory.CreateDirectory(dir);
        WriteIfChanged(System.IO.Path.Combine(dir, "app.csproj"), $"""
            <Project Sdk="Microsoft.NET.Sdk">
              <!-- Made by Lust for {System.Security.SecurityElement.Escape(file.Display)}. -->
              <PropertyGroup>
                <OutputType>Exe</OutputType>
                <TargetFramework>{Tfm}</TargetFramework>
                <AssemblyName>{AppName(file)}</AssemblyName>
                <Nullable>disable</Nullable>
                <ImplicitUsings>disable</ImplicitUsings>
                <CheckForOverflowUnderflow>true</CheckForOverflowUnderflow>
                <InvariantGlobalization>true</InvariantGlobalization>
                <RollForward>Major</RollForward>
                <SatelliteResourceLanguages>en</SatelliteResourceLanguages>
                <GenerateAssemblyInfo>false</GenerateAssemblyInfo>
                <DebugType>portable</DebugType>
                <TreatWarningsAsErrors>false</TreatWarningsAsErrors>
              </PropertyGroup>
            </Project>

            """);
        WriteIfChanged(System.IO.Path.Combine(dir, "Program.cs"), csharp);
        WriteIfChanged(System.IO.Path.Combine(dir, "LustRt.cs"), RuntimeSource());
        return dir;
    }

    static (int code, string output) Capture(string exe, IEnumerable<string> args, string cwd)
    {
        var psi = new ProcessStartInfo(exe) { WorkingDirectory = cwd, RedirectStandardOutput = true, RedirectStandardError = true, UseShellExecute = false };
        foreach (var a in args) psi.ArgumentList.Add(a);
        psi.Environment["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1";
        psi.Environment["DOTNET_NOLOGO"] = "1";
        psi.Environment["DOTNET_SKIP_FIRST_TIME_EXPERIENCE"] = "1";
        psi.Environment["DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE"] = "1";
        using var proc = Process.Start(psi)!;
        var err = proc.StandardError.ReadToEndAsync();
        var output = proc.StandardOutput.ReadToEnd();
        proc.WaitForExit();
        return (proc.ExitCode, output + err.Result);
    }

    /// Builds (or publishes) the project, restoring from the empty folder first,
    /// and from the normal package sources if .NET turns out to need something after all.
    // `dotnet msbuild -restore` does the same as `dotnet build`, without the build command's own
    // check-in with nuget.org on every run.
    static (int code, string output) DotnetBuild(string dir, params string[] extra)
    {
        var proj = System.IO.Path.Combine(dir, "app.csproj");
        var common = new[] { "msbuild", proj, "-restore", "-nologo", "-v:q", "-clp:NoSummary" }.Concat(extra);
        var (code, output) = Capture(Dotnet, common.Append($"-p:RestoreSources={EmptyFeed}"), dir);
        if (code != 0 && (output.Contains("NU1101") || output.Contains("NU1102") || output.Contains("NETSDK1")))
            (code, output) = Capture(Dotnet, common, dir);
        return (code, output);
    }

    /// When .NET refuses what Lust wrote, that's Lust's mistake, not the program's.
    static string BuildFailure(string output, string dir) =>
        "Lust wrote C# that .NET couldn't build. That's a bug in Lust, not in your program.\n" +
        string.Join("\n", output.Split('\n').Where(l => l.Contains("error", StringComparison.OrdinalIgnoreCase)).Select(l => "  " + l.Trim()).Distinct().Take(10)) +
        $"\nThe C# is in {dir}";

    /// Builds and runs. With capture, the program's output is collected instead of shown.
    public static Result Run(SourceFile file, string csharp, bool capture, IDictionary<string, string>? env = null, string? input = null)
    {
        var dir = Prepare(file, csharp);
        var (code, output) = DotnetBuild(dir, "-p:Configuration=Debug", "-p:UseAppHost=false");
        if (code != 0) return new Result(3, "", BuildFailure(output, dir));

        var dll = System.IO.Path.Combine(dir, "bin", "Debug", Tfm, AppName(file) + ".dll");
        var psi = new ProcessStartInfo(Dotnet) { WorkingDirectory = Environment.CurrentDirectory, UseShellExecute = false, RedirectStandardOutput = capture, RedirectStandardError = capture, RedirectStandardInput = capture };
        psi.ArgumentList.Add(dll);
        if (env != null) foreach (var (k, v) in env) psi.Environment[k] = v;
        using var proc = Process.Start(psi)!;
        if (!capture)
        {
            proc.WaitForExit();
            return new Result(proc.ExitCode, "", "");
        }
        if (input != null) proc.StandardInput.Write(input);
        proc.StandardInput.Close();
        var err = proc.StandardError.ReadToEndAsync();
        var stdout = proc.StandardOutput.ReadToEnd();
        proc.WaitForExit();
        return new Result(proc.ExitCode, stdout, err.Result);
    }

    /// Makes a program you can start without Lust, in a build folder next to the .lust file.
    public static Result Build(SourceFile file, string csharp, string outDir)
    {
        var dir = Prepare(file, csharp);
        var (code, output) = DotnetBuild(dir, "-t:Publish", "-p:Configuration=Release", $"-p:PublishDir={outDir.TrimEnd('/', '\\')}{System.IO.Path.DirectorySeparatorChar}");
        return code == 0 ? new Result(0, "", "") : new Result(3, "", BuildFailure(output, dir));
    }
}
