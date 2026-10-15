// Lust's runtime. The compiler copies this file into every Lust program it builds.
#nullable disable
using System;
using System.Diagnostics;
using System.Globalization;
using System.Threading;

/// A mistake found while the program runs, like reading past the end of an array.
public sealed class LustError : Exception
{
    public LustError(string message) : base(message) { }
}

public static class LustRt
{
    // LUST_SEED makes random repeatable, for tests.
    static readonly Random rng = int.TryParse(Environment.GetEnvironmentVariable("LUST_SEED"), out var seed) ? new Random(seed) : new Random();

    // Each file's full path, and the name to show for it in messages.
    static readonly System.Collections.Generic.Dictionary<string, string> names = new();
    public static void Name(string path, string display) => names[path] = display;

    public static void Print(long v) => Console.WriteLine(Text(v));
    public static void Print(double v) => Console.WriteLine(Text(v));
    public static void Print(bool v) => Console.WriteLine(Text(v));
    public static void Print(string v) => Console.WriteLine(v);

    public static string Text(long v) => v.ToString(CultureInfo.InvariantCulture);
    public static string Text(bool v) => v ? "true" : "not true";

    /// Floats always show a point, so 3.0 doesn't look like the int 3.
    public static string Text(double v)
    {
        if (double.IsNaN(v)) return "not a number";
        if (double.IsPositiveInfinity(v)) return "infinity";
        if (double.IsNegativeInfinity(v)) return "-infinity";
        var s = v.ToString("R", CultureInfo.InvariantCulture);
        return s.Contains('.') || s.Contains('E') ? s : s + ".0";
    }

    public static long Int(double v)
    {
        if (double.IsNaN(v) || double.IsInfinity(v) || v >= 9.2233720368547758E18 || v < -9.2233720368547758E18)
            throw new LustError($"{Text(v)} is too big to be an int.");
        return (long)Math.Truncate(v);
    }

    public static long Random(long low, long high)
    {
        if (low > high) throw new LustError($"random's lowest number ({low}) is bigger than its highest ({high}).");
        return high == long.MaxValue ? rng.NextInt64(low, high) : rng.NextInt64(low, high + 1);
    }

    public static void Wait(double seconds)
    {
        if (double.IsNaN(seconds) || seconds < 0) throw new LustError($"can't wait {Text(seconds)} seconds.");
        Thread.Sleep(TimeSpan.FromSeconds(seconds));
    }

    /// Reports a mistake that happened while running, at the Lust line it came from, then stops.
    public static void Fail(Exception e)
    {
        Console.Out.Flush();
        string message = e switch
        {
            LustError => e.Message,
            DivideByZeroException => "divided by zero.",
            OverflowException => "a number got too big for an int.",
            _ => "something went wrong: " + e.Message,
        };
        string where = "";
        foreach (var frame in new StackTrace(e, true).GetFrames() ?? Array.Empty<StackFrame>())
        {
            var file = frame.GetFileName();
            if (file != null && file.EndsWith(".lust", StringComparison.OrdinalIgnoreCase) && frame.GetFileLineNumber() > 0)
            {
                where = $"{(names.TryGetValue(file, out var shown) ? shown : file)}:{frame.GetFileLineNumber()}  ";
                break;
            }
        }
        Console.Error.WriteLine(where + message);
        Environment.Exit(1);
    }
}
