// Lust's runtime. The compiler copies this file into every Lust program it builds.
#nullable disable
using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Threading;

/// A mistake found while the program runs, like reading past the end of an array.
public sealed class LustError : Exception
{
    public LustError(string message) : base(message) { }
}

/// What every Lust class and object is built on.
public abstract class LustObject
{
    public abstract string LustName { get; }
    /// The thing with its fields, the way print shows it: Player { name "Ada", hp 20 }
    public abstract string LustShow();
    public override string ToString() => LustShow();
}

/// A Lust array: numbered from 0, and friendly about mistakes.
public sealed class LustArray<T> : IEnumerable<T>
{
    readonly List<T> items = new();

    public static LustArray<T> Of(params T[] values)
    {
        var a = new LustArray<T>();
        a.items.AddRange(values);
        return a;
    }

    public long Count => items.Count;

    void CheckPosition(long i)
    {
        if (items.Count == 0) throw new LustError($"the array is empty, so there's no item [{i}].");
        if (i < 0 || i >= items.Count) throw new LustError($"there's no item [{i}]. The array has {items.Count} item{(items.Count == 1 ? "" : "s")}, numbered 0 to {items.Count - 1}.");
    }

    public T Get(long i) { CheckPosition(i); return items[(int)i]; }
    public void Set(long i, T value) { CheckPosition(i); items[(int)i] = value; }
    public void Inject(T value) => items.Add(value);
    public void Remove(long i) { CheckPosition(i); items.RemoveAt((int)i); }

    /// Loops go through a copy, so changing the array inside the loop is safe.
    public T[] Snapshot() => items.ToArray();

    public IEnumerator<T> GetEnumerator() => items.GetEnumerator();
    IEnumerator IEnumerable.GetEnumerator() => items.GetEnumerator();
    public override string ToString() => "[" + string.Join(", ", items.Select(x => LustRt.Show(x))) + "]";
}

/// One row of a cArray. The compiler makes a class for each kind of row, with its columns.
public abstract class LustRow
{
    public long id;
    internal bool hasId;
    protected LustRow(bool hasId, long id) { this.hasId = hasId; this.id = id; }
}

/// A connected array: rows looked up by their id. Rows stay in the order they were added.
public sealed class LustCArray<R> where R : LustRow
{
    readonly Dictionary<long, R> byId = new();
    readonly List<R> order = new();

    public LustCArray(params R[] rows)
    {
        foreach (var r in rows) Inject(r);
    }

    public long Count => order.Count;

    public R Get(long id)
    {
        if (byId.TryGetValue(id, out var row)) return row;
        throw new LustError(order.Count == 0 ? $"there's no row with id {id}. The cArray is empty." : $"there's no row with id {id}. The ids are {string.Join(", ", order.Select(r => r.id))}.");
    }

    public void Inject(R row)
    {
        if (!row.hasId) row.id = order.Count == 0 ? 1 : Math.Max(order.Max(r => r.id) + 1, 1);
        else if (byId.ContainsKey(row.id)) throw new LustError($"there's already a row with id {row.id}.");
        row.hasId = true;
        byId[row.id] = row;
        order.Add(row);
    }

    public void Remove(long id)
    {
        var row = Get(id);
        byId.Remove(id);
        order.Remove(row);
    }

    public bool Has(long id) => byId.ContainsKey(id);
    public R[] Snapshot() => order.ToArray();
    public override string ToString() => "[" + string.Join(", ", order.Select(r => r.ToString())) + "]";
}

public static class LustRt
{
    // LUST_SEED makes random repeatable, for tests.
    static readonly Random rng = int.TryParse(Environment.GetEnvironmentVariable("LUST_SEED"), out var seed) ? new Random(seed) : new Random();

    // Each file's full path, and the name to show for it in messages.
    static readonly Dictionary<string, string> names = new();
    public static void Name(string path, string display) => names[path] = display;

    public static void Print(long v) => Console.WriteLine(Text(v));
    public static void Print(double v) => Console.WriteLine(Text(v));
    public static void Print(bool v) => Console.WriteLine(Text(v));
    public static void Print(string v) => Console.WriteLine(v ?? "nothing");
    public static void Print(object v) => Console.WriteLine(Text(v));

    public static string Text(long v) => v.ToString(CultureInfo.InvariantCulture);
    public static string Text(bool v) => v ? "true" : "not true";
    public static string Text(Enum v) => OptionName(v);
    static string OptionName(Enum v) { var s = v.ToString(); return s.StartsWith("o_") ? s.Substring(2) : s; }

    public static long Div(long a, long b) { if (b == 0) throw new DivideByZeroException(); return checked(a / b); }
    public static long Mod(long a, long b) { if (b == 0) throw new DivideByZeroException(); return a % b; }
    public static long Same(long v) => v;

    /// How many letters, counting an emoji or an accented letter as one.
    public static long Letters(string s) => new StringInfo(s).LengthInTextElements;

    /// Floats always show a point, so 3.0 doesn't look like the int 3.
    public static string Text(double v)
    {
        if (double.IsNaN(v)) return "not a number";
        if (double.IsPositiveInfinity(v)) return "infinity";
        if (double.IsNegativeInfinity(v)) return "-infinity";
        var s = v.ToString("R", CultureInfo.InvariantCulture);
        int e = s.IndexOf('E');
        if (e >= 0) return s.Substring(0, e).Contains('.') ? s : s.Substring(0, e) + ".0" + s.Substring(e);
        return s.Contains('.') ? s : s + ".0";
    }

    /// Anything, the way print shows it.
    public static string Text(object v) => v switch
    {
        null => "nothing",
        string s => s,
        long l => Text(l),
        double d => Text(d),
        bool b => Text(b),
        Enum e => OptionName(e),
        _ => v.ToString(),
    };

    /// A value inside something else (a field, an array item): text gets quotes, and
    /// things made from classes show just their class, so print doesn't go in circles.
    public static string Show(object v) => v switch
    {
        string s => "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\n", "\\n") + "\"",
        LustObject o => o.LustName,
        _ => Text(v),
    };

    public static long Int(double v)
    {
        if (double.IsNaN(v) || double.IsInfinity(v) || v >= 9.2233720368547758E18 || v < -9.2233720368547758E18)
            throw new LustError($"{Text(v)} is too big to be an int.");
        return (long)Math.Truncate(v);
    }

    public static bool IsNumber(string s) => double.TryParse(s?.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var d) && double.IsFinite(d);

    public static long ParseInt(string s)
    {
        var t = s?.Trim() ?? "";
        if (long.TryParse(t, NumberStyles.Integer, CultureInfo.InvariantCulture, out var v)) return v;
        if (double.TryParse(t, NumberStyles.Float, CultureInfo.InvariantCulture, out var d) && double.IsFinite(d)) return Int(d);
        throw new LustError($"\"{s}\" isn't a number. Check with .isNumber() first.");
    }

    public static double ParseFloat(string s)
    {
        if (double.TryParse(s?.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var v) && double.IsFinite(v)) return v;
        throw new LustError($"\"{s}\" isn't a number. Check with .isNumber() first.");
    }

    public static long Random(long low, long high)
    {
        if (low > high) throw new LustError($"random's lowest number ({low}) is bigger than its highest ({high}).");
        return high == long.MaxValue ? rng.NextInt64(low, high) : rng.NextInt64(low, high + 1);
    }

    /// Shows the question and waits for a line to be typed. Gives back "" when there's no more input.
    public static string Ask(string question)
    {
        if (!string.IsNullOrEmpty(question))
        {
            Console.Write(question.EndsWith(" ") ? question : question + " ");
            Console.Out.Flush();
        }
        return Console.ReadLine() ?? "";
    }

    public static void Wait(double seconds)
    {
        if (double.IsNaN(seconds) || seconds < 0) throw new LustError($"can't wait {Text(seconds)} seconds.");
        Thread.Sleep(TimeSpan.FromSeconds(seconds));
    }

    // Calls deep inside each other. Endless recursion becomes a Lust message instead of a crash.
    [ThreadStatic] static int depth;
    public static void Enter()
    {
        if (++depth > 20000) { depth = 0; throw new LustError("a function kept calling itself and never stopped (over 20000 calls deep)."); }
    }
    public static void Leave() => depth--;

    /// Runs the program on a thread with plenty of room for deep calls.
    public static void Start(Action run)
    {
        Exception failure = null;
        var t = new Thread(() => { try { run(); } catch (Exception e) { failure = e; } }, 512 * 1024 * 1024);
        t.Start();
        t.Join();
        if (failure != null) Fail(failure);
    }

    /// Reports a mistake that happened while running, at the Lust line it came from, then stops.
    public static void Fail(Exception e)
    {
        Console.Out.Flush();
        if (e is TypeInitializationException { InnerException: not null } tie) e = tie.InnerException;
        string message = e switch
        {
            LustError => e.Message,
            DivideByZeroException => "divided by zero.",
            OverflowException => "a number got too big for an int.",
            NullReferenceException => "used something that is nothing. A field or method was used on nothing.",
            InvalidCastException => "something wasn't the kind of thing it was supposed to be.",
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
