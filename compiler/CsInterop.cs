using System.Reflection;
using System.Runtime.Loader;
using System.Text.Json;

namespace Lust;

/// The C# type a name like Math stands for, after fetch cs:System.Math; (used for its static members).
public sealed class CsStaticType : LType
{
    public System.Type Clr { get; }
    public CsStaticType(System.Type clr) { Clr = clr; }
    public override string Name => CsNames.Short(Clr);
    public override string Cs => CsNames.Of(Clr);
    public override string WithArticle => $"the C# type {Name}";
}

/// A C# method or constructor Lust decided to call, with what each argument becomes.
public sealed class CsCall
{
    public MethodBase Method { get; init; } = null!;
    public ParameterInfo[] Params { get; init; } = Array.Empty<ParameterInfo>();
    public bool IsStatic => Method.IsStatic || Method is ConstructorInfo;
    public System.Type Returns => Method is MethodInfo mi ? mi.ReturnType : Method.DeclaringType!;
}

/// A C# property or field.
public sealed class CsMember
{
    public MemberInfo Member { get; init; } = null!;
    public System.Type ValueType { get; init; } = null!;
    public bool IsStatic { get; init; }
    public bool CanSet { get; init; }
}

/// Finding C# types (from .NET itself or from NuGet packages), and matching Lust values to C# ones.
public static class CsLibs
{
    // ===== finding types =====

    static readonly object gate = new();

    public static (System.Type? Type, string? Error) Find(string name, string? package, string? version)
    {
        lock (gate)
        {
            if (package != null)
            {
                var (asms, error) = Package(package, version!);
                if (error != null) return (null, error);
                foreach (var a in asms)
                    if (a.GetType(name) is { } pt) return (pt, null);
                var t0 = FindBuiltIn(name);
                if (t0 != null) return (t0, null);
                return (null, $"the package {package} {version} has no type called {name}. Check the spelling and the namespace.");
            }
            var t = FindBuiltIn(name);
            if (t != null) return (t, null);
            if (Enumerable.Range(1, 4).Any(n => FindBuiltIn($"{name}`{n}") != null))
                return (null, $"{name} is a generic C# type (it needs <…>), and Lust can't use those yet. Lust's own arrays and maps do the jobs of List and Dictionary.");
            return (null, $"C# has no type called {name}. Check the spelling and the namespace (like System.Math). Types from NuGet packages need from \"Package\" 1.0.0 at the end.");
        }
    }

    static readonly Dictionary<string, System.Type?> builtIn = new();

    static System.Type? FindBuiltIn(string name)
    {
        if (builtIn.TryGetValue(name, out var cached)) return cached;
        var t = System.Type.GetType(name) ?? AppDomain.CurrentDomain.GetAssemblies().Select(a => a.GetType(name)).FirstOrDefault(x => x != null);
        if (t == null)
        {
            // .NET's own libraries: try the ones whose name starts the type's name first (System.Text.Json for System.Text.Json.JsonSerializer).
            var tpa = (AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES") as string)?.Split(Path.PathSeparator) ?? Array.Empty<string>();
            var names = tpa.Select(Path.GetFileNameWithoutExtension).Where(n => n != null).Select(n => n!).Distinct().ToList();
            var likely = names.Where(n => name.StartsWith(n + ".", StringComparison.Ordinal)).OrderByDescending(n => n.Length);
            foreach (var asmName in likely.Concat(names.Where(n => !name.StartsWith(n + ".", StringComparison.Ordinal))))
            {
                try
                {
                    var asm = AssemblyLoadContext.Default.LoadFromAssemblyName(new AssemblyName(asmName));
                    t = asm.GetType(name);
                    if (t != null) break;
                }
                catch { }
            }
        }
        if (t != null && !(t.IsPublic || t.IsNestedPublic)) t = null;
        builtIn[name] = t;
        return t;
    }

    // ===== NuGet packages =====

    sealed class PackageContext : AssemblyLoadContext
    {
        readonly Dictionary<string, string> paths;
        public PackageContext(Dictionary<string, string> paths) : base(isCollectible: false) { this.paths = paths; }
        protected override Assembly? Load(AssemblyName name) =>
            name.Name != null && paths.TryGetValue(name.Name, out var p) ? LoadFromAssemblyPath(p) : null;
    }

    static readonly Dictionary<string, (List<Assembly>, string?)> packages = new();

    /// Downloads the package (once; .NET keeps it), then loads its libraries to look inside them.
    static (List<Assembly>, string?) Package(string id, string version)
    {
        var key = $"{id}/{version}".ToLowerInvariant();
        if (packages.TryGetValue(key, out var cached)) return cached;
        var result = LoadPackage(id, version);
        packages[key] = result;
        return result;
    }

    static (List<Assembly>, string?) LoadPackage(string id, string version)
    {
        var dir = Path.Combine(Path.GetTempPath(), "lust", "packages", $"{Safe(id)}-{Safe(version)}");
        Directory.CreateDirectory(dir);
        var proj = Path.Combine(dir, "p.csproj");
        var assets = Path.Combine(dir, "obj", "project.assets.json");
        var text = $"""
            <Project Sdk="Microsoft.NET.Sdk">
              <PropertyGroup><TargetFramework>{Runner.Tfm}</TargetFramework></PropertyGroup>
              <ItemGroup><PackageReference Include="{System.Security.SecurityElement.Escape(id)}" Version="{System.Security.SecurityElement.Escape(version)}" /></ItemGroup>
            </Project>
            """;
        bool fresh = !File.Exists(proj) || File.ReadAllText(proj) != text || !File.Exists(assets);
        if (fresh)
        {
            File.WriteAllText(proj, text);
            var (code, output) = Runner.Capture(Runner.Dotnet, new[] { "restore", proj, "-nologo", "-v:q" }, dir);
            if (code != 0)
            {
                try { File.Delete(assets); } catch { }
                var why = output.Split('\n').Select(l => l.Trim()).FirstOrDefault(l => l.Contains("error", StringComparison.OrdinalIgnoreCase)) ?? output.Trim();
                if (why.Contains("NU1301")) why = "NuGet couldn't be reached. Is the internet on?";
                else if (why.Contains("NU1101")) why = $"NuGet has no package called {id}.";
                else if (why.Contains("NU1102")) why = $"NuGet has no version {version} of {id}.";
                return (new(), $"couldn't get the package {id} {version}. {why}");
            }
        }
        try
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(assets));
            var root = doc.RootElement;
            var folder = root.GetProperty("packageFolders").EnumerateObject().First().Name;
            var libraries = root.GetProperty("libraries");
            var target = root.GetProperty("targets").EnumerateObject().First().Value;
            var paths = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            var main = new List<string>();
            foreach (var lib in target.EnumerateObject())
            {
                if (lib.Value.GetProperty("type").GetString() != "package") continue;
                if (!lib.Value.TryGetProperty("runtime", out var runtime)) continue;
                var libPath = libraries.GetProperty(lib.Name).GetProperty("path").GetString()!;
                foreach (var file in runtime.EnumerateObject())
                {
                    if (!file.Name.EndsWith(".dll", StringComparison.OrdinalIgnoreCase)) continue;
                    var full = Path.Combine(folder, libPath, file.Name.Replace('/', Path.DirectorySeparatorChar));
                    if (!File.Exists(full)) continue;
                    paths[Path.GetFileNameWithoutExtension(full)] = full;
                    if (lib.Name.StartsWith(id + "/", StringComparison.OrdinalIgnoreCase)) main.Add(full);
                }
            }
            var ctx = new PackageContext(paths);
            var asms = new List<Assembly>();
            foreach (var p in main.Concat(paths.Values.Except(main)))
            {
                try { asms.Add(ctx.LoadFromAssemblyPath(p)); } catch { }
            }
            return (asms, null);
        }
        catch (Exception e)
        {
            return (new(), $"couldn't read the package {id} {version}: {e.Message}");
        }
    }

    static string Safe(string s) => new(s.Select(c => char.IsLetterOrDigit(c) || c is '.' or '-' or '_' ? c : '_').ToArray());

    // ===== Lust types and C# types =====

    static readonly System.Type[] IntLike = { typeof(long), typeof(int), typeof(short), typeof(byte), typeof(sbyte), typeof(ushort), typeof(uint), typeof(ulong) };
    static readonly System.Type[] FloatLike = { typeof(double), typeof(float), typeof(decimal) };

    /// The element of a list-like C# type that Lust turns into an array (T[], List<T>, IEnumerable<T>…), or null.
    public static System.Type? ListElement(System.Type t)
    {
        if (t == typeof(string)) return null;
        if (t.IsArray && t.GetArrayRank() == 1) return t.GetElementType();
        if (t.IsGenericType)
        {
            var def = t.GetGenericTypeDefinition();
            if (def == typeof(List<>) || def == typeof(IEnumerable<>) || def == typeof(IList<>) || def == typeof(ICollection<>) ||
                def == typeof(IReadOnlyList<>) || def == typeof(IReadOnlyCollection<>))
                return t.GetGenericArguments()[0];
        }
        return null;
    }

    /// What a C# value becomes in Lust, or null when Lust can't use it.
    public static LType? FromCs(System.Type t)
    {
        if (t == typeof(void)) return LType.None;
        if (t.IsByRef || t.IsPointer || t.IsGenericParameter || t.ContainsGenericParameters) return null;
        if (IntLike.Contains(t)) return LType.Int;
        if (FloatLike.Contains(t)) return LType.Float;
        if (t == typeof(string) || t == typeof(char)) return LType.String;
        if (t == typeof(bool)) return LType.Truth;
        if (Nullable.GetUnderlyingType(t) != null) return null;
        if (typeof(Delegate).IsAssignableFrom(t)) return null;
        if (t.IsByRefLike) return null;
        if (ListElement(t) is { } elem)
        {
            var e = FromCs(elem);
            return e == null || e == LType.None ? null : ArrayType.Of(e);
        }
        return CsType.Of(t);
    }

    /// How well a Lust value fits a C# parameter: 0 is perfect, bigger is a looser fit, null doesn't fit.
    public static int? Fit(LType arg, System.Type p)
    {
        if (p.IsByRef || p.IsPointer || p.ContainsGenericParameters) return null;
        if (p == typeof(object)) return arg == LType.None ? null : 20;
        if (arg == LType.Error) return 0;
        if (arg == LType.Nothing) return !p.IsValueType || Nullable.GetUnderlyingType(p) != null ? 10 : null;
        if (arg == LType.Int)
        {
            int i = Array.IndexOf(IntLike, p);
            if (i >= 0) return i;
            int f = Array.IndexOf(FloatLike, p);
            return f >= 0 ? 10 + f : null;
        }
        if (arg == LType.Float) { int f = Array.IndexOf(FloatLike, p); return f >= 0 ? f : null; }
        if (arg == LType.String) return p == typeof(string) ? 0 : p == typeof(char) ? 5 : null;
        if (arg == LType.Truth) return p == typeof(bool) ? 0 : null;
        if (arg is ArrayType at && ListElement(p) is { } pe && (p.IsArray || p.IsInterface))
        {
            var inner = Fit(at.Elem, pe);
            return inner == null ? null : 1 + inner;
        }
        if (arg is CsType ct && ct.Clr != null)
        {
            if (ct.Clr == p) return 0;
            if (p.IsAssignableFrom(ct.Clr)) return 2;
            return null;
        }
        return null;
    }

    /// C# for turning a Lust value (written as `code`) into what parameter type p wants.
    public static string ToCs(string code, LType from, System.Type p)
    {
        if (from == LType.Nothing) return $"(({CsNames.Of(p)})null)";
        if (p == typeof(object)) return $"(object)({code})";
        if (from == LType.Int)
        {
            if (p == typeof(long)) return code;
            return $"checked(({CsNames.Of(p)})({code}))";
        }
        if (from == LType.Float)
        {
            if (p == typeof(double)) return code;
            return $"(({CsNames.Of(p)})({code}))";
        }
        if (from == LType.String && p == typeof(char)) return $"LustCs.Char({code})";
        if (from is ArrayType at && ListElement(p) is { } pe)
        {
            var conv = ToCs("v", at.Elem, pe);
            return $"LustCs.ToArray<{at.Elem.Cs}, {CsNames.Of(pe)}>({code}, v => {conv})";
        }
        return code;
    }

    /// C# for turning what C# gave back (written as `code`, of type t) into a Lust value.
    public static string FromCsCode(string code, System.Type t)
    {
        if (t == typeof(long) || t == typeof(double) || t == typeof(bool)) return code;
        if (t == typeof(ulong)) return $"checked((long)({code}))";
        if (IntLike.Contains(t)) return $"((long)({code}))";
        if (FloatLike.Contains(t)) return $"((double)({code}))";
        if (t == typeof(string)) return $"LustCs.Str({code})";
        if (t == typeof(char)) return $"({code}).ToString()";
        if (ListElement(t) is { } elem)
        {
            var lust = FromCs(elem)!;
            return $"LustCs.FromList<{CsNames.Of(elem)}, {lust.Cs}>({code}, v => {FromCsCode("v", elem)})";
        }
        return code;
    }

    // ===== members =====

    const BindingFlags Statics = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy;
    const BindingFlags Instances = BindingFlags.Public | BindingFlags.Instance;

    /// Picks the overload that fits the arguments best. On failure, says why.
    public static (CsCall? Call, string? Error) Pick(IEnumerable<MethodBase> candidates, IReadOnlyList<LType> args, string what)
    {
        CsCall? best = null;
        int bestScore = int.MaxValue;
        var usable = new List<MethodBase>();
        foreach (var m in candidates)
        {
            if (m.IsGenericMethodDefinition || m.ContainsGenericParameters) continue;
            var ps = m.GetParameters();
            if (ps.Any(p => p.ParameterType.IsByRef || p.ParameterType.IsPointer)) continue;
            if (m is MethodInfo mi && FromCs(mi.ReturnType) == null) continue;
            usable.Add(m);
            int required = ps.Count(p => !p.HasDefaultValue && !p.IsOptional);
            if (args.Count < required || args.Count > ps.Length) continue;
            int score = 0;
            bool fits = true;
            for (int i = 0; i < args.Count && fits; i++)
            {
                var f = Fit(args[i], ps[i].ParameterType);
                if (f == null) fits = false; else score += f.Value;
            }
            if (!fits) continue;
            score += (ps.Length - args.Count) * 3;   // fewer left out is better
            if (score < bestScore) { bestScore = score; best = new CsCall { Method = m, Params = ps }; }
        }
        if (best != null) return (best, null);
        if (usable.Count == 0) return (null, null);
        var got = args.Count == 0 ? "nothing" : string.Join(", ", args.Select(a => a.Name));
        var options = usable.Select(Describe).Distinct().Take(6).ToList();
        return (null, $"{what} doesn't take ({got}). It takes {(options.Count == 1 ? options[0] : "one of: " + string.Join(" · ", options))}.");
    }

    /// A C# method's parameters, in Lust's words: (float x, int digits)
    public static string Describe(MethodBase m) =>
        "(" + string.Join(", ", m.GetParameters().Select(p => $"{(FromCs(p.ParameterType)?.Name ?? CsNames.Short(p.ParameterType))} {p.Name}{(p.HasDefaultValue ? "?" : "")}")) + ")";

    public static IEnumerable<MethodBase> Methods(System.Type t, string name, bool statics) =>
        t.GetMethods(statics ? Statics : Instances).Where(m => m.Name == name && !m.IsSpecialName)
            .Concat(t.IsInterface && !statics ? t.GetInterfaces().SelectMany(i => i.GetMethods(Instances)).Where(m => m.Name == name) : Enumerable.Empty<MethodInfo>());

    public static bool HasMethodNamed(System.Type t, string name) =>
        t.GetMethods(Statics | Instances).Any(m => m.Name == name);

    public static CsMember? Member(System.Type t, string name, bool statics)
    {
        var flags = statics ? Statics : Instances;
        var prop = t.GetProperties(flags).FirstOrDefault(p => p.Name == name && p.GetIndexParameters().Length == 0 && p.GetMethod?.IsPublic == true);
        if (prop != null)
            return new CsMember { Member = prop, ValueType = prop.PropertyType, IsStatic = statics, CanSet = prop.SetMethod?.IsPublic == true };
        var field = t.GetFields(flags).FirstOrDefault(f => f.Name == name);
        if (field != null)
            return new CsMember { Member = field, ValueType = field.FieldType, IsStatic = statics, CanSet = !field.IsInitOnly && !field.IsLiteral };
        return null;
    }

    /// Names that are close, for "did you mean".
    public static IEnumerable<string> MemberNames(System.Type t, bool statics) =>
        t.GetMembers(statics ? Statics : Instances).Where(m => m is MethodInfo { IsSpecialName: false } or PropertyInfo or FieldInfo).Select(m => m.Name).Distinct();
}
