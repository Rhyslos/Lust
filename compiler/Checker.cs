namespace Lust;

/// Enforces Lust's rules: every name declared, every type right. It also
/// records each expression's type and each name's variable for the C# writer.
public sealed class Checker
{
    readonly Diagnostics diags;
    readonly List<Dictionary<string, Variable>> scopes = new();
    int loopDepth;
    int counter;
    HashSet<string> broken = new();

    Checker(Diagnostics diags) { this.diags = diags; }

    public static void Check(Program program, Diagnostics diags)
    {
        var c = new Checker(diags) { broken = program.BrokenNames };
        c.scopes.Add(new());
        foreach (var s in program.Stmts) c.Stmt(s);
    }

    void Error(Pos pos, string message) => diags.Add(pos, message);

    // ----- names -----

    Variable? Lookup(string id)
    {
        for (int i = scopes.Count - 1; i >= 0; i--)
            if (scopes[i].TryGetValue(id, out var v)) return v;
        return null;
    }

    Variable Declare(string id, LType type, Pos pos)
    {
        var existing = Lookup(id);
        if (existing != null) Error(pos, $"{id} is already declared on line {existing.Pos.Line}. Pick another name, or leave out the type to change it: {id} = …;");
        var v = new Variable { Id = id, Type = type, Pos = pos, CsName = $"v_{id}_{++counter}" };
        if (existing == null) scopes[^1][id] = v;
        return v;
    }

    void InScope(Action body)
    {
        scopes.Add(new());
        try { body(); } finally { scopes.RemoveAt(scopes.Count - 1); }
    }

    // ----- statements -----

    void Stmt(Stmt s)
    {
        switch (s)
        {
            case VarDecl d:
            {
                var type = LType.FromName(d.TypeRef.Name) ?? LType.Error;
                var init = Expr(d.Init);
                ExpectFits(init, d.Init, type, $"{d.Id} is {type.WithArticle}");
                d.Var = Declare(d.Id, type, d.IdPos);
                break;
            }
            case Assign a:
            {
                var v = Lookup(a.Target.Id);
                var value = Expr(a.Value);
                if (v == null && broken.Contains(a.Target.Id)) break;
                if (v == null) { Error(a.Target.Pos, $"there's nothing called {a.Target.Id} yet. Declare it first, with its type: int {a.Target.Id} = …;"); break; }
                a.Target.Var = v;
                a.Target.Type = v.Type;
                ExpectFits(value, a.Value, v.Type, $"{v.Id} is {v.Type.WithArticle}");
                break;
            }
            case ExprStmt e:
            {
                var t = Expr(e.Expr);
                if (t != LType.None && t != LType.Error && e.Expr is Call c)
                    Error(e.Pos, $"{c.Callee}(…) gives back {t.WithArticle}, but nothing uses it. Put it in a variable: {t.Name} x = {c.Callee}(…);");
                break;
            }
            case Wait w:
            {
                var t = Expr(w.Seconds);
                if (!t.IsNumber && t != LType.Error) Error(w.Seconds.Pos, $"wait takes a number of seconds, but this is {t.WithArticle}.");
                if (w.Seconds is IntLit { Value: < 0 } or FloatLit { Value: < 0 }) Error(w.Seconds.Pos, "can't wait a negative number of seconds.");
                break;
            }
            case IsChain chain:
                foreach (var (check, body) in chain.Branches)
                {
                    ExpectTruth(check, "a question");
                    InScope(() => Block(body));
                }
                if (chain.Otherwise != null) InScope(() => Block(chain.Otherwise));
                break;
            case Loop loop:
            {
                if (loop.Kind == LoopKind.Times)
                {
                    var t = Expr(loop.Expr!);
                    if (t != LType.Int && t != LType.Error)
                        Error(loop.Expr!.Pos, t == LType.Float ? "loop needs a whole number of times. Use .int() to drop the part after the point." : $"loop needs a number of times, but this is {t.WithArticle}.");
                }
                if (loop.Kind == LoopKind.While) ExpectTruth(loop.Expr!, "loop ( … )");
                loopDepth++;
                InScope(() => Block(loop.Body));
                loopDepth--;
                break;
            }
            case Stop:
                if (loopDepth == 0) Error(s.Pos, "stop only works inside a loop.");
                break;
            case Skip:
                if (loopDepth == 0) Error(s.Pos, "skip only works inside a loop.");
                break;
            case Block b:
                InScope(() => Block(b));
                break;
        }
    }

    void Block(Block b)
    {
        foreach (var s in b.Stmts) Stmt(s);
    }

    void ExpectTruth(Expr e, string where)
    {
        var t = Expr(e);
        if (t == LType.Truth || t == LType.Error) return;
        var what = e is Name n ? n.Id : "this";
        var name = e is Name n2 ? n2.Id : (t.IsNumber ? "hp" : "name");
        var example = where switch
        {
            "loop ( … )" => t.IsNumber ? $"loop ({name} > 0)" : $"loop ({name} \"Ada\")",
            "either" => t.IsNumber ? $"either {name} 0? … or …" : $"either {name} \"Ada\"? … or …",
            _ => t.IsNumber ? $"is {name} 0?" : $"is {name} \"Ada\"?",
        };
        var hint = t.IsNumber || t == LType.String
            ? (e is Name ? $" Try {example}" : $" Compare it with something, like {example}")
            : "";
        Error(e.Pos, $"{where} needs a yes-or-no answer, but {what} is {t.WithArticle}.{hint}");
    }

    void ExpectFits(LType actual, Expr e, LType wanted, string subject)
    {
        if (LType.Fits(actual, wanted)) return;
        string fix = (actual, wanted) switch
        {
            _ when actual == LType.None => " It doesn't give anything back.",
            _ when actual == LType.Float && wanted == LType.Int => " Use .int() to drop the part after the point.",
            _ when wanted == LType.String && (actual.IsNumber || actual == LType.Truth) => " Use .text() to turn it into text.",
            _ => "",
        };
        Error(e.Pos, $"{subject}, but this is {actual.WithArticle}.{fix}");
    }

    // ----- expressions -----

    LType Expr(Expr e)
    {
        e.Type = ExprInner(e);
        return e.Type;
    }

    LType ExprInner(Expr e)
    {
        switch (e)
        {
            case IntLit: return LType.Int;
            case FloatLit: return LType.Float;
            case StrLit: return LType.String;
            case TruthLit: return LType.Truth;
            case Name n:
            {
                if (n.Id is "print" or "random" or "wait")
                {
                    Error(n.Pos, n.Id == "wait" ? "wait goes on its own line: wait 1;" : $"{n.Id} is something you call: {n.Id}(…)");
                    return LType.Error;
                }
                var v = Lookup(n.Id);
                if (v == null && broken.Contains(n.Id)) return LType.Error;
                if (v == null) { Error(n.Pos, $"there's nothing called {n.Id} here. Declare it first, like int {n.Id} = 0;"); return LType.Error; }
                n.Var = v;
                return v.Type;
            }
            case Unary u:
            {
                var t = Expr(u.Operand);
                if (t == LType.Error) return t;
                if (u.Op == UnOp.Neg)
                {
                    if (t.IsNumber) return t;
                    Error(u.Pos, $"- only works on numbers, not on {t.WithArticle}.");
                    return LType.Error;
                }
                if (t == LType.Truth) return t;
                Error(u.Operand.Pos, $"not flips a truth, but this is {t.WithArticle}.");
                return LType.Error;
            }
            case Binary b: return BinaryType(b);
            case Same s:
            {
                var l = Expr(s.Left);
                var r = Expr(s.Right);
                if (l == LType.Error || r == LType.Error) return LType.Truth;
                if (l == LType.None || r == LType.None) { Error(s.Pos, "one side of this question doesn't give a value."); return LType.Truth; }
                if (l == r || (l.IsNumber && r.IsNumber)) return LType.Truth;
                Error(s.Right.Pos, $"can't compare {l.WithArticle} with {r.WithArticle}.{(r == LType.String && l.IsNumber || l == LType.String && r.IsNumber ? " Use .text() on the number to compare them as text." : "")}");
                return LType.Truth;
            }
            case Either ei:
            {
                ExpectTruth(ei.Check, "either");
                var y = Expr(ei.Yes);
                var n = Expr(ei.No);
                if (y == LType.Error || n == LType.Error) return y == LType.Error ? n : y;
                if (y == n) return y;
                if (y.IsNumber && n.IsNumber) return LType.Float;
                Error(ei.No.Pos, $"both sides of either must be the same type, but one is {y.WithArticle} and the other is {n.WithArticle}.");
                return LType.Error;
            }
            case Call c: return CallType(c);
            case Conversion cv:
            {
                var t = Expr(cv.Target);
                if (t == LType.Error) return t;
                switch (cv.Method)
                {
                    case "text":
                        if (t == LType.String) { Error(cv.MethodPos, "this is already text."); return t; }
                        if (t == LType.None) break;
                        return LType.String;
                    case "int":
                        if (t == LType.Float) return LType.Int;
                        if (t == LType.Int) { Error(cv.MethodPos, "this is already an int."); return t; }
                        break;
                    case "float":
                        if (t == LType.Int) return LType.Float;
                        if (t == LType.Float) { Error(cv.MethodPos, "this is already a float."); return t; }
                        break;
                    default:
                        Error(cv.MethodPos, $"Lust doesn't have .{cv.Method}(). The built-in ones are .text(), .int() and .float().");
                        return LType.Error;
                }
                Error(cv.MethodPos, $".{cv.Method}() doesn't work on {t.WithArticle}.");
                return LType.Error;
            }
        }
        return LType.Error;
    }

    LType BinaryType(Binary b)
    {
        var l = Expr(b.Left);
        var r = Expr(b.Right);
        if (l == LType.Error || r == LType.Error) return b.Op is BinOp.Lt or BinOp.Gt or BinOp.Le or BinOp.Ge or BinOp.And or BinOp.Or ? LType.Truth : LType.Error;
        string sym = b.Op switch
        {
            BinOp.Add => "+", BinOp.Sub => "-", BinOp.Mul => "*", BinOp.Div => "/", BinOp.Mod => "%",
            BinOp.Lt => "<", BinOp.Gt => ">", BinOp.Le => "<=", BinOp.Ge => ">=", BinOp.And => "&&", _ => "||",
        };
        switch (b.Op)
        {
            case BinOp.Add when l == LType.String || r == LType.String:
            {
                if (l == r) return LType.String;
                var other = l == LType.String ? b.Right : b.Left;
                var otherType = l == LType.String ? r : l;
                string fix = otherType.IsNumber || otherType == LType.Truth
                    ? $" Use {(other is Name n ? n.Id : "(…)")}.text() to turn {(other is Name n2 ? n2.Id : "it")} into text."
                    : "";
                Error(b.OpPos, $"can't add {otherType.WithArticle} to a string.{fix}");
                return LType.Error;
            }
            case BinOp.Add or BinOp.Sub or BinOp.Mul or BinOp.Div or BinOp.Mod:
                if (l.IsNumber && r.IsNumber) return l == LType.Float || r == LType.Float ? LType.Float : LType.Int;
                Error(b.OpPos, $"{sym} works on numbers, but this has {(l.IsNumber ? r : l).WithArticle}.");
                return LType.Error;
            case BinOp.Lt or BinOp.Gt or BinOp.Le or BinOp.Ge:
                if (!(l.IsNumber && r.IsNumber)) Error(b.OpPos, $"{sym} compares numbers, but this has {(l.IsNumber ? r : l).WithArticle}.");
                return LType.Truth;
            default: // && ||
                if (l != LType.Truth) Error(b.Left.Pos, $"{sym} joins yes-or-no checks, but this is {l.WithArticle}.");
                else if (r != LType.Truth) Error(b.Right.Pos, $"{sym} joins yes-or-no checks, but this is {r.WithArticle}.");
                return LType.Truth;
        }
    }

    LType CallType(Call c)
    {
        var types = c.Args.Select(Expr).ToList();
        switch (c.Callee)
        {
            case "print":
                if (c.Args.Count != 1) { Error(c.Pos, "print takes one thing to show: print(x)"); return LType.None; }
                if (types[0] == LType.None) Error(c.Args[0].Pos, "this doesn't give anything to print.");
                return LType.None;
            case "random":
                if (c.Args.Count != 2) { Error(c.Pos, "random takes two whole numbers, lowest and highest: random(1, 6)"); return LType.Int; }
                for (int i = 0; i < 2; i++)
                    if (types[i] != LType.Int && types[i] != LType.Error)
                        Error(c.Args[i].Pos, $"random takes whole numbers, but this is {types[i].WithArticle}.");
                if (c.Args[0] is IntLit lo && c.Args[1] is IntLit hi && lo.Value > hi.Value)
                    Error(c.Args[0].Pos, "random's lowest number comes first: random(1, 6)");
                return LType.Int;
            case "wait":
                Error(c.Pos, "wait goes on its own line, without (): wait 1;");
                return LType.None;
        }
        var v = Lookup(c.Callee);
        Error(c.Pos, v != null
            ? $"{c.Callee} is {v.Type.WithArticle}, not something you can call."
            : $"there's no function called {c.Callee}. The built-ins are print, random and wait; your own functions (do) come in a later milestone.");
        return LType.Error;
    }
}
