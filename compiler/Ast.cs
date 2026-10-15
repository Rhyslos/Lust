namespace Lust;

// The shape of a Lust program after parsing. The checker fills in each expression's Type.

public abstract class Node
{
    public Pos Pos { get; init; }
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
public sealed class Name : Expr
{
    public string Id { get; init; } = "";
    public Variable? Var { get; set; }
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

/// `either check? a or b`
public sealed class Either : Expr
{
    public Expr Check { get; init; } = null!;
    public Expr Yes { get; init; } = null!;
    public Expr No { get; init; } = null!;
}

/// A call to a built-in or a function: `print(x)`, `random(1, 6)`.
public sealed class Call : Expr
{
    public string Callee { get; init; } = "";
    public List<Expr> Args { get; init; } = new();
}

/// `x.text()`, `x.int()`, `x.float()`: built-in conversions.
public sealed class Conversion : Expr
{
    public Expr Target { get; init; } = null!;
    public string Method { get; init; } = "";
    public Pos MethodPos { get; init; }
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

public sealed class Assign : Stmt
{
    public Name Target { get; init; } = null!;
    public Expr Value { get; init; } = null!;
}

public sealed class ExprStmt : Stmt { public Expr Expr { get; init; } = null!; }

public sealed class Wait : Stmt { public Expr Seconds { get; init; } = null!; }

/// `is …? then { } or is …? then { } or { }`
public sealed class IsChain : Stmt
{
    public List<(Expr Check, Block Body)> Branches { get; init; } = new();
    public Block? Otherwise { get; init; }
}

public enum LoopKind { Forever, Times, While }
public sealed class Loop : Stmt
{
    public LoopKind Kind { get; init; }
    public Expr? Expr { get; init; }
    public Block Body { get; init; } = null!;
}

public sealed class Stop : Stmt { }
public sealed class Skip : Stmt { }

/// A type as written: `int`, `float`, `string`, `truth`.
public sealed class TypeRef : Node { public string Name { get; init; } = ""; }

public sealed class Program
{
    public SourceFile File { get; init; } = null!;
    public List<Stmt> Stmts { get; init; } = new();
    public HashSet<string> BrokenNames { get; } = new();
}

/// A declared variable. CsName is what it's called in the generated C#.
public sealed class Variable
{
    public string Id { get; init; } = "";
    public LType Type { get; init; } = LType.Error;
    public Pos Pos { get; init; }
    public string CsName { get; init; } = "";
}
