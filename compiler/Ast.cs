namespace Lust;

// The shape of a Lust program after parsing. The checker fills in types and
// what each name refers to, for the C# writer.

public abstract class Node
{
    public Pos Pos { get; init; }
}

// ----- types as written -----

public enum TypeKind { Named, Array, CArray, Map, Func }

/// `int`, `Player`, `farm.season`, `int array`, `cArray(string itemName, int amount)`.
public sealed class TypeRef : Node
{
    public TypeKind Kind { get; init; }
    public string Name { get; init; } = "";       // Named: the type's name
    public string? Module { get; init; }          // Named: `farm` in `farm.season`
    public TypeRef? Elem { get; init; }           // Array, Map; for Func what it gives back
    public List<ParamDecl> Columns { get; init; } = new();  // CArray; for Func its parameter types

    public override string ToString() => Kind switch
    {
        TypeKind.Array => $"{Elem} array",
        TypeKind.Map => $"{Elem} map",
        TypeKind.Func => $"do{(Elem != null ? " " + Elem : "")}({string.Join(", ", Columns.Select(c => c.Type.ToString()))})",
        TypeKind.CArray => $"cArray({string.Join(", ", Columns.Select(c => $"{c.Type} {c.Name}"))})",
        _ => Module != null ? $"{Module}.{Name}" : Name,
    };
}

public sealed record ParamDecl(TypeRef Type, string Name, Pos Pos);

// ----- declarations -----

public sealed class ClassDecl : Node
{
    public string Name { get; init; } = "";
    public bool IsObject { get; init; }
    public string? Parent { get; init; }
    public Pos ParentPos { get; init; }
    public List<FieldDecl> Fields { get; } = new();
    public List<FuncDecl> Methods { get; } = new();
    public ClassSym? Sym { get; set; }
}

/// One field in a has block. State fields (`state is idle;`) have IsState set,
/// with Type naming the state and Default the starting option.
public sealed class FieldDecl : Node
{
    public TypeRef Type { get; init; } = null!;
    public string Name { get; init; } = "";
    public Expr? Default { get; init; }
    public bool IsState { get; init; }
    public FieldSym? Sym { get; set; }
}

public sealed class FuncDecl : Node
{
    public string Name { get; init; } = "";
    public Pos NamePos { get; init; }
    public TypeRef? Returns { get; init; }
    public List<ParamDecl> Params { get; } = new();
    public Block Body { get; set; } = null!;
    public bool IsMethod { get; init; }
    public FuncSym? Sym { get; set; }
}

public sealed class StateDecl : Node
{
    public string Name { get; init; } = "state";
    public List<(string Name, Pos Pos)> Options { get; } = new();
}

public sealed class SendDecl : Node
{
    public List<(string Name, Pos Pos)> Names { get; } = new();
}

/// `save game = [Farm, Player, bag];`: what a save holds.
public sealed class SaveDecl : Node
{
    public string Name { get; init; } = "";
    public Pos NamePos { get; init; }
    public List<(string Name, Pos Pos)> Items { get; } = new();
    public SaveSym? Sym { get; set; }
}

/// `save game;` and `load game;`
public sealed class SaveStmt : Stmt
{
    public string Name { get; init; } = "";
    public Pos NamePos { get; init; }
    public bool IsLoad { get; init; }
    public SaveSym? Sym { get; set; }
}

/// `saved game`: has it been saved before?
public sealed class SavedExpr : Expr
{
    public string Name { get; init; } = "";
    public Pos NamePos { get; init; }
    public SaveSym? Sym { get; set; }
}

/// `fetch dog:Dog;`, `fetch dog:[A, B];`, `fetch dog;`, `get dice:roll;`
public sealed class FetchDecl : Node
{
    public string Module { get; init; } = "";      // the file's own name: farm in world/farm
    public string Path { get; init; } = "";        // as written: world/farm, ../game/farm, or just farm
    public Pos ModulePos { get; init; }
    public List<(string Name, Pos Pos)>? Names { get; init; }   // null: everything the file sends
    public bool IsGet { get; init; }
    public string Word => IsGet ? "get" : "fetch";
    public FileUnit? Unit { get; set; }            // the file it points to, once found

    // fetch cs:System.Math;   fetch cs:Raylib_cs.Raylib from "Raylib-cs" 6.1.1;
    public bool IsCs { get; init; }
    public string CsType { get; init; } = "";
    public string? Package { get; init; }
    public string? Version { get; init; }
}

// ----- expressions -----

public abstract class Expr : Node
{
    public LType Type { get; set; } = LType.Error;
}

public sealed class IntLit : Expr { public long Value { get; init; } }
public sealed class FloatLit : Expr { public double Value { get; init; } public string Text { get; init; } = ""; }
public sealed class StrLit : Expr { public string Value { get; init; } = ""; }
public sealed class TruthLit : Expr { }                       // `true`; `not true` is Not(TruthLit)
public sealed class NothingLit : Expr { }

public enum NameKind { Unresolved, Local, Field, Object, Option, Function, Method, CsType }

public sealed class Name : Expr
{
    public string Id { get; init; } = "";
    public NameKind Kind { get; set; }
    public Variable? Var { get; set; }
    public FieldSym? Field { get; set; }
    public ClassSym? Object { get; set; }
    public StateSym? State { get; set; }   // for an option: which state it belongs to
    public FuncSym? Func { get; set; }     // for a function or method used as a value: greet, bark
}

public enum UnOp { Neg, Not }
public sealed class Unary : Expr { public UnOp Op { get; init; } public Expr Operand { get; init; } = null!; }

public enum BinOp { Add, Sub, Mul, Div, Mod, Lt, Gt, Le, Ge, And, Or }
public sealed class Binary : Expr
{
    public BinOp Op { get; init; }
    public Expr Left { get; init; } = null!;
    public Expr Right { get; init; } = null!;
    public Pos OpPos { get; init; }
}

/// `a b` or `a not b` inside a question: are they the same?
public sealed class Same : Expr
{
    public Expr Left { get; init; } = null!;
    public Expr Right { get; init; } = null!;
    public bool Negated { get; init; }
}

/// `x like Wizard` inside a question.
public sealed class IsLike : Expr
{
    public Expr Value { get; init; } = null!;
    public string Class { get; init; } = "";
    public Pos ClassPos { get; init; }
    public bool Negated { get; init; }
    public ClassSym? Sym { get; set; }
}

/// `either check? a or b`
public sealed class Either : Expr
{
    public Expr Check { get; init; } = null!;
    public Expr Yes { get; init; } = null!;
    public Expr No { get; init; } = null!;
}

/// A call by name: a built-in (`print(x)`), a `do` function, or (inside a method) one of its own methods.
public sealed class Call : Expr
{
    public string Callee { get; init; } = "";
    public List<Expr> Args { get; init; } = new();
    public FuncSym? Func { get; set; }
    /// When the name holds a function (a variable, parameter or field): what to call.
    public Expr? Value { get; set; }
}

/// Calling a function that's kept somewhere other than a name: handlers[0](5)
public sealed class CallValue : Expr
{
    public Expr Target { get; init; } = null!;
    public List<Expr> Args { get; init; } = new();
}

/// `bo:bark` without (): the method, kept as a value to call later.
public sealed class MethodRef : Expr
{
    public Expr Target { get; init; } = null!;
    public string Method { get; init; } = "";
    public Pos MethodPos { get; init; }
    public FuncSym? Func { get; set; }
}

/// `x.text()`, `x.int()`, `x.float()`, `x.isNumber()`: built-in conversions.
public sealed class Conversion : Expr
{
    public Expr Target { get; init; } = null!;
    public string Method { get; init; } = "";
    public Pos MethodPos { get; init; }
}

public enum FieldKind { Unresolved, Field, Column, State, Option, Cs }

/// `bo.hp`, `bag[2].itemName`
public sealed class FieldGet : Expr
{
    public Expr Target { get; init; } = null!;
    public string Field { get; init; } = "";
    public Pos FieldPos { get; init; }
    public FieldKind Kind { get; set; }
    public FieldSym? Sym { get; set; }
    public StateSym? OptionState { get; set; }   // for an option written with a dot: fighting.blocking
    public string OptionPath { get; set; } = "";
    public CsMember? Cs { get; set; }             // a C# property or field
}

public enum MethodKind { Unresolved, User, ArrayInject, ArrayRemove, CArrayInject, CArrayRemove, CArrayHas, MapHas, MapRemove, CallField, Cs }

/// `bo:bark()`, `scores:inject(4)`
public sealed class MethodCall : Expr
{
    public Expr Target { get; init; } = null!;
    public string Method { get; init; } = "";
    public Pos MethodPos { get; init; }
    public List<Expr> Args { get; init; } = new();
    public MethodKind Kind { get; set; }
    public FuncSym? Func { get; set; }
    public FieldSym? Field { get; set; }   // CallField: a field that holds a function
    public object? Cs { get; set; }        // Cs: the C# method (a CsCall)
}

/// `scores[0]`, `bag[2]`
public sealed class Index : Expr
{
    public Expr Target { get; init; } = null!;
    public Expr Key { get; init; } = null!;
}

/// `#scores`
public sealed class Length : Expr { public Expr Target { get; init; } = null!; }

/// `new Dog { name "Bo", hp 12 }`
public sealed class New : Expr
{
    public string Class { get; init; } = "";
    public Pos ClassPos { get; init; }
    public List<(string Name, Pos Pos, Expr Value)> Inits { get; } = new();
    public ClassSym? Sym { get; set; }
    public List<FieldSym> InitFields { get; } = new();
    // new Random(42): C# types can take arguments.
    public List<Expr>? Args { get; init; }
    public CsCall? CsCtor { get; set; }
    public System.Type? CsClr { get; set; }
    public List<CsMember> CsInits { get; } = new();
}

/// `[1, 2, 3]`, or a cArray row `[1, "sword", 1]`. Its type comes from where it's used.
public sealed class ArrayLit : Expr
{
    public List<Expr> Items { get; } = new();
    public List<Expr>? Keys { get; set; }  // a map: ["apple" 3, "seed" 1] has keys "apple", "seed" and items 3, 1
    public bool RowHasId { get; set; }     // for a row: whether the id was written
}

// ----- statements -----

public abstract class Stmt : Node { }

public sealed class Block : Stmt { public List<Stmt> Stmts { get; init; } = new(); public Pos Close { get; init; } }

public sealed class VarDecl : Stmt
{
    public TypeRef TypeRef { get; init; } = null!;
    public string Id { get; init; } = "";
    public Pos IdPos { get; init; }
    public Expr Init { get; init; } = null!;
    public Variable? Var { get; set; }
}

/// `x = …;`, `bo.hp = …;`, `scores[0] = …;`, `bag[2].amount = …;`
public sealed class Assign : Stmt
{
    public Expr Target { get; init; } = null!;
    public Expr Value { get; init; } = null!;
}

/// `state is fighting;`, `bo.state is fighting;`, `farm.season is winter;`
public sealed class StateSet : Stmt
{
    public Expr Target { get; init; } = null!;
    public string Option { get; init; } = "";
    public Pos OptionPos { get; init; }
    public StateSym? State { get; set; }
}

public sealed class ExprStmt : Stmt { public Expr Expr { get; init; } = null!; }

public sealed class Wait : Stmt { public Expr Seconds { get; init; } = null!; }

public sealed class Give : Stmt { public Expr? Value { get; init; } }

/// `is …? then { } or is …? then { } or { }`
public sealed class IsChain : Stmt
{
    public List<(Expr Check, Block Body)> Branches { get; init; } = new();
    public Block? Otherwise { get; init; }
}

public enum LoopKind { Forever, Times, While, Each }
public enum EachKind { Array, CArray, State, Map }

public sealed class Loop : Stmt
{
    public LoopKind Kind { get; init; }
    public Expr? Expr { get; init; }
    public Block Body { get; init; } = null!;
    // For `loop (int n in scores)`
    public List<ParamDecl> Vars { get; init; } = new();
    public List<Variable> LoopVars { get; } = new();
    public EachKind Each { get; set; }
    public StateSym? EachState { get; set; }
}

public sealed class Stop : Stmt { }
public sealed class Skip : Stmt { }

// ----- a whole file -----

public sealed class Program
{
    public SourceFile File { get; init; } = null!;
    public List<Stmt> Stmts { get; } = new();
    public List<ClassDecl> Classes { get; } = new();
    public List<FuncDecl> Funcs { get; } = new();
    public List<StateDecl> States { get; } = new();
    public List<SendDecl> Sends { get; } = new();
    public List<FetchDecl> Fetches { get; } = new();
    public List<SaveDecl> Saves { get; } = new();
    public HashSet<string> BrokenNames { get; } = new();
}

/// A local variable or parameter. CsName is what it's called in the generated C#.
public sealed class Variable
{
    public string Id { get; init; } = "";
    public LType Type { get; init; } = LType.Error;
    public Pos Pos { get; init; }
    public string CsName { get; init; } = "";
    public bool TopLevel { get; init; }
    public bool ReadOnly { get; init; }   // a loop's current item
    /// Kept in a save, so it lives as a static field (LustProgram.v_x_3) that the save can reach.
    public bool Hoisted { get; set; }
}
