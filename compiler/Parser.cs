namespace Lust;

/// Turns tokens into a Program. On a mistake it reports it, skips to the next
/// statement and keeps going, so one typo doesn't hide the rest.
public sealed class Parser
{
    readonly List<Token> toks;
    readonly Diagnostics diags;
    readonly SourceFile file;
    int p;
    bool noSpacedCall; // while reading the first half of `is a b?`, `x (-1)` is two values, not a call

    Parser(SourceFile file, List<Token> toks, Diagnostics diags) { this.file = file; this.toks = toks; this.diags = diags; }

    public static Program Parse(SourceFile file, Diagnostics diags)
    {
        var parser = new Parser(file, Lexer.Lex(file, diags), diags);
        var program = parser.ParseProgram();
        program.BrokenNames.UnionWith(parser.BrokenNames);
        return program;
    }

    // ----- words -----

    /// Lust's own words: never usable as names.
    public static readonly HashSet<string> Reserved = new()
    {
        "class", "object", "has", "method", "do", "give", "new", "nothing",
        "is", "then", "or", "either", "not", "true",
        "loop", "stop", "skip", "in",
        "state", "send", "fetch", "get", "end",
        "int", "float", "string", "truth", "array", "cArray",
    };

    /// Words from other languages, and what to write in Lust instead.
    static readonly Dictionary<string, string> Hints = new()
    {
        ["if"] = "Lust asks questions instead of if: is hp 0? then { }",
        ["else"] = "Lust uses or instead of else: } or { }",
        ["elif"] = "Lust uses or is …? then { } instead of elif.",
        ["while"] = "Lust has one loop: loop (hp > 0) { }",
        ["for"] = "Lust has one loop: loop 3 { } or loop (int n in list) { }",
        ["foreach"] = "Lust has one loop: loop (int n in list) { }",
        ["break"] = "Lust uses stop; to leave a loop.",
        ["continue"] = "Lust uses skip; to jump to the next round of a loop.",
        ["return"] = "Lust uses give to hand back a value.",
        ["bool"] = "Lust calls it truth: truth alive = true;",
        ["boolean"] = "Lust calls it truth: truth alive = true;",
        ["false"] = "Lust has no false. Write not true.",
        ["null"] = "Lust calls it nothing.",
        ["void"] = "Lust has no void. Leave the return type out when nothing comes back.",
        ["var"] = "put the type first, like int lives = 3;",
        ["let"] = "put the type first, like int lives = 3;",
        ["function"] = "Lust makes functions with do.",
        ["func"] = "Lust makes functions with do.",
        ["fn"] = "Lust makes functions with do.",
        ["def"] = "Lust makes functions with do.",
        ["self"] = "inside a method, use a field's name on its own: hp, not self.hp.",
        ["this"] = "inside a method, use a field's name on its own: hp, not this.hp.",
        ["double"] = "Lust calls it float.",
        ["str"] = "Lust calls it string.",
        ["import"] = "Lust brings in other files with fetch and get.",
        ["export"] = "Lust shares things with send.",
        ["elseif"] = "Lust uses or is …? then { } for another question.",
        ["match"] = "Lust asks questions with is …? then { } or is …? then { }.",
        ["switch"] = "Lust asks questions with is …? then { } or is …? then { }.",
    };

    /// Words for parts of Lust that later milestones add.
    static readonly Dictionary<string, string> NotYet = new()
    {
        ["class"] = "classes", ["object"] = "objects", ["has"] = "has blocks", ["method"] = "methods",
        ["new"] = "classes", ["do"] = "functions (do)", ["give"] = "functions (do and give)",
        ["state"] = "state", ["send"] = "sharing between files", ["fetch"] = "sharing between files",
        ["get"] = "sharing between files", ["array"] = "arrays", ["cArray"] = "cArrays", ["in"] = "arrays",
        ["nothing"] = "nothing (it comes with classes)",
    };

    static string NotYetMessage(string feature) =>
        $"this version of Lust doesn't have {feature} yet. That comes in a later milestone.";

    // ----- token helpers -----

    Token Cur => toks[p];
    Token Prev => toks[Math.Max(0, p - 1)];
    Token PeekAt(int n) => toks[Math.Min(p + n, toks.Count - 1)];
    bool At(Tk k) => Cur.Kind == k;
    bool AtWord(string w) => Cur.Is(w);
    Token Next() { var t = Cur; if (t.Kind != Tk.Eof) p++; return t; }

    ParseStop Fail(Pos pos, string message)
    {
        if (!diags.SymbolMistakeOn(pos) && !diags.SymbolMistakeOn(Prev.Pos)) diags.Add(pos, message);
        return new ParseStop();
    }

    /// Where to point when something is missing: the end of the line if the next token is on a later one.
    Pos MissingAt => Cur.Kind == Tk.Eof || Cur.Pos.Line > Prev.Pos.Line ? Prev.End : Cur.Pos;

    Token Expect(Tk k, string message)
    {
        if (At(k)) return Next();
        throw Fail(MissingAt, message);
    }

    void ExpectWord(string w, string message)
    {
        if (AtWord(w)) { Next(); return; }
        throw Fail(Cur.Kind == Tk.Eof ? Prev.End : Cur.Pos, message);
    }

    /// Every statement ends with ;. A missing one is reported where the line ends, and parsing carries on.
    void EndStatement()
    {
        if (At(Tk.Semi)) { Next(); return; }
        diags.Add(Prev.End, "missing ; at the end of this line.");
        if (Cur.Pos.Line == Prev.Pos.Line && !At(Tk.RBrace) && !At(Tk.Eof)) throw new ParseStop();
    }

    /// After a mistake: skip past the rest of the statement. That's the next ; or a whole { } block,
    /// or stopping at a } that belongs to an outer block, or at a Lust word starting a new line.
    void Recover(int startLine)
    {
        while (!At(Tk.Eof))
        {
            if (At(Tk.Semi)) { Next(); return; }
            if (At(Tk.RBrace)) return;
            if (At(Tk.LBrace))
            {
                SkipBlock();
                // `new Dog { … };` ends with a ; after the braces.
                if (At(Tk.Semi)) { Next(); return; }
                // A chain like `} or { … }` belongs to the same broken statement.
                if (!AtWord("or")) return;
                continue;
            }
            if (Cur.Pos.Line != startLine && Cur.Kind == Tk.Ident && (Reserved.Contains(Cur.Text) || Hints.ContainsKey(Cur.Text)) && !Cur.Is("or") && !Cur.Is("then") && !Cur.Is("not") && !Cur.Is("true")) return;
            Next();
        }
    }

    void SkipBlock()
    {
        int depth = 0;
        while (!At(Tk.Eof))
        {
            var t = Next();
            if (t.Kind == Tk.LBrace) depth++;
            else if (t.Kind == Tk.RBrace && --depth == 0) return;
            if (AtWord("end") && PeekAt(1).Kind == Tk.Eof) return;
        }
    }

    /// Names whose declaration had a mistake. The checker stays quiet about them later.
    public readonly HashSet<string> BrokenNames = new();

    // ----- program -----

    Program ParseProgram()
    {
        var stmts = new List<Stmt>();
        bool ended = false;
        while (!At(Tk.Eof))
        {
            if (diags.Full) break;
            if (AtWord("end"))
            {
                Next();
                ended = true;
                if (!At(Tk.Eof)) diags.Add(Cur.Pos, "nothing can come after end. end is the file's last line.");
                break;
            }
            if (At(Tk.RBrace)) { diags.Add(Cur.Pos, "this } doesn't close anything."); Next(); continue; }
            var s = Statement();
            if (s != null) stmts.Add(s);
        }
        if (!ended && !diags.Full)
            diags.Add(toks.Count > 1 ? Prev.End : Cur.Pos, $"{file.Display} never ends. Add end on its last line.");
        return new Program { File = file, Stmts = stmts };
    }

    Stmt? Statement()
    {
        int start = p, line = Cur.Pos.Line;
        try
        {
            return StatementInner();
        }
        catch (ParseStop)
        {
            if (start + 1 < toks.Count && toks[start].Kind == Tk.Ident && toks[start + 1].Kind == Tk.Ident)
            {
                BrokenNames.Add(toks[start + 1].Text);
                if (start + 2 < toks.Count && toks[start + 2].Kind == Tk.Ident) BrokenNames.Add(toks[start + 2].Text);
            }
            if (p == start) Next();
            Recover(line);
            return null;
        }
    }

    Stmt StatementInner()
    {
        var t = Cur;
        if (t.Kind == Tk.LBrace) throw Fail(t.Pos, "a { } block needs a loop or a question in front of it.");
        if (t.Kind == Tk.Semi) throw Fail(t.Pos, "there's a ; here with nothing before it.");
        if (t.Kind != Tk.Ident) throw Fail(t.Pos, $"a line can't start with {Describe(t)}.");

        switch (t.Text)
        {
            case "is": return IsChainStmt();
            case "loop": return LoopStmt();
            case "stop": Next(); EndStatement(); return new Stop { Pos = t.Pos };
            case "skip": Next(); EndStatement(); return new Skip { Pos = t.Pos };
            case "wait":
            {
                Next();
                var e = Expression();
                EndStatement();
                return new Wait { Pos = t.Pos, Seconds = e };
            }
            case "end": throw Fail(t.Pos, "end only goes on the file's last line. Is a } missing above?");
            case "then": throw Fail(t.Pos, "then comes after a question: is hp 0? then { }");
            case "or": throw Fail(t.Pos, "or goes right after the } of a question: } or { }");
        }
        if (LType.FromName(t.Text) != null) return VarDeclStmt();
        if (Hints.TryGetValue(t.Text, out var hint)) throw Fail(t.Pos, hint);
        if (NotYet.TryGetValue(t.Text, out var feature)) throw Fail(t.Pos, NotYetMessage(feature));

        // Two names in a row look like a declaration with a type Lust doesn't have.
        if (PeekAt(1).Kind == Tk.Ident && !Reserved.Contains(PeekAt(1).Text) && PeekAt(2).Kind is Tk.Assign or Tk.Semi)
            throw Fail(t.Pos, $"Lust doesn't have a type called {t.Text}. The types are int, float, string and truth.");

        if (PeekAt(1).Kind == Tk.Assign)
        {
            if (Reserved.Contains(t.Text)) throw Fail(t.Pos, $"{t.Text} is one of Lust's own words; it can't be changed.");
            var target = (Name)Primary();
            Next(); // =
            var value = Expression();
            EndStatement();
            return new Assign { Pos = t.Pos, Target = target, Value = value };
        }

        var expr = Expression();
        if (expr is not Call)
            throw Fail(t.Pos, "this line doesn't do anything. A line can declare, change a variable, call something, ask a question or loop.");
        EndStatement();
        return new ExprStmt { Pos = t.Pos, Expr = expr };
    }

    Stmt VarDeclStmt()
    {
        var typeTok = Next();
        var type = new TypeRef { Pos = typeTok.Pos, Name = typeTok.Text };
        if (AtWord("array")) throw Fail(Cur.Pos, NotYetMessage("arrays"));
        if (At(Tk.LBrack)) throw Fail(Cur.Pos, $"{typeTok.Text}[…] only goes inside a has block. Here, write {typeTok.Text} name = value;");
        if (!At(Tk.Ident)) throw Fail(Cur.Kind == Tk.Eof ? Prev.End : Cur.Pos, $"give the {typeTok.Text} a name, like {typeTok.Text} {Example(typeTok.Text)};");
        var nameTok = Next();
        CheckName(nameTok);
        if (At(Tk.Semi) || At(Tk.Eof))
            throw Fail(nameTok.End, $"give {nameTok.Text} a starting value, like {typeTok.Text} {nameTok.Text} = {Example(typeTok.Text).Split(" = ")[1]};");
        if (!At(Tk.Assign)) throw Fail(Cur.Pos, $"expected = and a starting value after {nameTok.Text}.");
        Next();
        var init = Expression();
        EndStatement();
        return new VarDecl { Pos = typeTok.Pos, TypeRef = type, Id = nameTok.Text, IdPos = nameTok.Pos, Init = init };
    }

    static string Example(string type) => type switch
    {
        "int" => "lives = 3",
        "float" => "speed = 1.5",
        "string" => "name = \"Ada\"",
        "truth" => "alive = true",
        _ => "x = …",
    };

    void CheckName(Token t)
    {
        if (Reserved.Contains(t.Text)) throw Fail(t.Pos, $"{t.Text} is one of Lust's own words, so it can't be a name.");
        if (t.Text is "print" or "wait" or "random") throw Fail(t.Pos, $"{t.Text} is built into Lust, so it can't be a name.");
    }

    // ----- is … ? then { } or … -----

    Stmt IsChainStmt()
    {
        var start = Cur.Pos;
        var branches = new List<(Expr, Block)>();
        Block? otherwise = null;
        while (true)
        {
            ExpectWord("is", "expected is.");
            var check = Check();
            if (!At(Tk.Question))
                throw Fail(Cur.Kind == Tk.Eof ? Prev.End : Cur.Pos, AtWord("then") ? "a question ends with ? before then: is hp 0? then { }" : "a question ends with ?, like is hp 0? then { }");
            Next();
            ExpectWord("then", "after the ? comes then and a block: is hp 0? then { }");
            var body = BlockStmt();
            branches.Add((check, body));
            if (!AtWord("or")) break;
            Next();
            if (AtWord("is")) continue;
            otherwise = BlockStmt();
            break;
        }
        if (At(Tk.Semi)) diags.Add(Cur.Pos, "no ; is needed after a question's }.");
        return new IsChain { Pos = start, Branches = branches, Otherwise = otherwise };
    }

    /// A question's check: parts like `hp 0`, `alive not true` or `hp < 5`, joined by && and ||.
    Expr Check()
    {
        var left = CheckAnd();
        while (At(Tk.OrOr))
        {
            var op = Next();
            var right = CheckAnd();
            left = new Binary { Pos = left.Pos, OpPos = op.Pos, Op = BinOp.Or, Left = left, Right = right };
        }
        return left;
    }

    Expr CheckAnd()
    {
        var left = CheckPart();
        while (At(Tk.AndAnd))
        {
            var op = Next();
            var right = CheckPart();
            left = new Binary { Pos = left.Pos, OpPos = op.Pos, Op = BinOp.And, Left = left, Right = right };
        }
        return left;
    }

    Expr CheckPart()
    {
        noSpacedCall = true;
        Expr left;
        try { left = NotExpr(); } finally { noSpacedCall = false; }
        if (AtWord("not"))
        {
            var notTok = Next();
            if (!StartsValue(Cur)) throw Fail(Cur.Pos, $"after not, say what it isn't: is {Snippet(left)} not 0?");
            var right = NotExpr();
            return new Same { Pos = left.Pos, Left = left, Right = right, Negated = true };
        }
        if (StartsValue(Cur))
        {
            var right = NotExpr();
            return new Same { Pos = left.Pos, Left = left, Right = right };
        }
        return left;
    }

    /// Could this token start a value (the second half of `is a b?`)?
    static bool StartsValue(Token t) => t.Kind switch
    {
        Tk.Int or Tk.Float or Tk.String or Tk.LParen or Tk.Hash or Tk.LBrack => true,
        Tk.Ident => !Reserved.Contains(t.Text) || t.Text is "true" or "nothing" or "either",
        _ => false,
    };

    static string Snippet(Expr e) => e is Name n ? n.Id : "x";

    // ----- loop -----

    Stmt LoopStmt()
    {
        var start = Next().Pos;
        if (At(Tk.LBrace)) return new Loop { Pos = start, Kind = LoopKind.Forever, Body = BlockStmt() };
        if (At(Tk.LParen))
        {
            // loop (int n in list) is for later; spot it so the message is clear.
            if (PeekAt(1).Kind == Tk.Ident && (LType.FromName(PeekAt(1).Text) != null || PeekAt(2).Is("in") || PeekAt(3).Is("in")))
                throw Fail(PeekAt(1).Pos, NotYetMessage("arrays"));
            Next();
            var cond = Expression();
            Expect(Tk.RParen, "expected ) to close the loop's check.");
            return new Loop { Pos = start, Kind = LoopKind.While, Expr = cond, Body = BlockStmt() };
        }
        var count = Expression();
        if (!At(Tk.LBrace)) throw Fail(Cur.Pos, "expected { after the number of times to loop.");
        return new Loop { Pos = start, Kind = LoopKind.Times, Expr = count, Body = BlockStmt() };
    }

    Block BlockStmt()
    {
        var open = Expect(Tk.LBrace, "expected { to start a block.");
        var stmts = new List<Stmt>();
        while (!At(Tk.RBrace))
        {
            if (At(Tk.Eof) || diags.Full) throw Fail(open.Pos, "this { is never closed. Add a } where the block ends.");
            if (AtWord("end") && PeekAt(1).Kind == Tk.Eof) throw Fail(open.Pos, "this { is never closed. Add a } where the block ends.");
            var s = Statement();
            if (s != null) stmts.Add(s);
        }
        var close = Next();
        return new Block { Pos = open.Pos, Stmts = stmts, Close = close.Pos };
    }

    // ----- expressions -----

    public Expr Expression()
    {
        if (AtWord("either")) return EitherExpr();
        return OrExpr();
    }

    Expr EitherExpr()
    {
        var start = Next().Pos;
        var check = Check();
        if (!At(Tk.Question)) throw Fail(Cur.Pos, "either needs a question ending with ?: either hp > 0? \"up\" or \"down\"");
        Next();
        var yes = Expression();
        if (!AtWord("or")) throw Fail(Cur.Kind == Tk.Eof ? Prev.End : Cur.Pos, "either needs an or: either hp > 0? \"up\" or \"down\"");
        Next();
        var no = Expression();
        return new Either { Pos = start, Check = check, Yes = yes, No = no };
    }

    Expr OrExpr()
    {
        var left = AndExpr();
        while (At(Tk.OrOr))
        {
            var op = Next();
            left = new Binary { Pos = left.Pos, OpPos = op.Pos, Op = BinOp.Or, Left = left, Right = AndExpr() };
        }
        return left;
    }

    Expr AndExpr()
    {
        var left = NotExpr();
        while (At(Tk.AndAnd))
        {
            var op = Next();
            left = new Binary { Pos = left.Pos, OpPos = op.Pos, Op = BinOp.And, Left = left, Right = NotExpr() };
        }
        return left;
    }

    Expr NotExpr()
    {
        if (AtWord("not"))
        {
            var t = Next();
            return new Unary { Pos = t.Pos, Op = UnOp.Not, Operand = NotExpr() };
        }
        return Compare();
    }

    Expr Compare()
    {
        var left = Sum();
        BinOp? op = Cur.Kind switch { Tk.Lt => BinOp.Lt, Tk.Gt => BinOp.Gt, Tk.Le => BinOp.Le, Tk.Ge => BinOp.Ge, _ => null };
        if (op == null) return left;
        var opTok = Next();
        var right = Sum();
        if (Cur.Kind is Tk.Lt or Tk.Gt or Tk.Le or Tk.Ge)
            throw Fail(Cur.Pos, "one comparison at a time. Join them with &&: is a < b && b < c?");
        return new Binary { Pos = left.Pos, OpPos = opTok.Pos, Op = op.Value, Left = left, Right = right };
    }

    Expr Sum()
    {
        var left = Product();
        while (At(Tk.Plus) || At(Tk.Minus))
        {
            var op = Next();
            left = new Binary { Pos = left.Pos, OpPos = op.Pos, Op = op.Kind == Tk.Plus ? BinOp.Add : BinOp.Sub, Left = left, Right = Product() };
        }
        return left;
    }

    Expr Product()
    {
        var left = Negate();
        while (At(Tk.Star) || At(Tk.Slash) || At(Tk.Percent))
        {
            var op = Next();
            var kind = op.Kind switch { Tk.Star => BinOp.Mul, Tk.Slash => BinOp.Div, _ => BinOp.Mod };
            left = new Binary { Pos = left.Pos, OpPos = op.Pos, Op = kind, Left = left, Right = Negate() };
        }
        return left;
    }

    Expr Negate()
    {
        if (At(Tk.Minus))
        {
            var t = Next();
            var operand = Negate();
            // -3 is just the number -3.
            if (operand is IntLit i) return new IntLit { Pos = t.Pos, Value = -i.Value };
            if (operand is FloatLit f) return new FloatLit { Pos = t.Pos, Value = -f.Value, Text = "-" + f.Text };
            return new Unary { Pos = t.Pos, Op = UnOp.Neg, Operand = operand };
        }
        return Postfix();
    }

    Expr Postfix()
    {
        var e = Primary();
        while (true)
        {
            if (At(Tk.Dot))
            {
                Next();
                if (!At(Tk.Ident)) throw Fail(Cur.Pos, "expected a name after the dot, like x.text().");
                var m = Next();
                if (!At(Tk.LParen))
                    throw Fail(m.Pos, m.Text is "text" or "int" or "float" ? $"add () to call it: .{m.Text}()" : $"{Snippet(e)} has no field called {m.Text}. Fields come with classes, in a later milestone.");
                Next();
                if (!At(Tk.RParen)) throw Fail(Cur.Pos, $".{m.Text}() doesn't take anything between its ().");
                Next();
                e = new Conversion { Pos = e.Pos, Target = e, Method = m.Text, MethodPos = m.Pos };
                continue;
            }
            if (At(Tk.Colon)) throw Fail(Cur.Pos, NotYetMessage("methods"));
            if (At(Tk.LBrack) && !Cur.SpaceBefore) throw Fail(Cur.Pos, NotYetMessage("arrays"));
            return e;
        }
    }

    Expr Primary()
    {
        var t = Cur;
        switch (t.Kind)
        {
            case Tk.Int: Next(); return new IntLit { Pos = t.Pos, Value = t.IntValue };
            case Tk.Float: Next(); return new FloatLit { Pos = t.Pos, Value = t.FloatValue, Text = t.Text };
            case Tk.String: Next(); return new StrLit { Pos = t.Pos, Value = t.StringValue };
            case Tk.LParen:
            {
                Next();
                var e = Expression();
                Expect(Tk.RParen, "expected ) here.");
                return e;
            }
            case Tk.Hash: throw Fail(t.Pos, NotYetMessage("arrays"));
            case Tk.LBrack: throw Fail(t.Pos, NotYetMessage("arrays"));
            case Tk.Eof: throw Fail(Prev.End, "the file stops in the middle of this line.");
            case Tk.Ident: break;
            default: throw Fail(t.Pos, $"expected a value here, but found {Describe(t)}.");
        }

        if (t.Text == "true") { Next(); return new TruthLit { Pos = t.Pos }; }
        if (t.Text == "nothing") throw Fail(t.Pos, NotYetMessage("nothing (it comes with classes)"));
        if (t.Text == "either") return EitherExpr();
        if (Hints.TryGetValue(t.Text, out var hint)) throw Fail(t.Pos, hint);
        if (NotYet.TryGetValue(t.Text, out var feature)) throw Fail(t.Pos, NotYetMessage(feature));
        if (Reserved.Contains(t.Text)) throw Fail(t.Pos, $"expected a value here, but found the word {t.Text}.");

        Next();
        bool call = At(Tk.LParen) && (!Cur.SpaceBefore || !noSpacedCall);
        if (!call) return new Name { Pos = t.Pos, Id = t.Text };

        Next(); // (
        var args = new List<Expr>();
        bool saved = noSpacedCall;
        noSpacedCall = false;
        try
        {
            if (!At(Tk.RParen))
            {
                args.Add(Expression());
                while (At(Tk.Comma)) { Next(); args.Add(Expression()); }
            }
            Expect(Tk.RParen, $"expected ) to close {t.Text}(.");
        }
        finally { noSpacedCall = saved; }
        return new Call { Pos = t.Pos, Callee = t.Text, Args = args };
    }

    static string Describe(Token t) => t.Kind switch
    {
        Tk.Ident => $"the word {t.Text}",
        Tk.Int or Tk.Float => $"the number {t.Text}",
        Tk.String => "some text",
        Tk.Eof => "the end of the file",
        _ => t.Text,
    };
}
