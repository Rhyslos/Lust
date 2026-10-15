using System.Globalization;
using System.Text;

namespace Lust;

/// Writes a checked Lust program as C#. Every statement is tagged with its
/// Lust line (#line), so a crash can be reported at the right place in the .lust file.
///
/// What becomes what:
///   state            → an enum            S_file_name
///   cArray row       → a class            R_n
///   class            → a class            C_file_Name   (fields f_x, methods m_x)
///   object           → a class with one   O_file_Name.Instance
///   do function      → a static method    M_file.F_name
///   top-level code   → LustProgram.Run()  (only for the file you run)
public sealed class Emitter
{
    readonly StringBuilder sb = new();
    readonly Compilation comp;
    SourceFile file = null!;
    ClassSym? cls;
    int indent;
    int counter;

    Emitter(Compilation comp) { this.comp = comp; }

    public static string Emit(Compilation comp)
    {
        var e = new Emitter(comp);
        e.WriteAll();
        return e.sb.ToString();
    }

    void WriteAll()
    {
        Line($"// Written by Lust from {comp.Main.Source.Display}. It's rewritten on every run, so don't edit it.");
        Line("#nullable disable");
        Line("#pragma warning disable");
        Line("using System;");
        Line("using System.Collections.Generic;");
        Line("");

        foreach (var f in comp.Files.Where(f => !f.IsNative))
            foreach (var s in f.OwnStates)
                Line($"public enum {s.CsName} {{ {string.Join(", ", s.Options.Select((o, i) => $"[LustName({CsString(o)})] o{i}"))} }}");

        foreach (var t in comp.CArrays.OrderBy(t => t.Number)) Row(t);

        // Classes from ui.lust are written by hand in the runtime (LustUi.cs).
        foreach (var f in comp.Files.Where(f => !f.IsNative))
        {
            file = f.Source;
            foreach (var c in f.OwnClasses) Class(c);
        }

        foreach (var f in comp.Files.Where(f => !f.IsNative))
        {
            if (f.OwnFuncs.Count == 0) continue;
            file = f.Source;
            cls = null;
            Line($"public static class M_{f.CsModule}");
            Line("{");
            indent++;
            foreach (var fn in f.OwnFuncs) Function(fn, $"public static {(fn.Return == LType.None ? "void" : fn.Return.Cs)} F_{fn.Name}");
            indent--;
            Line("}");
            Line("");
        }

        var saves = comp.Files.Where(f => !f.IsNative).SelectMany(f => f.OwnSaves).ToList();
        if (saves.Count > 0)
        {
            Line("static class LustSaves");
            Line("{");
            indent++;
            foreach (var save in saves)
            {
                var items = save.Items.Select(it => it.Object != null
                    ? $"new LustSave.Obj({CsString(it.Name)}, () => {it.Object.CsName}.Instance)"
                    : $"new LustSave.Var({CsString(it.Name)}, typeof({it.Var!.Type.Cs}), () => (object)LustProgram.{it.Var.CsName}, v => LustProgram.{it.Var.CsName} = ({it.Var.Type.Cs})v)");
                Line($"internal static readonly LustSave s_{save.File.CsModule}_{save.Name} = new LustSave({CsString(save.Name)}{string.Concat(items.Select(i => ", " + i))});");
            }
            indent--;
            Line("}");
            Line("");
        }

        file = comp.Main.Source;
        cls = null;
        Line("static class LustProgram");
        Line("{");
        indent++;
        // Variables kept in a save live here, so the save can reach them from anywhere.
        foreach (var v in comp.Main.TopScope.Values.Where(v => v.Hoisted))
            Line($"internal static {v.Type.Cs} {v.CsName};");
        Line("static void Main()");
        Line("{");
        indent++;
        Line("Console.OutputEncoding = System.Text.Encoding.UTF8;");
        foreach (var f in comp.Files.Where(f => !f.IsNative))
            Line($"LustRt.Name({CsString(LinePath(f.Source))}, {CsString(f.Source.Display)});");
        if (saves.Count > 0)
        {
            Line($"LustSave.Program = {CsString(ProgramName())};");
            // Class names in save files: Dog, or farm.Dog when two files both have a Dog.
            var all = comp.Files.Where(f => !f.IsNative).SelectMany(f => f.OwnClasses).ToList();
            foreach (var c in all)
            {
                bool clash = all.Count(o => o.Name == c.Name) > 1;
                Line($"LustSave.Name(typeof({c.CsName}), {CsString(clash ? $"{c.File.Module}.{c.Name}" : c.Name)});");
            }
        }
        Line("LustRt.Start(Run);");
        indent--;
        Line("}");
        Line("");
        Line("static void Run()");
        Line("{");
        indent++;
        foreach (var s in comp.Main.Ast.Stmts) Stmt(s);
        Raw("#line default");
        indent--;
        Line("}");
        indent--;
        Line("}");
    }

    /// The program's name, for its folder of saves: the file's name, or its folder's when the file is main.lust.
    string ProgramName()
    {
        var path = comp.Main.Source.Path;
        var name = comp.Main.Module;
        if (name.Equals("main", StringComparison.OrdinalIgnoreCase))
        {
            var folder = System.IO.Path.GetFileName(System.IO.Path.GetDirectoryName(path));
            if (!string.IsNullOrEmpty(folder)) name = folder;
        }
        return new string(name.Where(ch => !System.IO.Path.GetInvalidFileNameChars().Contains(ch)).ToArray());
    }

    void Line(string text) => sb.Append(' ', indent * 4).Append(text).Append('\n');
    void Raw(string text) => sb.Append(text).Append('\n');

    // #line takes the file name exactly as written, without \ escapes, so it isn't a normal C# string.
    static string LinePath(SourceFile f) => f.Path.Replace("\"", "");
    void At(Pos pos) => Raw($"#line {pos.Line} \"{LinePath(pos.File)}\"");

    // ----- declarations -----

    void Row(CArrayType t)
    {
        var name = t.Row.Cs;
        Line($"public sealed class {name} : LustRow");
        Line("{");
        indent++;
        foreach (var (col, type) in t.Columns) Line($"public {type.Cs} c_{col};");
        // Positional parameter names, so no column name can clash with C# or with each other.
        var prms = string.Join("", t.Columns.Select((c, i) => $", {c.Type.Cs} a{i}"));
        Line($"public {name}(bool hasId_, long id_{prms}) : base(hasId_, id_) {{ {string.Join(" ", t.Columns.Select((c, i) => $"c_{c.Name} = a{i};"))} }}");
        Line($"public override string ToString() => \"[\" + id{string.Join("", t.Columns.Select(c => $" + \", \" + LustRt.Show(c_{c.Name})"))} + \"]\";");
        indent--;
        Line("}");
        Line("");
    }

    void Class(ClassSym c)
    {
        cls = c;
        var parent = c.Parent?.CsName ?? "LustObject";
        Line($"public {(c.IsObject ? "sealed " : "")}class {c.CsName} : {parent}");
        Line("{");
        indent++;
        if (c.IsObject) Line($"public static readonly {c.CsName} Instance = new {c.CsName}();");
        foreach (var f in c.Fields.Where(f => f.Owner == c)) Line($"public {f.Type.Cs} {f.CsName};");
        Line($"public {c.CsName}()");
        Line("{");
        indent++;
        foreach (var (field, value) in c.Defaults)
        {
            At(value.Pos);
            Line($"{field.CsName} = {Coerce(value, field.Type)};");
        }
        Raw("#line default");
        indent--;
        Line("}");
        Line($"public override string LustName => {CsString(c.Name)};");
        // print shows a class like its has block: Player { name "Ada", hp 20 }
        var shown = c.Fields.Select(f => $"{CsString(f.Name + " ")} + LustRt.Show({f.CsName})").ToList();
        var show = shown.Count == 0
            ? CsString(c.Name + " { }")
            : $"{CsString(c.Name + " { ")} + {string.Join(" + \", \" + ", shown)} + \" }}\"";
        Line($"public override string LustShow() => {show};");
        foreach (var m in c.OwnMethods)
        {
            var modifier = m.IsOverride ? "public override" : c.IsObject ? "public" : "public virtual";
            Function(m, $"{modifier} {(m.Return == LType.None ? "void" : m.Return.Cs)} m_{m.Name}");
        }
        indent--;
        Line("}");
        Line("");
        cls = null;
    }

    void Function(FuncSym fn, string head)
    {
        var d = fn.Decl;
        var prms = string.Join(", ", fn.ParamVars.Select(v => $"{v.Type.Cs} {v.CsName}"));
        Line($"{head}({prms})");
        currentFunc = fn;
        Line("{");
        indent++;
        Line("LustRt.Enter();");
        Line("try");
        Block(d.Body);
        Line("finally { LustRt.Leave(); }");
        indent--;
        Line("}");
        currentFunc = null;
        Raw("#line default");
    }

    // ----- statements -----

    void Stmt(Stmt s)
    {
        switch (s)
        {
            case VarDecl d:
                At(d.Pos);
                if (d.Var!.Hoisted) Line($"LustProgram.{d.Var.CsName} = {Coerce(d.Init, d.Var.Type)};");
                else Line($"{d.Var.Type.Cs} {d.Var.CsName} = {Coerce(d.Init, d.Var.Type)};");
                break;
            case SaveStmt sv:
                At(sv.Pos);
                Line($"{sv.Sym!.CsName}.{(sv.IsLoad ? "Load" : "Save")}();");
                break;
            case Assign a:
                At(a.Pos);
                if (a.Target is FieldGet { Kind: FieldKind.Cs } csf) Line($"{CsMemberRaw(csf)} = {CsLibs.ToCs(Coerce(a.Value, a.Target.Type), a.Value.Type == LType.Int && a.Target.Type == LType.Float ? LType.Float : a.Value.Type, csf.Cs!.ValueType)};");
                else if (a.Target is Index ix) Line($"{Expr(ix.Target)}.Set({Expr(ix.Key)}, {Coerce(a.Value, a.Target.Type)});");
                else Line($"{Expr(a.Target)} = {Coerce(a.Value, a.Target.Type)};");
                break;
            case StateSet ss:
                At(ss.Pos);
                if (ss.Target is Index six) Line($"{Expr(six.Target)}.Set({Expr(six.Key)}, {Option(ss.State!, ss.Option)});");
                else Line($"{Expr(ss.Target)} = {Option(ss.State!, ss.Option)};");
                break;
            case ExprStmt e:
                At(e.Pos);
                // A C# call on its own line: what it gives back is dropped, so no conversion around it.
                Line(e.Expr is MethodCall { Kind: MethodKind.Cs } csm ? $"{CsCallRaw(csm)};" : $"{Expr(e.Expr)};");
                break;
            case Wait w:
                At(w.Pos);
                Line($"LustRt.Wait({Expr(w.Seconds)});");
                break;
            case Give g:
                At(g.Pos);
                Line(g.Value == null ? "return;" : $"return {Coerce(g.Value, ReturnType(g))};");
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
                LoopStmt(loop);
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

    // The function a give belongs to, found while writing it.
    FuncSym? currentFunc;
    LType ReturnType(Give g) => currentFunc?.Return ?? g.Value!.Type;

    void Block(Block b)
    {
        Line("{");
        indent++;
        foreach (var s in b.Stmts) Stmt(s);
        indent--;
        Line("}");
    }

    void LoopStmt(Loop loop)
    {
        At(loop.Pos);
        switch (loop.Kind)
        {
            case LoopKind.Forever:
                Line("while (true)");
                Block(loop.Body);
                return;
            case LoopKind.While:
                Line($"while ({Expr(loop.Expr!)})");
                Block(loop.Body);
                return;
            case LoopKind.Times:
            {
                int n = ++counter;
                Line($"for (long i_{n} = 0, n_{n} = {Expr(loop.Expr!)}; i_{n} < n_{n}; i_{n}++)");
                Block(loop.Body);
                return;
            }
        }
        var vars = loop.LoopVars;
        switch (loop.Each)
        {
            case EachKind.Array:
                Line($"foreach ({vars[0].Type.Cs} {vars[0].CsName} in {Expr(loop.Expr!)}.Snapshot())");
                Block(loop.Body);
                break;
            case EachKind.State:
                Line($"foreach ({vars[0].Type.Cs} {vars[0].CsName} in Enum.GetValues<{loop.EachState!.CsName}>())");
                Block(loop.Body);
                break;
            case EachKind.Map:
            {
                int n = ++counter;
                Line($"foreach (var kv_{n} in {Expr(loop.Expr!)}.Snapshot())");
                Line("{");
                indent++;
                Line($"string {vars[0].CsName} = kv_{n}.Key;");
                Line($"{vars[1].Type.Cs} {vars[1].CsName} = {(vars[1].Type == LType.Float && ((MapType)loop.Expr!.Type).Elem == LType.Int ? "(double)" : "")}kv_{n}.Value;");
                foreach (var s in loop.Body.Stmts) Stmt(s);
                indent--;
                Line("}");
                break;
            }
            case EachKind.CArray:
            {
                var table = (CArrayType)loop.Expr!.Type;
                int n = ++counter;
                Line($"foreach (var row_{n} in {Expr(loop.Expr!)}.Snapshot())");
                Line("{");
                indent++;
                Line($"long {vars[0].CsName} = row_{n}.id;");
                for (int i = 0; i < table.Columns.Count && i + 1 < vars.Count; i++)
                    Line($"{vars[i + 1].Type.Cs} {vars[i + 1].CsName} = row_{n}.c_{table.Columns[i].Name};");
                foreach (var s in loop.Body.Stmts) Stmt(s);
                indent--;
                Line("}");
                break;
            }
        }
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
            case NothingLit: return "null";
            case Name n:
                return n.Kind switch
                {
                    NameKind.Local => n.Var!.Hoisted ? "LustProgram." + n.Var.CsName : n.Var.CsName,
                    NameKind.Field => n.Field!.CsName,
                    NameKind.Object => $"{n.Object!.CsName}.Instance",
                    NameKind.Option => Option(n.State!, n.Id),
                    NameKind.CsType => n.Type.Cs,
                    NameKind.Function => $"new {n.Type.Cs}({n.Func!.CsName})",
                    NameKind.Method => $"new {n.Type.Cs}(m_{n.Func!.Name})",
                    _ => throw new InvalidOperationException("unresolved name " + n.Id),
                };
            case Unary u: return u.Op == UnOp.Neg ? $"(-{Expr(u.Operand)})" : $"(!{Expr(u.Operand)})";
            case Binary { Op: BinOp.Div or BinOp.Mod } bi when bi.Left.Type == LType.Int && bi.Right.Type == LType.Int:
                // Through the runtime, so 1 / 0 is a "divided by zero" while running, not a C# build error.
                return $"LustRt.{(bi.Op == BinOp.Div ? "Div" : "Mod")}({Expr(bi.Left)}, {Expr(bi.Right)})";
            case Binary { Op: BinOp.Add or BinOp.Sub or BinOp.Mul } bc when bc.Type == LType.Int && IsConstant(bc):
                // Numbers only: C# would work it out while building and refuse an overflow, so hide them.
                return $"({Expr(bc.Left)} {(bc.Op == BinOp.Add ? "+" : bc.Op == BinOp.Sub ? "-" : "*")} LustRt.Same({Expr(bc.Right)}))";
            case Binary b:
            {
                string op = b.Op switch
                {
                    BinOp.Add => "+", BinOp.Sub => "-", BinOp.Mul => "*", BinOp.Div => "/", BinOp.Mod => "%",
                    BinOp.Lt => "<", BinOp.Gt => ">", BinOp.Le => "<=", BinOp.Ge => ">=", BinOp.And => "&&", _ => "||",
                };
                return $"({Expr(b.Left)} {op} {Expr(b.Right)})";
            }
            case Same s:
            {
                // Compare as objects when one side is nothing, so C# doesn't need a common type.
                string l = Expr(s.Left), r = Expr(s.Right);
                // Two functions are the same when they're the same function (or the same method of the same thing).
                // C# values compare the C# way (two equal dates are the same).
                if (s.Left.Type is FuncType or CsType || s.Right.Type is FuncType or CsType)
                    return $"({(s.Negated ? "!" : "")}Equals({l}, {r}))";
                if (s.Left.Type is ClassType || s.Right.Type is ClassType || s.Left.Type == LType.Nothing)
                    return $"({(s.Negated ? "!" : "")}ReferenceEquals({l}, {r}))";
                // is state fighting? is also yes for fighting.blocking.
                if (s.Left.Type is StateType)
                    return $"({(s.Negated ? "!" : "")}LustRt.Under({l}, {r}))";
                return $"({l} {(s.Negated ? "!=" : "==")} {r})";
            }
            case IsLike il: return $"({(il.Negated ? "!" : "")}({Expr(il.Value)} is {il.Sym!.CsName}))";
            case Either ei:
            {
                var type = ei.Type;
                return $"({Expr(ei.Check)} ? {CastTo(ei.Yes, type)} : {CastTo(ei.No, type)})";
            }
            case Call { Value: not null } vc:
                return $"LustRt.Fn({Expr(vc.Value)}, {CsString(vc.Callee)})({FnArguments(vc.Args, (FuncType)vc.Value.Type)})";
            case CallValue cvl:
                return $"LustRt.Fn({Expr(cvl.Target)})({FnArguments(cvl.Args, (FuncType)cvl.Target.Type)})";
            case MethodRef mr:
                return $"new {mr.Type.Cs}(LustRt.Use({Expr(mr.Target)}).m_{mr.Method})";
            case Call c:
                return c.Callee switch
                {
                    "print" => $"LustRt.Print({Expr(c.Args[0])})",
                    "random" => $"LustRt.Random({Expr(c.Args[0])}, {Expr(c.Args[1])})",
                    "readText" => $"LustRt.ReadText({Expr(c.Args[0])})",
                    "writeText" => $"LustRt.WriteText({Expr(c.Args[0])}, {Expr(c.Args[1])})",
                    "ask" => $"LustRt.Ask({(c.Args.Count == 1 ? Expr(c.Args[0]) : "\"\"")})",
                    _ => $"{(c.Func!.Owner != null ? "m_" + c.Func.Name : c.Func.CsName)}({Arguments(c.Args, c.Func)})",
                };
            case MethodCall mc:
            {
                var target = Expr(mc.Target);
                return mc.Kind switch
                {
                    MethodKind.User => $"{target}.m_{mc.Method}({Arguments(mc.Args, mc.Func!)})",
                    MethodKind.Cs => CsLibs.FromCsCode(CsCallRaw(mc), ((CsCall)mc.Cs!).Returns),
                    MethodKind.CallField => $"LustRt.Fn({target}.{mc.Field!.CsName}, {CsString(mc.Method)})({FnArguments(mc.Args, (FuncType)mc.Field.Type)})",
                    MethodKind.ArrayInject => $"{target}.Inject({Coerce(mc.Args[0], ((ArrayType)mc.Target.Type).Elem)})",
                    MethodKind.ArrayRemove => $"{target}.Remove({Expr(mc.Args[0])})",
                    MethodKind.CArrayInject => $"{target}.Inject({Expr(mc.Args[0])})",
                    MethodKind.CArrayRemove => $"{target}.Remove({Expr(mc.Args[0])})",
                    MethodKind.CArrayHas => $"{target}.Has({Expr(mc.Args[0])})",
                    MethodKind.MapHas => $"{target}.Has({Expr(mc.Args[0])})",
                    MethodKind.MapRemove => $"{target}.Remove({Expr(mc.Args[0])})",
                    _ => throw new InvalidOperationException("unresolved method " + mc.Method),
                };
            }
            case Conversion cv:
            {
                var target = Expr(cv.Target);
                bool fromText = cv.Target.Type == LType.String;
                return cv.Method switch
                {
                    "text" => cv.Target.Type is CsType ? $"LustRt.Text((object){target})" : $"LustRt.Text({target})",
                    "int" => fromText ? $"LustRt.ParseInt({target})" : $"LustRt.Int({target})",
                    "float" => fromText ? $"LustRt.ParseFloat({target})" : $"((double){target})",
                    "lines" => $"LustRt.Lines({target})",
                    _ => $"LustRt.IsNumber({target})",
                };
            }
            case FieldGet fg:
                return fg.Kind switch
                {
                    FieldKind.Field => $"{Expr(fg.Target)}.{fg.Sym!.CsName}",
                    FieldKind.Column => fg.Field == "id" ? $"{Expr(fg.Target)}.id" : $"{Expr(fg.Target)}.c_{fg.Field}",
                    FieldKind.Option => Option(fg.OptionState!, fg.OptionPath),
                    FieldKind.Cs => CsLibs.FromCsCode(CsMemberRaw(fg), fg.Cs!.ValueType),
                    _ => throw new InvalidOperationException("unresolved field " + fg.Field),
                };
            case Index ix: return $"{Expr(ix.Target)}.Get({Expr(ix.Key)})";
            case Length len:
                return len.Target.Type == LType.String ? $"LustRt.Letters({Expr(len.Target)})" : $"{Expr(len.Target)}.Count";
            case New { CsClr: not null } cn:
            {
                var args = cn.CsCtor == null ? "" : CsArgs(cn.Args ?? new(), cn.CsCtor);
                var inits = cn.Inits.Select((init, i) => $"{cn.CsInits[i].Member.Name} = {CsLibs.ToCs(Coerce(init.Value, init.Value.Type), init.Value.Type, cn.CsInits[i].ValueType)}");
                return $"new {CsNames.Of(cn.CsClr)}({args}){(cn.Inits.Count > 0 ? " { " + string.Join(", ", inits) + " }" : "")}";
            }
            case New nw:
            {
                if (nw.Inits.Count == 0) return $"new {nw.Sym!.CsName}()";
                var inits = nw.Inits.Select((init, i) => $"{nw.InitFields[i].CsName} = {Coerce(init.Value, nw.InitFields[i].Type)}");
                return $"new {nw.Sym!.CsName} {{ {string.Join(", ", inits)} }}";
            }
            case SavedExpr se: return $"{se.Sym!.CsName}.Exists()";
            case ArrayLit al:
                return al.Type switch
                {
                    MapType mt => al.Items.Count == 0 ? $"new {mt.Cs}()" : $"{mt.Cs}.Of({string.Join(", ", al.Items.Select((x, i) => $"({Expr(al.Keys![i])}, {Coerce(x, mt.Elem)})"))})",
                    ArrayType at => al.Items.Count == 0 ? $"new {at.Cs}()" : $"{at.Cs}.Of({string.Join(", ", al.Items.Select(x => Coerce(x, at.Elem)))})",
                    CArrayType ct => $"new {ct.Cs}({string.Join(", ", al.Items.Select(Expr))})",
                    RowType rt => RowLit(al, rt.Table),
                    _ => throw new InvalidOperationException("untyped array"),
                };
        }
        throw new InvalidOperationException("can't write " + e.GetType().Name);
    }

    static string Option(StateSym st, string path) => $"{st.CsName}.o{st.Options.IndexOf(path)}";

    static bool IsConstant(Expr e) => e switch
    {
        IntLit or FloatLit => true,
        Unary u => IsConstant(u.Operand),
        Binary b => IsConstant(b.Left) && IsConstant(b.Right),
        _ => false,
    };

    string RowLit(ArrayLit al, CArrayType table)
    {
        int offset = al.RowHasId ? 1 : 0;
        var id = al.RowHasId ? Expr(al.Items[0]) : "0L";
        var cols = table.Columns.Select((c, i) => Coerce(al.Items[i + offset], c.Type));
        return $"new {table.Row.Cs}({(al.RowHasId ? "true" : "false")}, {id}{string.Join("", cols.Select(c => ", " + c))})";
    }

    /// Both sides of either need the same C# type.
    string CastTo(Expr e, LType type)
    {
        if (type == LType.Float && e.Type == LType.Int) return $"((double){Expr(e)})";
        if (type is ClassType && (e.Type != type)) return $"(({type.Cs}){Expr(e)})";
        return Expr(e);
    }

    /// A C# call without turning what it gives back into a Lust value.
    string CsCallRaw(MethodCall mc)
    {
        var call = (CsCall)mc.Cs!;
        var args = CsArgs(mc.Args, call);
        var target = call.IsStatic ? CsNames.Of(call.Method.DeclaringType!) : $"({Expr(mc.Target)})";
        return $"{target}.{call.Method.Name}({args})";
    }

    string CsMemberRaw(FieldGet fg) =>
        fg.Cs!.IsStatic ? $"{CsNames.Of(((CsStaticType)fg.Target.Type).Clr)}.{fg.Cs.Member.Name}" : $"({Expr(fg.Target)}).{fg.Cs.Member.Name}";

    string CsArgs(List<Expr> args, CsCall call) =>
        string.Join(", ", args.Select((a, i) => CsLibs.ToCs(Expr(a), a.Type, call.Params[i].ParameterType)));

    string FnArguments(List<Expr> args, FuncType ft) =>
        string.Join(", ", args.Select((a, i) => Coerce(a, ft.Params[i])));

    string Arguments(List<Expr> args, FuncSym fn) =>
        string.Join(", ", args.Select((a, i) => Coerce(a, fn.Params[i].Type)));

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
                case '{': sb.Append('{'); break;
                default:
                    if (char.IsControl(c)) sb.Append($"\\u{(int)c:x4}");
                    else sb.Append(c);
                    break;
            }
        }
        return sb.Append('"').ToString();
    }
}
