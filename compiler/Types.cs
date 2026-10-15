namespace Lust;

/// A Lust type. Classes, arrays, cArrays and state come in later milestones.
public sealed class LType
{
    public string Name { get; }
    public string Cs { get; }   // the C# type it becomes
    LType(string name, string cs) { Name = name; Cs = cs; }

    public static readonly LType Int = new("int", "long");
    public static readonly LType Float = new("float", "double");
    public static readonly LType String = new("string", "string");
    public static readonly LType Truth = new("truth", "bool");
    public static readonly LType Nothing = new("nothing", "object");
    public static readonly LType None = new("nothing at all", "void");  // what a call with no result gives
    public static readonly LType Error = new("?", "object");           // after a mistake; never reported twice

    public bool IsNumber => this == Int || this == Float;
    public override string ToString() => Name;

    public static LType? FromName(string name) => name switch
    {
        "int" => Int,
        "float" => Float,
        "string" => String,
        "truth" => Truth,
        _ => null,
    };

    /// Can a value of type `from` go where `to` is wanted? An int fits where a float goes.
    public static bool Fits(LType from, LType to) =>
        from == to || from == Error || to == Error || (from == Int && to == Float);

    /// "an int", "a string": for messages.
    public string WithArticle => Name switch
    {
        "int" => "an int",
        "nothing at all" => "nothing at all",
        _ => "a " + Name,
    };
}
