namespace Lust;

/// Enforces Lust's rules for a whole program, in phases:
///   1. collect what each file declares
///   2. connect files (send, fetch, get)
///   3. build classes and functions (fields, like, method signatures)
///   4. check every body, and record types and meanings for the C# writer
public sealed class Checker
{
    readonly Compilation comp;
    Diagnostics diags => comp.Diags;
    int counter;

    Checker(Compilation comp) { this.comp = comp; }

    public static void Check(Compilation comp)
    {
        var c = new Checker(comp);
        foreach (var f in comp.Files) c.Collect(f);
        foreach (var f in comp.Files) c.CheckSends(f);
        foreach (var f in comp.Files) c.Connect(f);
        foreach (var f in comp.Files)
        {
            foreach (var cls in f.OwnClasses) c.Build(cls);
            foreach (var fn in f.OwnFuncs) c.Signature(fn, null);
        }
        foreach (var f in comp.Files) c.CheckBodies(f);
    }

    void Error(Pos pos, string message) => diags.Add(pos, message);

    // ===== 1. collect =====

    void Collect(FileUnit f)
    {
        var ast = f.Ast;
        foreach (var s in ast.States)
        {
            if (f.States.TryGetValue(s.Name, out var old))
            {
                Error(s.Pos, s.Name == "state"
                    ? $"this file already has a main state, on line {old.Decl.Pos.Line}. Give this one a name: state season = [...];"
                    : $"there's already a state called {s.Name}, on line {old.Decl.Pos.Line}.");
                continue;
            }
            var sym = new StateSym { Name = s.Name, File = f, Decl = s, Display = s.Name };
            foreach (var (opt, pos) in s.Options)
            {
                if (sym.Options.Contains(opt)) Error(pos, $"{opt} is already an option of {s.Name}.");
                else sym.Options.Add(opt);
            }
            f.States[s.Name] = sym;
            f.OwnStates.Add(sym);
        }
        foreach (var cd in ast.Classes)
        {
            if (f.Classes.TryGetValue(cd.Name, out var old)) { Error(cd.Pos, $"there's already a {old.Kind} called {cd.Name}, on line {old.Decl.Pos.Line}."); continue; }
            if (f.States.ContainsKey(cd.Name)) { Error(cd.Pos, $"there's already a state called {cd.Name}."); continue; }
            var sym = new ClassSym { Name = cd.Name, File = f, Decl = cd };
            cd.Sym = sym;
            f.Classes[cd.Name] = sym;
            f.OwnClasses.Add(sym);
        }
        foreach (var fd in ast.Funcs)
        {
            if (f.Funcs.TryGetValue(fd.Name, out var old)) { Error(fd.NamePos, $"there's already a function called {fd.Name}, on line {old.Decl.Pos.Line}."); continue; }
            if (f.Classes.TryGetValue(fd.Name, out var cls)) { Error(fd.NamePos, $"there's already a {cls.Kind} called {fd.Name}."); continue; }
            var sym = new FuncSym { Name = fd.Name, File = f, Decl = fd };
            fd.Sym = sym;
            f.Funcs[fd.Name] = sym;
            f.OwnFuncs.Add(sym);
        }
    }

    void CheckSends(FileUnit f)
    {
        foreach (var send in f.Ast.Sends)
            foreach (var (name, pos) in send.Names)
            {
                if (!f.Classes.ContainsKey(name) && !f.Funcs.ContainsKey(name) && !f.States.ContainsKey(name))
                {
                    Error(pos, $"there's nothing called {name} in this file to send.{Suggest(name, f.Classes.Keys.Concat(f.Funcs.Keys).Concat(f.States.Keys))}");
                    continue;
                }
                if (f.Sent.ContainsKey(name)) Error(pos, $"{name} is already sent, on line {f.Sent[name].Line}.");
                else f.Sent[name] = pos;
            }
    }

    // ===== 2. connect files =====

    void Connect(FileUnit f)
    {
        foreach (var fetch in f.Ast.Fetches)
        {
            if (fetch.IsCs) continue;
            var other = fetch.Unit;
            if (other == null) continue; // already reported
            if (f.Classes.TryGetValue(other.Module, out var clash) && clash.File == f)
                Error(clash.Decl.Pos, $"{other.Module} is also the name of a file this one fetches from. Give the {clash.Kind} another name.");
            // Things are called by the file's own name (farm.season), so two files called farm can't both come in.
            if (f.ModuleFiles.TryGetValue(other.Module, out var same) && same != other)
            {
                Error(fetch.ModulePos, $"this file already fetches a different file called {other.Module}.lust. Two files with the same name can't both be fetched into one file.");
                continue;
            }
            f.ModuleFiles[other.Module] = other;
            f.Modules.Add(other.Module);
            var names = fetch.Names?.Select(n => (n.Name, n.Pos)).ToList()
                ?? other.Sent.Keys.Select(n => (n, fetch.ModulePos)).ToList();
            if (fetch.Names == null && names.Count == 0)
                Error(fetch.ModulePos, $"{other.Source.Display} doesn't send anything. Add send lines to it, like send Player;");
            foreach (var (name, pos) in names)
            {
                bool isClass = other.OwnClasses.Any(c => c.Name == name);
                bool isFunc = other.OwnFuncs.Any(fn => fn.Name == name);
                bool isState = other.OwnStates.Any(s => s.Name == name);
                if (!isClass && !isFunc && !isState)
                {
                    Error(pos, $"{other.Source.Display} has nothing called {name}.{Suggest(name, other.Sent.Keys)}");
                    continue;
                }
                if (!other.Sent.ContainsKey(name))
                {
                    Error(pos, $"{other.Source.Display} doesn't send {name}. Add send {name}; to {other.Source.Display}.");
                    continue;
                }
                if (fetch.Names != null)
                {
                    if (fetch.IsGet && !isFunc) { Error(pos, $"{name} is a {(isState ? "state" : other.Classes[name].Kind)}, so it's brought in with fetch: fetch {fetch.Path}:{name};"); continue; }
                    if (!fetch.IsGet && isFunc) { Error(pos, $"{name} is a function, so it's brought in with get: get {fetch.Path}:{name};"); continue; }
                }
                if (isClass) AddVisible(f.Classes, name, other.Classes[name], pos, f);
                else if (isFunc) AddVisible(f.Funcs, name, other.Funcs[name], pos, f);
                else
                {
                    var st = other.States[name];
                    f.States[$"{other.Module}.{name}"] = st;
                    if (name != "state" && !f.States.ContainsKey(name)) f.States[name] = st;
                }
            }
        }
    }

    void AddVisible<T>(Dictionary<string, T> into, string name, T sym, Pos pos, FileUnit f) where T : class
    {
        if (into.TryGetValue(name, out var existing))
        {
            if (existing != sym) Error(pos, $"{name} already means something else in {f.Source.Display}. Two things can't share a name.");
            return;
        }
        if (into is Dictionary<string, ClassSym> && f.Funcs.ContainsKey(name) || into is Dictionary<string, FuncSym> && f.Classes.ContainsKey(name))
        {
            Error(pos, $"{name} already means something else in {f.Source.Display}. Two things can't share a name.");
            return;
        }
        into[name] = sym;
    }

    // ===== types as written =====

    LType ResolveType(TypeRef t, FileUnit f)
    {
        switch (t.Kind)
        {
            case TypeKind.Array:
            {
                var elem = ResolveType(t.Elem!, f);
                return elem == LType.Error ? LType.Error : ArrayType.Of(elem);
            }
            case TypeKind.Map:
            {
                var elem = ResolveType(t.Elem!, f);
                return elem == LType.Error ? LType.Error : MapType.Of(elem);
            }
            case TypeKind.Func:
            {
                var prms = t.Columns.Select(c => ResolveType(c.Type, f)).ToList();
                var ret = t.Elem == null ? LType.None : ResolveType(t.Elem, f);
                return prms.Contains(LType.Error) || ret == LType.Error ? LType.Error : FuncType.Of(prms, ret);
            }
            case TypeKind.CArray:
            {
                var cols = new List<(string, LType)>();
                foreach (var col in t.Columns)
                {
                    if (cols.Any(c => c.Item1 == col.Name)) Error(col.Pos, $"there's already a column called {col.Name}.");
                    var ct = ResolveType(col.Type, f);
                    if (ct is CArrayType) Error(col.Type.Pos, "a cArray's column can't be another cArray.");
                    cols.Add((col.Name, ct));
                }
                if (cols.Any(c => c.Item2 == LType.Error)) return LType.Error;
                var type = CArrayType.Of(cols);
                lock (comp.CArrays) comp.CArrays.Add(type);
                return type;
            }
        }
        if (t.Module != null)
        {
            if (f.States.TryGetValue($"{t.Module}.{t.Name}", out var st)) return st.Type;
            if (!f.Modules.Contains(t.Module)) Error(t.Pos, $"{t.Module} isn't a file this one fetched from.");
            else Error(t.Pos, $"{t.Module}.{t.Name} isn't a state that was fetched here. Fetch it first: fetch {t.Module}:{t.Name};");
            return LType.Error;
        }
        var prim = LType.Primitive(t.Name);
        if (prim != null) return prim;
        if (f.Classes.TryGetValue(t.Name, out var cls))
        {
            if (cls.IsObject) { Error(t.Pos, $"{t.Name} is an object, not a type. Use it directly by name: {t.Name}.something"); return LType.Error; }
            return cls.Type;
        }
        if (f.States.TryGetValue(t.Name, out var state)) return state.Type;
        Error(t.Pos, $"Lust doesn't have a type called {t.Name}.{Suggest(t.Name, f.Classes.Keys.Concat(f.States.Keys).Concat(new[] { "int", "float", "string", "truth" }))} The types are int, float, string, truth, arrays, cArrays, classes and states.");
        return LType.Error;
    }

    // ===== 3. classes and functions =====

    void Build(ClassSym cls)
    {
        if (cls.Built) return;
        if (cls.Building)
        {
            Error(cls.Decl.ParentPos, $"{cls.Name} ends up being like itself. A chain of like can't go in a circle.");
            return;
        }
        cls.Building = true;
        var decl = cls.Decl;
        var f = cls.File;

        if (decl.Parent != null)
        {
            if (!f.Classes.TryGetValue(decl.Parent, out var parent))
                Error(decl.ParentPos, $"there's no class called {decl.Parent}.{Suggest(decl.Parent, f.Classes.Keys)}");
            else if (parent.IsObject)
                Error(decl.ParentPos, $"{decl.Parent} is an object. Only a class can be built on with like.");
            else
            {
                Build(parent);
                if (!parent.IsLike(cls))
                {
                    cls.Parent = parent;
                    cls.Fields.AddRange(parent.Fields);
                    foreach (var (k, v) in parent.Methods) cls.Methods[k] = v;
                }
            }
        }

        var own = new HashSet<string>();
        foreach (var fd in decl.Fields)
        {
            if (!own.Add(fd.Name)) { Error(fd.Pos, $"{cls.Name} already has a field called {fd.Name}."); continue; }
            LType type;
            if (fd.IsState)
            {
                var key = fd.Type.Module != null ? $"{fd.Type.Module}.{fd.Type.Name}" : fd.Type.Name;
                if (key == "state" && !f.States.ContainsKey("state"))
                {
                    var fetchedMain = f.States.Keys.Where(k => k.EndsWith(".state")).ToList();
                    if (fetchedMain.Count == 1) key = fetchedMain[0];
                }
                if (!f.States.TryGetValue(key, out var st))
                {
                    Error(fd.Pos, fd.Type.Name == "state" && fd.Type.Module == null
                        ? "this file has no main state. Declare one at the top: state = [idle, busy];"
                        : $"there's no state called {key} here.{Suggest(key, f.States.Keys)}");
                    continue;
                }
                type = st.Type;
            }
            else type = ResolveType(fd.Type, f);

            var inherited = cls.Field(fd.Name);
            if (inherited != null)
            {
                fd.Sym = inherited;
                if (inherited.Type != type && type != LType.Error)
                {
                    Error(fd.Pos, $"{cls.Parent!.Name} already has {fd.Name} as {inherited.Type.WithArticle}. A class that's like it can change its starting value, but not its type.");
                    continue;
                }
            }
            else
            {
                var sym = new FieldSym { Name = fd.Name, Type = type, Owner = cls, Pos = fd.Pos };
                cls.Fields.Add(sym);
                fd.Sym = sym;
            }
            if (fd.Default != null) cls.Defaults.Add((fd.Sym, fd.Default));
        }

        var ownMethods = new HashSet<string>();
        foreach (var md in decl.Methods)
        {
            if (!ownMethods.Add(md.Name)) { Error(md.NamePos, $"{cls.Name} already has a method called {md.Name}."); continue; }
            if (cls.Field(md.Name) != null) { Error(md.NamePos, $"{cls.Name} already has a field called {md.Name}. A method needs a different name."); continue; }
            var sym = new FuncSym { Name = md.Name, File = f, Decl = md, Owner = cls };
            md.Sym = sym;
            Signature(sym, cls);
            if (cls.Methods.TryGetValue(md.Name, out var parentMethod))
            {
                if (!parentMethod.Params.Select(p => p.Type).SequenceEqual(sym.Params.Select(p => p.Type)) || parentMethod.Return != sym.Return)
                    Error(md.NamePos, $"{md.Name} replaces {cls.Parent!.Name}'s {md.Name}, so it must take and give back the same types: method {parentMethod.Signature}");
                sym.IsOverride = true;
            }
            cls.Methods[md.Name] = sym;
            cls.OwnMethods.Add(sym);
        }
        foreach (var fd in decl.Fields)
            if (ownMethods.Contains(fd.Name)) Error(fd.Pos, $"{cls.Name} has a method called {fd.Name} too. A field needs a different name.");

        cls.Building = false;
        cls.Built = true;
    }

    void Signature(FuncSym fn, ClassSym? owner)
    {
        var d = fn.Decl;
        var seen = new HashSet<string>();
        foreach (var prm in d.Params)
        {
            if (!seen.Add(prm.Name)) Error(prm.Pos, $"there's already a parameter called {prm.Name}.");
            if (owner?.Field(prm.Name) != null) Error(prm.Pos, $"{owner.Name} has a field called {prm.Name}, so a parameter can't use that name. Try new{char.ToUpperInvariant(prm.Name[0])}{prm.Name[1..]}.");
            fn.Params.Add((prm.Name, ResolveType(prm.Type, fn.File)));
        }
        if (d.Returns != null) fn.Return = ResolveType(d.Returns, fn.File);
    }

    // ===== 4. bodies =====

    sealed class Ctx
    {
        public FileUnit File = null!;
        public ClassSym? Class;
        public FuncSym? Func;
        public bool TopLevel;
        public List<Dictionary<string, Variable>> Scopes = new() { new() };
        public int LoopDepth;
    }

    void CheckBodies(FileUnit f)
    {
        // Top-level code first, so functions can be told when they try to use a top-level variable.
        var top = new Ctx { File = f, TopLevel = true };
        foreach (var s in f.Ast.Stmts) Stmt(s, top);
        foreach (var scope in top.Scopes) f.TopVars.UnionWith(scope.Keys);

        foreach (var cls in f.OwnClasses)
        {
            var ctx = new Ctx { File = f, Class = cls };
            foreach (var (field, value) in cls.Defaults)
            {
                if (field.Type is StateType fst && value is Name opt && !fst.Sym.Options.Contains(opt.Id))
                {
                    Error(opt.Pos, NotAnOption(opt.Id, fst.Sym));
                    continue;
                }
                Expr(value, ctx, field.Type);
                ExpectFits(value, field.Type, $"{field.Name} is {field.Type.WithArticle}");
                if (!IsPlain(value)) Error(value.Pos, "a starting value in a has block must be a plain value, like 0, \"text\", true, nothing or []. Work things out in a method instead.");
            }
            foreach (var m in cls.OwnMethods) Body(m, cls);
        }
        foreach (var fn in f.OwnFuncs) Body(fn, null);
    }

    void Body(FuncSym fn, ClassSym? cls)
    {
        var ctx = new Ctx { File = fn.File, Class = cls, Func = fn };
        for (int i = 0; i < fn.Params.Count; i++)
        {
            var (name, type) = fn.Params[i];
            var pos = fn.Decl.Params[i].Pos;
            // A repeated parameter was already reported by Signature; give it a hidden name so it isn't reported twice.
            if (fn.Params.Take(i).Any(p => p.Name == name)) { fn.ParamVars.Add(new Variable { Id = name, Type = type, Pos = pos, CsName = $"v_{name}_dup{++counter}" }); continue; }
            fn.ParamVars.Add(Declare(ctx, name, type, pos));
        }
        Block(fn.Decl.Body, ctx);
        if (fn.Return != LType.None && fn.Return != LType.Error && !Returns(fn.Decl.Body))
            Error(fn.Decl.Body.Close, $"{fn.Name} must give back {fn.Return.WithArticle} at the end. Add give …; before this }}.");
    }

    static bool IsPlain(Expr e) => e switch
    {
        IntLit or FloatLit or StrLit or TruthLit or NothingLit => true,
        Unary { Op: UnOp.Not, Operand: TruthLit } => true,
        Name { Kind: NameKind.Option } => true,
        ArrayLit a => a.Items.All(IsPlain) && (a.Keys?.All(IsPlain) ?? true),
        _ => e.Type == LType.Error,
    };

    /// Does this always end with give? (So a function with a return type can't fall off the end.)
    static bool Returns(Stmt s) => s switch
    {
        Give => true,
        Block b => b.Stmts.Any(Returns),
        IsChain c => c.Otherwise != null && c.Branches.All(br => Returns(br.Body)) && Returns(c.Otherwise),
        Loop { Kind: LoopKind.Forever } l => !Stops(l.Body),
        _ => false,
    };

    /// Does this block contain a stop for the loop it's in (not one inside a nested loop)?
    static bool Stops(Stmt s) => s switch
    {
        Stop => true,
        Block b => b.Stmts.Any(Stops),
        IsChain c => c.Branches.Any(br => Stops(br.Body)) || (c.Otherwise != null && Stops(c.Otherwise)),
        _ => false,
    };

    // ----- names -----

    Variable? Lookup(Ctx ctx, string id)
    {
        for (int i = ctx.Scopes.Count - 1; i >= 0; i--)
            if (ctx.Scopes[i].TryGetValue(id, out var v)) return v;
        return null;
    }

    Variable Declare(Ctx ctx, string id, LType type, Pos pos, bool readOnly = false)
    {
        var existing = Lookup(ctx, id);
        if (existing != null) Error(pos, $"{id} is already declared on line {existing.Pos.Line}. Pick another name, or leave out the type to change it: {id} = …;");
        else if (ctx.Class?.Field(id) != null && ctx.Func != null && !ctx.Func.Params.Any(p => p.Name == id))
            Error(pos, $"{ctx.Class.Name} has a field called {id}, so a variable here can't use that name.");
        var v = new Variable { Id = id, Type = type, Pos = pos, CsName = $"v_{id}_{++counter}", TopLevel = ctx.TopLevel, ReadOnly = readOnly };
        if (existing == null) ctx.Scopes[^1][id] = v;
        return v;
    }

    void InScope(Ctx ctx, Action body)
    {
        ctx.Scopes.Add(new());
        try { body(); } finally { ctx.Scopes.RemoveAt(ctx.Scopes.Count - 1); }
    }

    /// All the states this file can see, each once.
    static IEnumerable<StateSym> VisibleStates(FileUnit f) => f.States.Values.Distinct();

    // ----- statements -----

    void Stmt(Stmt s, Ctx ctx)
    {
        switch (s)
        {
            case VarDecl d:
            {
                var type = ResolveType(d.TypeRef, ctx.File);
                Expr(d.Init, ctx, type);
                ExpectFits(d.Init, type, $"{d.Id} is {type.WithArticle}");
                d.Var = Declare(ctx, d.Id, type, d.IdPos);
                break;
            }
            case Assign a:
            {
                if (a.Target is Name nm && Unknown(nm, ctx))
                {
                    Expr(a.Value, ctx);
                    if (!ctx.File.Ast.BrokenNames.Contains(nm.Id))
                        Error(nm.Pos, $"there's nothing called {nm.Id} yet. Declare it first, with its type: {(a.Value.Type is { } vt && vt != LType.Error && vt != LType.Nothing && vt != LType.None ? vt.Name : "int")} {nm.Id} = …;");
                    break;
                }
                var target = Target(a.Target, ctx);
                Expr(a.Value, ctx, target);
                if (target != LType.Error) ExpectFits(a.Value, target, $"{Describe(a.Target)} is {target.WithArticle}");
                break;
            }
            case StateSet ss:
            {
                var t = Expr(ss.Target, ctx);
                if (t == LType.Error) break;
                if (t is not StateType st)
                {
                    Error(ss.Target.Pos, $"{Describe(ss.Target)} is {t.WithArticle}, not a state, so it can't be set with is. Use = to change it: {Describe(ss.Target)} = …;");
                    break;
                }
                CheckWritable(ss.Target);
                if (!st.Sym.Options.Contains(ss.Option))
                    Error(ss.OptionPos, NotAnOption(ss.Option, st.Sym));
                ss.State = st.Sym;
                break;
            }
            case ExprStmt e:
            {
                var t = Expr(e.Expr, ctx);
                if (t != LType.None && t != LType.Error)
                {
                    var what = e.Expr is Call c ? $"{c.Callee}(…)" : e.Expr is MethodCall m ? $"{m.Method}(…)" : "this";
                    Error(e.Pos, $"{what} gives back {t.WithArticle}, but nothing uses it. Put it in a variable: {t.Name} x = …;");
                }
                break;
            }
            case Wait w:
            {
                var t = Expr(w.Seconds, ctx);
                if (!t.IsNumber && t != LType.Error) Error(w.Seconds.Pos, $"wait takes a number of seconds, but this is {t.WithArticle}.");
                if (w.Seconds is IntLit { Value: < 0 } or FloatLit { Value: < 0 }) Error(w.Seconds.Pos, "can't wait a negative number of seconds.");
                break;
            }
            case Give g:
            {
                if (ctx.Func == null) { Error(g.Pos, "give only works inside a function or method."); if (g.Value != null) Expr(g.Value, ctx); break; }
                var ret = ctx.Func.Return;
                if (g.Value == null)
                {
                    if (ret != LType.None && ret != LType.Error) Error(g.Pos, $"{ctx.Func.Name} gives back {ret.WithArticle}, so give needs one: give …;");
                    break;
                }
                Expr(g.Value, ctx, ret == LType.None ? null : ret);
                if (ret == LType.None)
                    Error(g.Value.Pos, $"{ctx.Func.Name} doesn't give anything back. To give {g.Value.Type.WithArticle}, say so: {(ctx.Func.Owner != null ? "method" : "do")} {g.Value.Type.Name} {ctx.Func.Name}(…)");
                else ExpectFits(g.Value, ret, $"{ctx.Func.Name} gives back {ret.WithArticle}");
                break;
            }
            case IsChain chain:
                foreach (var (check, body) in chain.Branches)
                {
                    ExpectTruth(check, "a question", ctx);
                    InScope(ctx, () => Block(body, ctx));
                }
                if (chain.Otherwise != null) InScope(ctx, () => Block(chain.Otherwise, ctx));
                break;
            case Loop loop:
                LoopStmt(loop, ctx);
                break;
            case Stop:
                if (ctx.LoopDepth == 0) Error(s.Pos, "stop only works inside a loop.");
                break;
            case Skip:
                if (ctx.LoopDepth == 0) Error(s.Pos, "skip only works inside a loop.");
                break;
            case Block b:
                InScope(ctx, () => Block(b, ctx));
                break;
        }
    }

    void Block(Block b, Ctx ctx)
    {
        foreach (var s in b.Stmts) Stmt(s, ctx);
    }

    void LoopStmt(Loop loop, Ctx ctx)
    {
        switch (loop.Kind)
        {
            case LoopKind.Times:
            {
                var t = Expr(loop.Expr!, ctx);
                if (t != LType.Int && t != LType.Error)
                    Error(loop.Expr!.Pos, t == LType.Float ? "loop needs a whole number of times. Use .int() to drop the part after the point." : $"loop needs a number of times, but this is {t.WithArticle}.");
                break;
            }
            case LoopKind.While:
                ExpectTruth(loop.Expr!, "loop ( … )", ctx);
                break;
        }
        ctx.LoopDepth++;
        InScope(ctx, () =>
        {
            if (loop.Kind == LoopKind.Each) EachVars(loop, ctx);
            Block(loop.Body, ctx);
        });
        ctx.LoopDepth--;
    }

    void EachVars(Loop loop, Ctx ctx)
    {
        var source = loop.Expr!;
        var vars = loop.Vars.Select(v => (v.Name, Type: ResolveType(v.Type, ctx.File), v.Pos, v.Type)).ToList();

        // loop (season s in season): the source is a state's name
        StateSym? stateSource = null;
        if (source is Name n && Lookup(ctx, n.Id) == null && ctx.Class?.Field(n.Id) == null && ctx.File.States.TryGetValue(n.Id, out var st1)) stateSource = st1;
        if (source is FieldGet { Target: Name mod } fg && Lookup(ctx, mod.Id) == null && ctx.File.Modules.Contains(mod.Id) && ctx.File.States.TryGetValue($"{mod.Id}.{fg.Field}", out var st2)) stateSource = st2;
        if (stateSource != null)
        {
            loop.Each = EachKind.State;
            loop.EachState = stateSource;
            if (vars.Count != 1) { Error(loop.Pos, $"a loop through {stateSource.Display} takes one name: loop ({stateSource.Display} s in {stateSource.Display})"); return; }
            if (vars[0].Type != stateSource.Type && vars[0].Type != LType.Error)
                Error(vars[0].Item4.Pos, $"{vars[0].Name} goes through {stateSource.Display}'s options, so its type is {stateSource.Display}.");
            loop.LoopVars.Add(Declare(ctx, vars[0].Name, stateSource.Type, vars[0].Pos, readOnly: true));
            return;
        }

        var t = Expr(source, ctx);
        void DeclareAll()
        {
            foreach (var v in vars) loop.LoopVars.Add(Declare(ctx, v.Name, v.Type, v.Pos, readOnly: true));
        }
        switch (t)
        {
            case ArrayType at:
                loop.Each = EachKind.Array;
                if (vars.Count != 1) Error(loop.Pos, $"a loop through an array takes one name: loop ({at.Elem.Name} x in {Describe(source)})");
                else if (vars[0].Type != LType.Error && !LType.Fits(at.Elem, vars[0].Type))
                    Error(vars[0].Item4.Pos, $"{Describe(source)} holds {Plural(at.Elem)}, so write loop ({at.Elem.Name} {vars[0].Name} in {Describe(source)})");
                DeclareAll();
                break;
            case MapType mt:
            {
                loop.Each = EachKind.Map;
                var want = $"loop (string key, {mt.Elem.Name} value in {Describe(source)})";
                if (vars.Count != 2) { Error(loop.Pos, $"a loop through a map names the key and the value: {want}"); DeclareAll(); break; }
                if (vars[0].Type != LType.String && vars[0].Type != LType.Error) Error(vars[0].Item4.Pos, $"the first name is the key, which is text: {want}");
                if (vars[1].Type != LType.Error && !LType.Fits(mt.Elem, vars[1].Type)) Error(vars[1].Item4.Pos, $"{Describe(source)} holds {Plural(mt.Elem)}: {want}");
                DeclareAll();
                break;
            }
            case CArrayType ct:
            {
                loop.Each = EachKind.CArray;
                var want = $"loop (int id, {string.Join(", ", ct.Columns.Select(c => $"{c.Type.Name} {c.Name}"))} in {Describe(source)})";
                if (vars.Count != ct.Columns.Count + 1) { Error(loop.Pos, $"a loop through a cArray names the id and every column: {want}"); DeclareAll(); break; }
                if (vars[0].Type != LType.Int && vars[0].Type != LType.Error) Error(vars[0].Item4.Pos, $"the first name is the row's id, an int: {want}");
                for (int i = 0; i < ct.Columns.Count; i++)
                    if (vars[i + 1].Type != ct.Columns[i].Type && vars[i + 1].Type != LType.Error)
                        Error(vars[i + 1].Item4.Pos, $"the column {ct.Columns[i].Name} is {ct.Columns[i].Type.WithArticle}: {want}");
                DeclareAll();
                break;
            }
            case var _ when t == LType.Error:
                DeclareAll();
                break;
            default:
                Error(source.Pos, $"loop ( … in …) goes through an array, a cArray, a map or a state, but {Describe(source)} is {t.WithArticle}.");
                DeclareAll();
                break;
        }
    }

    static string Plural(LType t) => t switch
    {
        ClassType c => c.Name + "s",
        _ => t.Name + "s",
    };

    /// The type of something being assigned to, after checking it can be changed.
    LType Target(Expr target, Ctx ctx)
    {
        var t = Expr(target, ctx);
        if (t == LType.Error) return t;
        if (!CheckWritable(target)) return LType.Error;
        return t;
    }

    bool CheckWritable(Expr target)
    {
        switch (target)
        {
            case Name { Kind: NameKind.Local, Var: { ReadOnly: true } v }:
                Error(target.Pos, $"{v.Id} is the loop's current item, so it can't be changed here.");
                return false;
            case Name { Kind: NameKind.Local or NameKind.Field }:
                return true;
            case Name { Kind: NameKind.Object } n:
                Error(target.Pos, $"{n.Id} is an object; it can't be replaced. Change its fields instead: {n.Id}.something = …;");
                return false;
            case Name { Kind: NameKind.Option } n:
                Error(target.Pos, $"{n.Id} is a state option; it can't be changed.");
                return false;
            case FieldGet { Kind: FieldKind.Column, Field: "id" }:
                Error(target.Pos, "a row's id can't be changed.");
                return false;
            case FieldGet:
                return true;
            case Index { Target.Type: CArrayType }:
                Error(target.Pos, "a cArray row can't be replaced in one go. Change its columns: bag[2].amount = 5;");
                return false;
            case Index:
                return true;
            default:
                Error(target.Pos, "this can't be changed.");
                return false;
        }
    }

    void ExpectTruth(Expr e, string where, Ctx ctx)
    {
        var t = Expr(e, ctx, LType.Truth);
        if (t == LType.Truth || t == LType.Error) return;
        var what = e is Name n ? n.Id : "this";
        var name = e is Name n2 ? n2.Id : (t.IsNumber ? "hp" : "name");
        var example = where switch
        {
            "loop ( … )" => t.IsNumber ? $"loop ({name} > 0)" : $"loop ({name} \"Ada\")",
            "either" => t.IsNumber ? $"either {name} 0? … or …" : $"either {name} \"Ada\"? … or …",
            _ => t.IsNumber ? $"is {name} 0?" : t is StateType st ? $"is {name} {st.Sym.Options[0]}?" : t is ClassType ? $"is {name} nothing?" : $"is {name} \"Ada\"?",
        };
        var hint = t.IsNumber || t == LType.String || t is StateType || t is ClassType
            ? (e is Name ? $" Try {example}" : $" Compare it with something, like {example}")
            : "";
        Error(e.Pos, $"{where} needs a yes-or-no answer, but {what} is {t.WithArticle}.{hint}");
    }

    void ExpectFits(Expr e, LType wanted, string subject)
    {
        var actual = e.Type;
        if (LType.Fits(actual, wanted)) return;
        if (actual == LType.None) { Error(e.Pos, $"{subject}, but this doesn't give anything back."); return; }
        string fix = (actual, wanted) switch
        {
            _ when actual == LType.None => " It doesn't give anything back.",
            _ when actual == LType.Float && wanted == LType.Int => " Use .int() to drop the part after the point.",
            _ when wanted == LType.String && (actual.IsNumber || actual == LType.Truth || actual is StateType) => " Use .text() to turn it into text.",
            _ when actual == LType.String && wanted.IsNumber => $" Use .{wanted.Name}() to turn text into a number.",
            _ when actual == LType.Nothing => " Only things made from classes can be nothing.",
            _ when actual is ClassType ac && wanted is ClassType wc => $" A {ac.Name} isn't like a {wc.Name}.",
            _ when actual is FuncType af && af.Return == wanted => " Add () to call it.",
            _ when actual is FuncType && wanted is FuncType => " A function only fits where it takes and gives back exactly the same types.",
            _ => "",
        };
        Error(e.Pos, $"{subject}, but this is {actual.WithArticle}.{fix}");
    }

    static string Describe(Expr e) => e switch
    {
        Name n => n.Id,
        FieldGet f => $"{Describe(f.Target)}.{f.Field}",
        Index i => $"{Describe(i.Target)}[…]",
        _ => "this",
    };

    // ----- expressions -----

    LType Expr(Expr e, Ctx ctx, LType? expected = null)
    {
        e.Type = ExprInner(e, ctx, expected);
        return e.Type;
    }

    LType ExprInner(Expr e, Ctx ctx, LType? expected)
    {
        switch (e)
        {
            case IntLit: return LType.Int;
            case FloatLit: return LType.Float;
            case StrLit: return LType.String;
            case TruthLit: return LType.Truth;
            case NothingLit: return LType.Nothing;
            case Name n: return NameType(n, ctx, expected);
            case Unary u:
            {
                var t = Expr(u.Operand, ctx);
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
            case Binary b: return BinaryType(b, ctx);
            case Same s:
            {
                LType l, r;
                if (AmbiguousOption(s.Left, ctx))
                {
                    // `is idle s?`: work out the right side first, so the option knows which state it's from.
                    r = Expr(s.Right, ctx);
                    l = Expr(s.Left, ctx, r == LType.Error ? null : r);
                }
                else
                {
                    l = Expr(s.Left, ctx);
                    r = Expr(s.Right, ctx, l == LType.Error || l == LType.Nothing ? null : l);
                }
                if (l == LType.Error || r == LType.Error) return LType.Truth;
                if (l == LType.None || r == LType.None) { Error(s.Pos, "one side of this question doesn't give a value."); return LType.Truth; }
                if (LType.Fits(l, r) || LType.Fits(r, l) || (l.IsNumber && r.IsNumber)) return LType.Truth;
                string hint = (r == LType.String && l.IsNumber) || (l == LType.String && r.IsNumber) ? " Use .text() on the number to compare them as text." : "";
                Error(s.Right.Pos, $"can't compare {l.WithArticle} with {r.WithArticle}.{hint}");
                return LType.Truth;
            }
            case IsLike il:
            {
                var t = Expr(il.Value, ctx);
                if (!ctx.File.Classes.TryGetValue(il.Class, out var target))
                {
                    Error(il.ClassPos, $"there's no class called {il.Class}.{Suggest(il.Class, ctx.File.Classes.Keys)}");
                    return LType.Truth;
                }
                il.Sym = target;
                if (t == LType.Error) return LType.Truth;
                if (t is not ClassType ct) { Error(il.Value.Pos, $"like asks what kind of thing something is, but this is {t.WithArticle}."); return LType.Truth; }
                if (!target.IsLike(ct.Sym) && !ct.Sym.IsLike(target))
                    Error(il.ClassPos, $"{ct.Sym.Name.WithIndefinite()} can never be {target.Name.WithIndefinite()}, so this question is always no.");
                return LType.Truth;
            }
            case Either ei:
            {
                ExpectTruth(ei.Check, "either", ctx);
                LType y, n;
                if (expected == null && AmbiguousOption(ei.Yes, ctx))
                {
                    n = Expr(ei.No, ctx);
                    y = Expr(ei.Yes, ctx, n == LType.Error || n == LType.Nothing ? null : n);
                }
                else
                {
                    y = Expr(ei.Yes, ctx, expected);
                    n = Expr(ei.No, ctx, expected ?? (y == LType.Error || y == LType.Nothing ? null : y));
                }
                if (y == LType.Error || n == LType.Error) return y == LType.Error ? n : y;
                if (y == n) return y;
                if (y.IsNumber && n.IsNumber) return LType.Float;
                if (LType.Fits(n, y)) return y;
                if (LType.Fits(y, n)) return n;
                Error(ei.No.Pos, $"both sides of either must be the same type, but one is {y.WithArticle} and the other is {n.WithArticle}.");
                return LType.Error;
            }
            case Call c: return CallType(c, ctx);
            case CallValue cv:
            {
                var t = Expr(cv.Target, ctx);
                if (t is FuncType ft) return CallFunctionValue(ft, cv.Args, "it", cv.Pos, ctx);
                foreach (var a in cv.Args) Expr(a, ctx);
                if (t != LType.Error) Error(cv.Pos, $"this is {t.WithArticle}, not a function, so it can't be called.");
                return LType.Error;
            }
            case MethodRef mr:
            {
                var t = Expr(mr.Target, ctx);
                if (t == LType.Error) return t;
                if (t is ClassType ct && ct.Sym.Methods.TryGetValue(mr.Method, out var meth)) { mr.Func = meth; return FuncOf(meth); }
                if (t is ClassType ct2 && ct2.Sym.Field(mr.Method) != null)
                {
                    Error(mr.MethodPos, $"{mr.Method} is a field: {Describe(mr.Target)}.{mr.Method}");
                    return LType.Error;
                }
                if (t is CsType) { Error(mr.MethodPos, $"add () to call it: {Describe(mr.Target)}:{mr.Method}(…). A C# method can't be kept as a value."); return LType.Error; }
                Error(mr.MethodPos, t is ClassType ct3 ? $"{ct3.Sym.Name} has no method called {mr.Method}.{Suggest(mr.Method, ct3.Sym.Methods.Keys)}" : $"{Describe(mr.Target)} is {t.WithArticle}, which has no methods.");
                return LType.Error;
            }
            case MethodCall mc: return MethodType(mc, ctx);
            case Conversion cv: return ConversionType(cv, ctx);
            case FieldGet fg:
            {
                // fighting.blocking: an option inside another option
                var path = DottedPath(fg);
                if (path != null && fg.Target is not null && RootIsFree(fg, ctx))
                {
                    StateSym? st = expected is StateType est && est.Sym.Options.Contains(path) ? est.Sym : null;
                    if (st == null)
                    {
                        var owners = VisibleStates(ctx.File).Where(x => x.Options.Contains(path)).ToList();
                        if (owners.Count == 1) st = owners[0];
                        else if (owners.Count > 1) { Error(fg.Pos, $"{path} is an option of {string.Join(" and ", owners.Select(o => o.Display))}, so Lust can't tell which one is meant here."); return LType.Error; }
                    }
                    if (st != null) { fg.Kind = FieldKind.Option; fg.OptionState = st; fg.OptionPath = path; return st.Type; }
                }
                return FieldType(fg, ctx);
            }
            case Index ix:
            {
                var t = Expr(ix.Target, ctx);
                if (t is MapType mt)
                {
                    var mk = Expr(ix.Key, ctx, LType.String);
                    if (mk != LType.String && mk != LType.Error)
                        Error(ix.Key.Pos, $"a map's keys are text, but this is {mk.WithArticle}.{(mk.IsNumber ? " Use .text() to turn it into text." : "")}");
                    return mt.Elem;
                }
                var k = Expr(ix.Key, ctx, LType.Int);
                if (t == LType.Error) return t;
                if (k != LType.Int && k != LType.Error) Error(ix.Key.Pos, $"{(t is CArrayType ? "a row's id" : "an item's number")} is an int, but this is {k.WithArticle}.");
                if (t is ArrayType at) return at.Elem;
                if (t is CArrayType ct) return ct.Row;
                Error(ix.Pos, $"[ ] picks from an array, a cArray or a map, but {Describe(ix.Target)} is {t.WithArticle}.");
                return LType.Error;
            }
            case Length len:
            {
                var t = Expr(len.Target, ctx);
                if (t == LType.Error || t is ArrayType or CArrayType or MapType || t == LType.String) return LType.Int;
                Error(len.Pos, $"# counts the items in an array, cArray or map, or the letters in text, but this is {t.WithArticle}.");
                return LType.Int;
            }
            case New nw: return NewType(nw, ctx);
            case ArrayLit al: return ArrayLitType(al, ctx, expected);
        }
        return LType.Error;
    }

    /// "x isn't an option of state…", pointing out options that are inside others.
    static string NotAnOption(string opt, StateSym st)
    {
        var inside = st.Options.FirstOrDefault(o => o.EndsWith("." + opt));
        if (inside != null) return $"{opt} is inside {inside[..inside.LastIndexOf('.')]}, so write {inside}.";
        return $"{opt} isn't an option of {st.Display}. The options are {string.Join(", ", st.Options)}.{Suggest(opt, st.Options)}";
    }

    /// a.b.c written as names with dots, or null.
    static string? DottedPath(Expr e) => e switch
    {
        Name n => n.Id,
        FieldGet { Target: var t } f when DottedPath(t) is string p => p + "." + f.Field,
        _ => null,
    };

    /// Is the first name in a.b.c something other than a variable, field, object or file? Then it may be an option.
    bool RootIsFree(Expr e, Ctx ctx)
    {
        while (e is FieldGet f) e = f.Target;
        if (e is not Name n) return false;
        return Lookup(ctx, n.Id) == null && ctx.Class?.Field(n.Id) == null && !ctx.File.Classes.ContainsKey(n.Id) && !ctx.File.Modules.Contains(n.Id);
    }

    /// Is this a bare option name that belongs to more than one state here?
    bool AmbiguousOption(Expr e, Ctx ctx) =>
        e is Name n && Lookup(ctx, n.Id) == null && ctx.Class?.Field(n.Id) == null && !ctx.File.Classes.ContainsKey(n.Id) &&
        VisibleStates(ctx.File).Count(st => st.Options.Contains(n.Id)) > 1;

    /// A plain name that means nothing here at all (not a variable, field, object, option, function or file).
    bool Unknown(Name n, Ctx ctx) =>
        !Parser.Builtins.Contains(n.Id) && Lookup(ctx, n.Id) == null && ctx.Class?.Field(n.Id) == null &&
        !ctx.File.Classes.ContainsKey(n.Id) && !ctx.File.Funcs.ContainsKey(n.Id) && !ctx.File.States.ContainsKey(n.Id) &&
        !ctx.File.Modules.Contains(n.Id) && !VisibleStates(ctx.File).Any(s => s.Options.Contains(n.Id)) &&
        (ctx.TopLevel || !ctx.File.TopVars.Contains(n.Id));

    LType NameType(Name n, Ctx ctx, LType? expected)
    {
        if (Parser.Builtins.Contains(n.Id))
        {
            Error(n.Pos, n.Id == "wait" ? "wait goes on its own line: wait 1;"
                : expected is FuncType ? $"{n.Id} is built into Lust, so it can't be kept as a value. Wrap it in your own function: do show(string s) {{ {n.Id}(s); }}"
                : $"{n.Id} is something you call: {n.Id}(…)");
            return LType.Error;
        }
        var v = Lookup(ctx, n.Id);
        if (v != null) { n.Kind = NameKind.Local; n.Var = v; return v.Type; }
        var field = ctx.Class?.Field(n.Id);
        if (field != null) { n.Kind = NameKind.Field; n.Field = field; return field.Type; }
        if (ctx.File.Classes.TryGetValue(n.Id, out var cls))
        {
            if (cls.IsObject) { n.Kind = NameKind.Object; n.Object = cls; return cls.Type; }
            Error(n.Pos, $"{n.Id} is a class, a blueprint. Make one with new {n.Id}, or use a variable that holds one.");
            return LType.Error;
        }
        if (expected is StateType est && est.Sym.Options.Contains(n.Id))
        {
            n.Kind = NameKind.Option; n.State = est.Sym; return est;
        }
        var owners = VisibleStates(ctx.File).Where(s => s.Options.Contains(n.Id)).ToList();
        if (owners.Count == 1) { n.Kind = NameKind.Option; n.State = owners[0]; return owners[0].Type; }
        if (owners.Count > 1)
        {
            Error(n.Pos, $"{n.Id} is an option of {string.Join(" and ", owners.Select(o => o.Display))}, so Lust can't tell which one is meant here.");
            return LType.Error;
        }
        var nested = VisibleStates(ctx.File).SelectMany(st => st.Options).FirstOrDefault(o => o.EndsWith("." + n.Id));
        if (nested != null)
        {
            Error(n.Pos, $"{n.Id} is inside {nested[..nested.LastIndexOf('.')]}, so write {nested}.");
            return LType.Error;
        }
        // A function or method on its own, without (), is the function itself, kept as a value.
        if (ctx.Class != null && ctx.Class.Methods.TryGetValue(n.Id, out var meth)) { n.Kind = NameKind.Method; n.Func = meth; return FuncOf(meth); }
        if (ctx.File.Funcs.TryGetValue(n.Id, out var fnv)) { n.Kind = NameKind.Function; n.Func = fnv; return FuncOf(fnv); }
        if (ctx.File.States.TryGetValue(n.Id, out var stype))
        {
            Error(n.Pos, $"{n.Id} is a state, a list of options. Use one of its options, like {stype.Options[0]}.");
            return LType.Error;
        }
        if (ctx.File.Modules.Contains(n.Id))
        {
            Error(n.Pos, $"{n.Id} is a file. Use the things it sent by their own names.");
            return LType.Error;
        }
        if (ctx.File.Ast.BrokenNames.Contains(n.Id)) return LType.Error;
        if (!ctx.TopLevel && ctx.File.TopVars.Contains(n.Id))
        {
            Error(n.Pos, $"{n.Id} is a variable at the top of the file, and functions and methods can't see those. Pass it in as a parameter, or keep it in an object.");
            return LType.Error;
        }
        var names = ctx.Scopes.SelectMany(s => s.Keys).Concat(ctx.Class?.Fields.Select(f => f.Name) ?? Enumerable.Empty<string>()).Concat(ctx.File.Classes.Where(c => c.Value.IsObject).Select(c => c.Key));
        var suggestion = Suggest(n.Id, names);
        Error(n.Pos, suggestion.Length > 0
            ? $"there's nothing called {n.Id} here.{suggestion}"
            : $"there's nothing called {n.Id} here. Declare it first, like int {n.Id} = 0;");
        return LType.Error;
    }

    LType FieldType(FieldGet fg, Ctx ctx)
    {
        if (fg.Target is Name mod && Lookup(ctx, mod.Id) == null && ctx.Class?.Field(mod.Id) == null && ctx.File.Modules.Contains(mod.Id))
        {
            Error(fg.Pos, ctx.File.States.ContainsKey($"{mod.Id}.{fg.Field}")
                ? $"{mod.Id}.{fg.Field} is a state, a list of options. Use one of its options instead."
                : $"{mod.Id} is a file. Use the things it sent by their own names: {fg.Field}");
            return LType.Error;
        }
        var t = Expr(fg.Target, ctx);
        if (t == LType.Error) return t;
        if (Parser_Conversions.Contains(fg.Field) && !(t is ClassType cc && cc.Sym.Field(fg.Field) != null) && t is not RowType)
        {
            Error(fg.FieldPos, $"add () to call it: .{fg.Field}()");
            return LType.Error;
        }
        switch (t)
        {
            case ClassType ct:
            {
                var f = ct.Sym.Field(fg.Field);
                if (f == null)
                {
                    var hint = ct.Sym.Methods.ContainsKey(fg.Field) ? $" {fg.Field} is a method: {Describe(fg.Target)}:{fg.Field}(…)" : Suggest(fg.Field, ct.Sym.Fields.Select(x => x.Name));
                    Error(fg.FieldPos, $"{ct.Sym.Name} has no field called {fg.Field}.{hint}");
                    return LType.Error;
                }
                fg.Kind = FieldKind.Field;
                fg.Sym = f;
                return f.Type;
            }
            case RowType rt:
            {
                var col = rt.Table.Column(fg.Field);
                if (col == null)
                {
                    Error(fg.FieldPos, $"the rows have no column called {fg.Field}. The columns are id, {string.Join(", ", rt.Table.Columns.Select(c => c.Name))}.{Suggest(fg.Field, rt.Table.Columns.Select(c => c.Name))}");
                    return LType.Error;
                }
                fg.Kind = FieldKind.Column;
                return col;
            }
            case CArrayType:
                Error(fg.FieldPos, $"pick a row first, by its id: {Describe(fg.Target)}[1].{fg.Field}");
                return LType.Error;
            default:
                Error(fg.FieldPos, t == LType.Nothing ? "nothing has no fields." : $"{Describe(fg.Target)} is {t.WithArticle}, which has no fields.{(fg.Field is "length" or "count" or "size" ? $" To count items, use #{Describe(fg.Target)}." : "")}");
                return LType.Error;
        }
    }

    static readonly HashSet<string> Parser_Conversions = new() { "text", "int", "float", "isNumber" };

    LType ConversionType(Conversion cv, Ctx ctx)
    {
        var t = Expr(cv.Target, ctx);
        if (t == LType.Error) return t;
        switch (cv.Method)
        {
            case "text":
                if (t == LType.String) { Error(cv.MethodPos, "this is already text."); return t; }
                if (t.IsNumber || t == LType.Truth || t is StateType) return LType.String;
                break;
            case "int":
                if (t == LType.Float || t == LType.String) return LType.Int;
                if (t == LType.Int) { Error(cv.MethodPos, "this is already an int."); return t; }
                break;
            case "float":
                if (t == LType.Int || t == LType.String) return LType.Float;
                if (t == LType.Float) { Error(cv.MethodPos, "this is already a float."); return t; }
                break;
            case "isNumber":
                if (t == LType.String) return LType.Truth;
                break;
        }
        Error(cv.MethodPos, $".{cv.Method}() doesn't work on {t.WithArticle}.{(cv.Method == "text" && t is ClassType ? " print shows what's inside a class." : "")}");
        return LType.Error;
    }

    LType BinaryType(Binary b, Ctx ctx)
    {
        var l = Expr(b.Left, ctx);
        var r = Expr(b.Right, ctx);
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
                string fix = otherType.IsNumber || otherType == LType.Truth || otherType is StateType
                    ? (other is Name n ? $" Use {n.Id}.text() to turn {n.Id} into text." : other is FieldGet fg ? $" Use {Describe(fg)}.text() to turn it into text." : " Use (…).text() to turn it into text.")
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

    void Args(List<Expr> args, IReadOnlyList<(string Name, LType Type)> prms, string callee, Pos pos, Ctx ctx, bool unnamed = false)
    {
        if (args.Count != prms.Count)
        {
            foreach (var a in args) Expr(a, ctx);
            var want = prms.Count == 0 ? "nothing" : string.Join(", ", prms.Select(p => unnamed ? p.Type.Name : $"{p.Type.Name} {p.Name}"));
            Error(pos, prms.Count == 0
                ? $"{callee} takes nothing, but got {args.Count}. Leave the () empty."
                : $"{callee} takes {(prms.Count == 1 ? "1 thing" : $"{prms.Count} things")} ({want}), but got {args.Count}.");
            return;
        }
        for (int i = 0; i < args.Count; i++)
        {
            Expr(args[i], ctx, prms[i].Type);
            ExpectFits(args[i], prms[i].Type, $"{callee}'s {prms[i].Name} is {prms[i].Type.WithArticle}");
        }
    }

    static FuncType FuncOf(FuncSym fn) => FuncType.Of(fn.Params.Select(p => p.Type).ToList(), fn.Return);

    static readonly string[] Ordinals = { "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth" };
    static List<(string, LType)> Unnamed(FuncType ft) => ft.Params.Select((t, i) => (i < Ordinals.Length ? Ordinals[i] + " thing" : $"thing {i + 1}", t)).ToList();

    /// Calls a function kept as a value. The checker has already worked out the target's type.
    LType CallFunctionValue(FuncType ft, List<Expr> args, string callee, Pos pos, Ctx ctx)
    {
        Args(args, Unnamed(ft), callee, pos, ctx, unnamed: true);
        return ft.Return;
    }

    LType CallType(Call c, Ctx ctx)
    {
        switch (c.Callee)
        {
            case "print":
            {
                var types = c.Args.Select(a => Expr(a, ctx)).ToList();
                if (c.Args.Count != 1) { Error(c.Pos, "print takes one thing to show: print(x)"); return LType.None; }
                if (types[0] == LType.None) Error(c.Args[0].Pos, "this doesn't give anything to print.");
                if (types[0] == LType.Nothing) { } // prints "nothing"
                return LType.None;
            }
            case "random":
            {
                var types = c.Args.Select(a => Expr(a, ctx, LType.Int)).ToList();
                if (c.Args.Count != 2) { Error(c.Pos, "random takes two whole numbers, lowest and highest: random(1, 6)"); return LType.Int; }
                for (int i = 0; i < 2; i++)
                    if (types[i] != LType.Int && types[i] != LType.Error)
                        Error(c.Args[i].Pos, $"random takes whole numbers, but this is {types[i].WithArticle}.");
                if (c.Args[0] is IntLit lo && c.Args[1] is IntLit hi && lo.Value > hi.Value)
                    Error(c.Args[0].Pos, "random's lowest number comes first: random(1, 6)");
                return LType.Int;
            }
            case "ask":
            {
                var types = c.Args.Select(a => Expr(a, ctx, LType.String)).ToList();
                if (c.Args.Count > 1) Error(c.Pos, "ask takes one question to show: ask(\"What's your name?\")");
                else if (c.Args.Count == 1 && types[0] != LType.String && types[0] != LType.Error)
                    Error(c.Args[0].Pos, $"ask shows text, but this is {types[0].WithArticle}.{(types[0].IsNumber ? " Use .text() to turn it into text." : "")}");
                return LType.String;
            }
            case "wait":
                foreach (var a in c.Args) Expr(a, ctx);
                Error(c.Pos, "wait goes on its own line, without (): wait 1;");
                return LType.None;
        }
        // A variable, parameter or field that holds a function: call what it holds.
        if (Lookup(ctx, c.Callee) is { Type: FuncType } || ctx.Class?.Field(c.Callee) is { Type: FuncType })
        {
            c.Value = new Name { Pos = c.Pos, Id = c.Callee };
            var ft = (FuncType)Expr(c.Value, ctx);
            return CallFunctionValue(ft, c.Args, c.Callee, c.Pos, ctx);
        }
        FuncSym? fn = null;
        if (ctx.Class != null && ctx.Class.Methods.TryGetValue(c.Callee, out var m)) fn = m;
        else if (ctx.File.Funcs.TryGetValue(c.Callee, out var f)) fn = f;
        if (fn == null)
        {
            foreach (var a in c.Args) Expr(a, ctx);
            if (ctx.File.Classes.TryGetValue(c.Callee, out var cls))
            {
                Error(c.Pos, cls.IsObject ? $"{c.Callee} is an object, not a function. Call its methods with :, like {c.Callee}:something()" : $"{c.Callee} is a class. Make one with new {c.Callee} {{ … }}");
                return LType.Error;
            }
            var v = Lookup(ctx, c.Callee);
            if (v != null) { Error(c.Pos, $"{c.Callee} is {v.Type.WithArticle}, not something you can call."); return LType.Error; }
            var all = ctx.File.Funcs.Keys.Concat(ctx.Class?.Methods.Keys ?? Enumerable.Empty<string>()).Concat(Parser.Builtins);
            Error(c.Pos, $"there's no function called {c.Callee}.{Suggest(c.Callee, all)}");
            return LType.Error;
        }
        c.Func = fn;
        Args(c.Args, fn.Params, c.Callee, c.Pos, ctx);
        return fn.Return;
    }

    LType MethodType(MethodCall mc, Ctx ctx)
    {
        var t = Expr(mc.Target, ctx);
        if (t == LType.Error) { foreach (var a in mc.Args) Expr(a, ctx); return t; }
        string who = Describe(mc.Target);
        switch (t)
        {
            case ClassType ct:
            {
                if (!ct.Sym.Methods.TryGetValue(mc.Method, out var m) && ct.Sym.Field(mc.Method) is { Type: FuncType fieldFn } fnField)
                {
                    // bo:onHit(3) where onHit is a field holding a function
                    mc.Kind = MethodKind.CallField;
                    mc.Field = fnField;
                    return CallFunctionValue(fieldFn, mc.Args, mc.Method, mc.MethodPos, ctx);
                }
                if (m == null)
                {
                    foreach (var a in mc.Args) Expr(a, ctx);
                    var hint = ct.Sym.Field(mc.Method) != null ? $" {mc.Method} is a field: {who}.{mc.Method}" : Suggest(mc.Method, ct.Sym.Methods.Keys);
                    Error(mc.MethodPos, $"{ct.Sym.Name} has no method called {mc.Method}.{hint}");
                    return LType.Error;
                }
                mc.Kind = MethodKind.User;
                mc.Func = m;
                Args(mc.Args, m.Params, mc.Method, mc.MethodPos, ctx);
                return m.Return;
            }
            case ArrayType at:
                switch (mc.Method)
                {
                    case "inject":
                        mc.Kind = MethodKind.ArrayInject;
                        Args(mc.Args, new[] { ("item", at.Elem) }, "inject", mc.MethodPos, ctx);
                        return LType.None;
                    case "remove":
                        mc.Kind = MethodKind.ArrayRemove;
                        Args(mc.Args, new[] { ("position", LType.Int) }, "remove", mc.MethodPos, ctx);
                        return LType.None;
                }
                foreach (var a in mc.Args) Expr(a, ctx);
                Error(mc.MethodPos, $"arrays have inject and remove, not {mc.Method}.{(mc.Method is "add" or "push" or "append" ? $" Use {who}:inject(…)." : mc.Method is "length" or "count" or "size" ? $" To count items, use #{who}." : "")}");
                return LType.Error;
            case MapType mapT:
                switch (mc.Method)
                {
                    case "has":
                        mc.Kind = MethodKind.MapHas;
                        Args(mc.Args, new[] { ("key", LType.String) }, "has", mc.MethodPos, ctx);
                        return LType.Truth;
                    case "remove":
                        mc.Kind = MethodKind.MapRemove;
                        Args(mc.Args, new[] { ("key", LType.String) }, "remove", mc.MethodPos, ctx);
                        return LType.None;
                }
                foreach (var a in mc.Args) Expr(a, ctx);
                Error(mc.MethodPos, $"maps have has and remove, not {mc.Method}.{(mc.Method is "inject" or "add" or "set" or "put" ? $" To add or change a key, write {who}[\"key\"] = …;" : "")}");
                return LType.Error;
            case CArrayType ct2:
                switch (mc.Method)
                {
                    case "inject":
                        mc.Kind = MethodKind.CArrayInject;
                        if (mc.Args.Count != 1 || mc.Args[0] is not ArrayLit row)
                        {
                            foreach (var a in mc.Args) Expr(a, ctx);
                            Error(mc.MethodPos, $"inject takes one row, written out: {who}:inject([{string.Join(", ", ct2.Columns.Select(c => c.Name))}]);");
                            return LType.None;
                        }
                        Row(row, ct2, ctx);
                        return LType.None;
                    case "remove":
                        mc.Kind = MethodKind.CArrayRemove;
                        Args(mc.Args, new[] { ("id", LType.Int) }, "remove", mc.MethodPos, ctx);
                        return LType.None;
                    case "has":
                        mc.Kind = MethodKind.CArrayHas;
                        Args(mc.Args, new[] { ("id", LType.Int) }, "has", mc.MethodPos, ctx);
                        return LType.Truth;
                }
                foreach (var a in mc.Args) Expr(a, ctx);
                Error(mc.MethodPos, $"cArrays have inject, remove and has, not {mc.Method}.");
                return LType.Error;
        }
        foreach (var a in mc.Args) Expr(a, ctx);
        Error(mc.MethodPos, t == LType.Nothing ? "nothing has no methods." : $"{who} is {t.WithArticle}, which has no methods.");
        return LType.Error;
    }

    LType NewType(New nw, Ctx ctx)
    {
        if (!ctx.File.Classes.TryGetValue(nw.Class, out var cls))
        {
            foreach (var i in nw.Inits) Expr(i.Value, ctx);
            Error(nw.ClassPos, $"there's no class called {nw.Class}.{Suggest(nw.Class, ctx.File.Classes.Keys)}");
            return LType.Error;
        }
        nw.Sym = cls;
        if (cls.IsObject)
        {
            Error(nw.ClassPos, $"{nw.Class} is an object, so there's only ever one. Use it directly: {nw.Class}.something");
            return LType.Error;
        }
        var seen = new HashSet<string>();
        foreach (var (name, pos, value) in nw.Inits)
        {
            var f = cls.Field(name);
            if (f == null)
            {
                Expr(value, ctx);
                Error(pos, $"{cls.Name} has no field called {name}.{Suggest(name, cls.Fields.Select(x => x.Name))}");
                continue;
            }
            if (!seen.Add(name)) Error(pos, $"{name} is already filled in.");
            nw.InitFields.Add(f);
            Expr(value, ctx, f.Type);
            ExpectFits(value, f.Type, $"{name} is {f.Type.WithArticle}");
        }
        return cls.Type;
    }

    LType ArrayLitType(ArrayLit al, Ctx ctx, LType? expected)
    {
        if (expected is MapType mt0 || (expected == null && al.Keys != null))
        {
            if (al.Keys == null && al.Items.Count > 0)
            {
                foreach (var item in al.Items) Expr(item, ctx);
                Error(al.Items[0].Pos, $"a map's items are a key and a value, like [\"apple\" 3, \"seed\" 1].");
                return expected ?? LType.Error;
            }
            var keys = al.Keys ?? new List<Expr>();
            foreach (var k in keys)
            {
                var kt = Expr(k, ctx, LType.String);
                if (kt != LType.String && kt != LType.Error) Error(k.Pos, $"a map's keys are text, but this is {kt.WithArticle}.{(kt.IsNumber ? " Use .text() to turn it into text." : "")}");
            }
            foreach (var group in keys.OfType<StrLit>().GroupBy(k => k.Value).Where(g => g.Count() > 1))
                Error(group.Skip(1).First().Pos, $"\"{group.Key}\" is already a key in this map.");
            if (expected is MapType mt)
            {
                foreach (var item in al.Items)
                {
                    Expr(item, ctx, mt.Elem);
                    ExpectFits(item, mt.Elem, $"this map holds {Plural(mt.Elem)}");
                }
                return mt;
            }
            var vtypes = al.Items.Select(item => Expr(item, ctx)).ToList();
            if (vtypes.Contains(LType.Error)) return LType.Error;
            if (vtypes.Any(t => t == LType.None || t == LType.Nothing))
            {
                Error(al.Pos, "Lust can't tell what kind of map this is. Put it in a variable with a type first: int map prices = [\"apple\" 3];");
                return LType.Error;
            }
            var velem = vtypes.All(t => t.IsNumber) && vtypes.Contains(LType.Float) ? LType.Float : vtypes[0];
            foreach (var item in al.Items) ExpectFits(item, velem, $"this map holds {Plural(velem)}");
            return MapType.Of(velem);
        }
        if (al.Keys != null)
        {
            foreach (var k in al.Keys) Expr(k, ctx);
            foreach (var item in al.Items) Expr(item, ctx);
            Error(al.Pos, expected is CArrayType ? "a cArray's rows are written in [ ] with commas, like [1, \"sword\", 1]." : $"this is written like a map (key value), but {(expected != null ? $"it should be {expected.WithArticle}" : "an array is wanted")}.");
            return expected ?? LType.Error;
        }
        if (expected is ArrayType at)
        {
            foreach (var item in al.Items)
            {
                Expr(item, ctx, at.Elem);
                ExpectFits(item, at.Elem, $"this array holds {Plural(at.Elem)}");
            }
            return at;
        }
        if (expected is CArrayType ct)
        {
            foreach (var item in al.Items)
            {
                if (item is ArrayLit row) Row(row, ct, ctx);
                else
                {
                    Expr(item, ctx);
                    Error(item.Pos, $"each row is written in [ ], like [1, {string.Join(", ", ct.Columns.Select(c => c.Name))}]");
                }
            }
            return ct;
        }
        if (al.Items.Count == 0)
        {
            Error(al.Pos, "Lust can't tell what kind of array [] is here. Put it in a variable with a type first: int array scores = [];");
            return LType.Error;
        }
        var types = al.Items.Select(item => Expr(item, ctx)).ToList();
        if (types.Contains(LType.Error)) return LType.Error;
        if (types.Any(t => t == LType.None || t == LType.Nothing))
        {
            var bad = al.Items[types.FindIndex(t => t == LType.None || t == LType.Nothing)];
            Error(bad.Pos, bad.Type == LType.None
                ? "this doesn't give anything back, so it can't go in an array."
                : "Lust can't tell what kind of array this is. Put it in a variable with a type first: Dog array dogs = [nothing];");
            return LType.Error;
        }
        // An int and a float together give a float.
        var elem = types.All(t => t.IsNumber) && types.Contains(LType.Float) ? LType.Float : types[0];
        foreach (var item in al.Items) ExpectFits(item, elem, $"this array holds {Plural(elem)}");
        return ArrayType.Of(elem);
    }

    void Row(ArrayLit row, CArrayType table, Ctx ctx)
    {
        row.Type = table.Row;
        int cols = table.Columns.Count;
        var names = string.Join(", ", table.Columns.Select(c => c.Name));
        if (row.Items.Count != cols && row.Items.Count != cols + 1)
        {
            foreach (var item in row.Items) Expr(item, ctx);
            Error(row.Pos, $"a row has {(cols == 1 ? "1 value" : $"{cols} values")}: [{names}], or [id, {names}] to pick its id.");
            return;
        }
        row.RowHasId = row.Items.Count == cols + 1;
        int offset = row.RowHasId ? 1 : 0;
        if (row.RowHasId)
        {
            var idType = Expr(row.Items[0], ctx, LType.Int);
            if (idType != LType.Int && idType != LType.Error) Error(row.Items[0].Pos, $"a row's id is an int, but this is {idType.WithArticle}.");
        }
        for (int i = 0; i < cols; i++)
        {
            var item = row.Items[i + offset];
            Expr(item, ctx, table.Columns[i].Type);
            ExpectFits(item, table.Columns[i].Type, $"the column {table.Columns[i].Name} is {table.Columns[i].Type.WithArticle}");
        }
    }

    // ----- spelling suggestions -----

    static string Suggest(string name, IEnumerable<string> candidates)
    {
        var best = candidates.Distinct()
            .Select(c => (c, d: Distance(name.ToLowerInvariant(), c.ToLowerInvariant())))
            .Where(x => x.d <= Math.Max(1, name.Length / 3) && x.c != name)
            .OrderBy(x => x.d).Select(x => x.c).FirstOrDefault();
        return best == null ? "" : $" Did you mean {best}?";
    }

    static int Distance(string a, string b)
    {
        var d = new int[a.Length + 1, b.Length + 1];
        for (int i = 0; i <= a.Length; i++) d[i, 0] = i;
        for (int j = 0; j <= b.Length; j++) d[0, j] = j;
        for (int i = 1; i <= a.Length; i++)
            for (int j = 1; j <= b.Length; j++)
            {
                d[i, j] = Math.Min(Math.Min(d[i - 1, j] + 1, d[i, j - 1] + 1), d[i - 1, j - 1] + (a[i - 1] == b[j - 1] ? 0 : 1));
                // Two letters swapped (nmae → name) count as one slip.
                if (i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1]) d[i, j] = Math.Min(d[i, j], d[i - 2, j - 2] + 1);
            }
        return d[a.Length, b.Length];
    }
}

static class TextExtensions
{
    public static string WithIndefinite(this string word) => ("aeiouAEIOU".Contains(word[0]) ? "an " : "a ") + word;
}
