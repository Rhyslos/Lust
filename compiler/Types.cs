namespace Lust;

/// A Lust type. Each one knows its Lust name (for messages) and the C# type it becomes.
/// Types are shared: there's one `int array` type, one type per class, and so on,
/// so two types are the same exactly when they're the same object.
public abstract class LType
{
    public abstract string Name { get; }
    public abstract string Cs { get; }
    public override string ToString() => Name;

    /// "an int", "a Player": for messages.
    public virtual string WithArticle => ("aeiou".Contains(char.ToLowerInvariant(Name[0])) ? "an " : "a ") + Name;

    public static readonly LType Int = new PrimType("int", "long");
    public static readonly LType Float = new PrimType("float", "double");
    public static readonly LType String = new PrimType("string", "string");
    public static readonly LType Truth = new PrimType("truth", "bool");
    public static readonly LType Nothing = new PrimType("nothing", "object", "nothing");
    public static readonly LType None = new PrimType("nothing at all", "void", "nothing at all");
    public static readonly LType Error = new PrimType("?", "object");

    public bool IsNumber => this == Int || this == Float;

    public static LType? Primitive(string name) => name switch
    {
        "int" => Int,
        "float" => Float,
        "string" => String,
        "truth" => Truth,
        _ => null,
    };

    /// Can a value of type `from` go where `to` is wanted?
    public static bool Fits(LType from, LType to)
    {
        if (from == to || from == Error || to == Error) return true;
        if (from == Int && to == Float) return true;
        if (from == Nothing && to is ClassType) return true;
        if (from is ClassType fc && to is ClassType tc) return fc.Sym.IsLike(tc.Sym);
        return false;
    }
}

public sealed class PrimType : LType
{
    readonly string name, cs;
    readonly string? article;
    public PrimType(string name, string cs, string? article = null) { this.name = name; this.cs = cs; this.article = article; }
    public override string Name => name;
    public override string Cs => cs;
    public override string WithArticle => article ?? base.WithArticle;
}

public sealed class ClassType : LType
{
    public ClassSym Sym { get; }
    public ClassType(ClassSym sym) { Sym = sym; }
    public override string Name => Sym.Name;
    public override string Cs => Sym.CsName;
}

public sealed class ArrayType : LType
{
    public LType Elem { get; }
    ArrayType(LType elem) { Elem = elem; }
    public override string Name => Elem.Name + " array";
    public override string Cs => $"LustArray<{Elem.Cs}>";
    public override string WithArticle => (Elem is CArrayType ? "a " : base.WithArticle.Split(' ')[0] + " ") + Name;

    static readonly Dictionary<LType, ArrayType> made = new();
    public static ArrayType Of(LType elem)
    {
        lock (made)
        {
            if (!made.TryGetValue(elem, out var t)) made[elem] = t = new ArrayType(elem);
            return t;
        }
    }
}

/// A connected array. Its rows have an id and the named columns.
public sealed class CArrayType : LType
{
    public IReadOnlyList<(string Name, LType Type)> Columns { get; }
    public int Number { get; }
    public RowType Row { get; }
    CArrayType(IReadOnlyList<(string, LType)> columns, int number) { Columns = columns; Number = number; Row = new RowType(this); }
    public override string Name => $"cArray({string.Join(", ", Columns.Select(c => $"{c.Type.Name} {c.Name}"))})";
    public override string Cs => $"LustCArray<{Row.Cs}>";
    public override string WithArticle => "a " + Name;

    public LType? Column(string name) => name == "id" ? Int : Columns.FirstOrDefault(c => c.Name == name).Type;

    // Shared by every compile in this process (tests compile many programs at once), so
    // column types are compared as the exact same type objects, not by name.
    static readonly List<CArrayType> made = new();
    public static CArrayType Of(IReadOnlyList<(string, LType)> columns)
    {
        lock (made)
        {
            foreach (var t in made)
                if (t.Columns.Count == columns.Count && t.Columns.Zip(columns).All(p => p.First.Name == p.Second.Item1 && p.First.Type == p.Second.Item2))
                    return t;
            var n = new CArrayType(columns, made.Count + 1);
            made.Add(n);
            return n;
        }
    }
}

/// One row of a cArray, as in `bag[2]`.
public sealed class RowType : LType
{
    public CArrayType Table { get; }
    public RowType(CArrayType table) { Table = table; }
    public override string Name => "row";
    public override string Cs => $"R_{Table.Number}";
}

public sealed class StateType : LType
{
    public StateSym Sym { get; }
    public StateType(StateSym sym) { Sym = sym; }
    public override string Name => Sym.Display;
    public override string Cs => Sym.CsName;
    public override string WithArticle => $"a {Sym.Display} option";
}

// ----- symbols: the things a program declares -----

public sealed class ClassSym
{
    public string Name { get; init; } = "";
    public FileUnit File { get; init; } = null!;
    public ClassDecl Decl { get; init; } = null!;
    public bool IsObject => Decl.IsObject;
    public ClassSym? Parent { get; set; }
    public ClassType Type { get; }
    public string CsName => $"{(IsObject ? "O" : "C")}_{File.CsModule}_{Name}";
    public string Kind => IsObject ? "object" : "class";

    /// Every field, including ones from the class it's like, in order.
    public List<FieldSym> Fields { get; } = new();
    public Dictionary<string, FuncSym> Methods { get; } = new();
    /// Methods written in this class (not inherited), and field starting values it sets.
    public List<FuncSym> OwnMethods { get; } = new();
    public List<(FieldSym Field, Expr Value)> Defaults { get; } = new();
    public bool Built { get; set; }
    public bool Building { get; set; }

    public ClassSym() { Type = new ClassType(this); }

    public FieldSym? Field(string name) => Fields.FirstOrDefault(f => f.Name == name);

    public bool IsLike(ClassSym other)
    {
        for (var c = this; c != null; c = c.Parent) if (c == other) return true;
        return false;
    }
}

public sealed class FieldSym
{
    public string Name { get; init; } = "";
    public LType Type { get; init; } = LType.Error;
    public ClassSym Owner { get; init; } = null!;
    public Pos Pos { get; init; }
    public string CsName => "f_" + Name;
}

public sealed class FuncSym
{
    public string Name { get; init; } = "";
    public FileUnit File { get; init; } = null!;
    public FuncDecl Decl { get; init; } = null!;
    public ClassSym? Owner { get; init; }
    public List<(string Name, LType Type)> Params { get; } = new();
    public LType Return { get; set; } = LType.None;
    public bool IsOverride { get; set; }
    public List<Variable> ParamVars { get; } = new();
    public string CsName => Owner != null ? "m_" + Name : $"M_{File.CsModule}.F_{Name}";
    public string Signature => $"{(Return == LType.None ? "" : Return.Name + " ")}{Name}({string.Join(", ", Params.Select(p => $"{p.Type.Name} {p.Name}"))})";
}

public sealed class StateSym
{
    public string Name { get; init; } = "";        // "state" for a file's main state
    public FileUnit File { get; init; } = null!;
    public StateDecl Decl { get; init; } = null!;
    public List<string> Options { get; } = new();
    public StateType Type { get; }
    public string CsName => $"S_{File.CsModule}_{Name}";
    /// What it's called in messages: "state", "season", or "farm.season" when it comes from another file.
    public string Display { get; set; } = "";

    public StateSym() { Type = new StateType(this); }
}
