using System.Globalization;
using System.Text;

namespace Lust;

/// Writes a checked Lust program as C#. Every statement is tagged with its
/// Lust line (#line), so a crash can be reported at the right place in the .lust file.
public sealed class Emitter
{
    readonly StringBuilder sb = new();
    readonly SourceFile file;
    int indent;
    int counter;

    Emitter(SourceFile file) { this.file = file; }

    public static string Emit(Program program)
    {
        var e = new Emitter(program.File);
        e.Line($"// Written by Lust from {program.File.Display}. It's rewritten on every run, so don't edit it.");
        e.Line("#nullable disable");
        e.Line("#pragma warning disable");
        e.Line("using System;");
        e.Line("");
        e.Line("static class LustProgram");
        e.Line("{");
        e.indent++;
        e.Line("static void Main()");
        e.Line("{");
        e.indent++;
        e.Line("Console.OutputEncoding = System.Text.Encoding.UTF8;");
        e.Line($"LustRt.Name({CsString(e.LinePath)}, {CsString(program.File.Display)});");
        e.Line("try { Run(); }");
        e.Line("catch (Exception e) { LustRt.Fail(e); }");
        e.indent--;
        e.Line("}");
        e.Line("");
        e.Line("static void Run()");
        e.Line("{");
        e.indent++;
        foreach (var s in program.Stmts) e.Stmt(s);
        e.Raw("#line default");
        e.indent--;
        e.Line("}");
        e.indent--;
        e.Line("}");
        return e.sb.ToString();
    }

    void Line(string text) => sb.Append(' ', indent * 4).Append(text).Append('\n');
    void Raw(string text) => sb.Append(text).Append('\n');
    // #line takes the file name exactly as written, without \ escapes, so it isn't a normal C# string.
    string LinePath => file.Path.Replace("\"", "");
    void At(Pos pos) => Raw($"#line {pos.Line} \"{LinePath}\"");

    // ----- statements -----

    void Stmt(Stmt s)
    {
        switch (s)
        {
            case VarDecl d:
                At(d.Pos);
                Line($"{d.Var!.Type.Cs} {d.Var.CsName} = {Coerce(d.Init, d.Var.Type)};");
                break;
            case Assign a:
                At(a.Pos);
                Line($"{a.Target.Var!.CsName} = {Coerce(a.Value, a.Target.Var.Type)};");
                break;
            case ExprStmt e:
                At(e.Pos);
                Line($"{Expr(e.Expr)};");
                break;
            case Wait w:
                At(w.Pos);
                Line($"LustRt.Wait({Expr(w.Seconds)});");
                break;
            case IsChain chain:
                for (int i = 0; i < chain.Branches.Count; i++)
                {
                    var (check, body) = chain.Branches[i];
                    At(check.Pos);
                    Line($"{(i == 0 ? "if" : "else if")} ({Expr(check)})");
                    Block(body);
                }
                if (chain.Otherwise != null)
                {
                    Line("else");
                    Block(chain.Otherwise);
                }
                break;
            case Loop loop:
                At(loop.Pos);
                switch (loop.Kind)
                {
                    case LoopKind.Forever:
                        Line("while (true)");
                        break;
                    case LoopKind.While:
                        Line($"while ({Expr(loop.Expr!)})");
                        break;
                    case LoopKind.Times:
                        int n = ++counter;
                        Line($"for (long i_{n} = 0, n_{n} = {Expr(loop.Expr!)}; i_{n} < n_{n}; i_{n}++)");
                        break;
                }
                Block(loop.Body);
                break;
            case Stop:
                At(s.Pos);
                Line("break;");
                break;
            case Skip:
                At(s.Pos);
                Line("continue;");
                break;
            case Block b:
                Block(b);
                break;
        }
    }

    void Block(Block b)
    {
        Line("{");
        indent++;
        foreach (var s in b.Stmts) Stmt(s);
        indent--;
        Line("}");
    }

    // ----- expressions -----

    /// An int going where a float is wanted gets turned into a float.
    string Coerce(Expr e, LType wanted) =>
        wanted == LType.Float && e.Type == LType.Int ? $"((double){Expr(e)})" : Expr(e);

    string Expr(Expr e)
    {
        switch (e)
        {
            case IntLit i:
                return i.Value < 0 ? $"({i.Value.ToString(CultureInfo.InvariantCulture)}L)" : i.Value.ToString(CultureInfo.InvariantCulture) + "L";
            case FloatLit f:
            {
                var text = f.Value.ToString("R", CultureInfo.InvariantCulture) + "d";
                return f.Value < 0 ? $"({text})" : text;
            }
            case StrLit s: return CsString(s.Value);
            case TruthLit: return "true";
            case Name n: return n.Var!.CsName;
            case Unary u: return u.Op == UnOp.Neg ? $"(-{Expr(u.Operand)})" : $"(!{Expr(u.Operand)})";
            case Binary b:
            {
                string op = b.Op switch
                {
                    BinOp.Add => "+", BinOp.Sub => "-", BinOp.Mul => "*", BinOp.Div => "/", BinOp.Mod => "%",
                    BinOp.Lt => "<", BinOp.Gt => ">", BinOp.Le => "<=", BinOp.Ge => ">=", BinOp.And => "&&", _ => "||",
                };
                return $"({Expr(b.Left)} {op} {Expr(b.Right)})";
            }
            case Same s: return $"({Expr(s.Left)} {(s.Negated ? "!=" : "==")} {Expr(s.Right)})";
            case Either ei: return $"({Expr(ei.Check)} ? {Coerce(ei.Yes, ei.Type)} : {Coerce(ei.No, ei.Type)})";
            case Call c:
                return c.Callee switch
                {
                    "print" => $"LustRt.Print({Expr(c.Args[0])})",
                    "random" => $"LustRt.Random({Expr(c.Args[0])}, {Expr(c.Args[1])})",
                    _ => throw new InvalidOperationException("unknown call " + c.Callee),
                };
            case Conversion cv:
                return cv.Method switch
                {
                    "text" => $"LustRt.Text({Expr(cv.Target)})",
                    "int" => $"LustRt.Int({Expr(cv.Target)})",
                    _ => $"((double){Expr(cv.Target)})",
                };
        }
        throw new InvalidOperationException("can't write " + e.GetType().Name);
    }

    public static string CsString(string s)
    {
        var sb = new StringBuilder("\"");
        foreach (char c in s)
        {
            switch (c)
            {
                case '"': sb.Append("\\\""); break;
                case '\\': sb.Append("\\\\"); break;
                case '\n': sb.Append("\\n"); break;
                case '\r': sb.Append("\\r"); break;
                case '\t': sb.Append("\\t"); break;
                default:
                    if (char.IsControl(c)) sb.Append($"\\u{(int)c:x4}");
                    else sb.Append(c);
                    break;
            }
        }
        return sb.Append('"').ToString();
    }
}
