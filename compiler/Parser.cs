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
        "is", "then", "or", "either", "not", "true", "like",
        "loop", "stop", "skip", "in",
        "state", "send", "fetch", "get", "end",
        "int", "float", "string", "truth", "array", "cArray", "map",
        "save", "load", "saved",
    };

    public static readonly HashSet<string> Builtins = new() { "print", "wait", "random", "ask", "readText", "writeText" };

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
        ["enum"] = "Lust calls it state: state = [idle, busy];",
        ["extends"] = "Lust uses like: class Wizard like Player { }",
        ["inherits"] = "Lust uses like: class Wizard like Player { }",
        ["static"] = "for one ready-made thing, use object: object Game { }",
        ["struct"] = "Lust has class (a blueprint) and object (one ready-made thing).",
    };

    /// Hints that only make sense where a value goes (elsewhere these words can be names, like a function called double).
    static readonly HashSet<string> ValueHints = new() { "false", "null", "self", "this" };
    /// Hints about types and declarations: skipped when the word is being called, as in double(3).
    static readonly HashSet<string> TypeHints = new() { "bool", "boolean", "double", "str", "void", "var", "let", "struct", "static", "enum", "import", "export", "extends", "inherits" };

    /// Words from other languages that are followed by ( in their own language: if (…), while (…).
    static readonly HashSet<string> ParenHints = new() { "if", "while", "for", "foreach", "switch", "match", "elif", "elseif" };

    /// At the start of a line: is this word being used the other language's way, rather than as a name?
    bool HintApplies(Token t)
    {
        if (!Hints.ContainsKey(t.Text)) return false;
        var next = PeekAt(1).Kind;
        if (next == Tk.Semi) return true;   // break; continue; return;
        if (next is Tk.Assign or Tk.Dot or Tk.Colon or Tk.LBrack) return t.Text is "self" or "this" && next == Tk.Dot;
        if (next == Tk.LParen) return ParenHints.Contains(t.Text);
        return true;
    }

    // ----- token helpers -----

    Token Cur => toks[p];
    Token Prev => toks[Math.Max(0, p - 1)];
    Token PeekAt(int n) => toks[Math.Min(p + n, toks.Count - 1)];
    bool At(Tk k) => Cur.Kind == k;
    bool AtWord(string w) => Cur.Is(w);
    Token Next() { var t = Cur; if (t.Kind != Tk.Eof) p++; return t; }

    ParseStop Fail(Pos pos, string message)
    {
        // A bad symbol earlier on the same line usually causes this one; then it's left out.
        bool knockOn = diags.SymbolMistakeOn(pos) || (Prev.Pos.Line == pos.Line && diags.SymbolMistakeOn(Prev.Pos));
        if (!knockOn) diags.Add(pos, message);
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
        throw Fail(MissingAt, message);
    }

    Token ExpectName(string what)
    {
        if (!At(Tk.Ident)) throw Fail(MissingAt, $"expected {what} here.");
        var t = Next();
        CheckName(t);
        return t;
    }

    /// Every statement ends with ;. A missing one is reported where the line ends, and parsing carries on.
    void EndStatement()
    {
        if (At(Tk.Semi)) { Next(); return; }
        if (diags.SymbolMistakeOn(Prev.Pos)) { if (Cur.Pos.Line == Prev.Pos.Line && !At(Tk.RBrace) && !At(Tk.Eof)) throw new ParseStop(); return; }
        if (Cur.Pos.Line == Prev.Pos.Line && StartsValue(Cur) && !AtWord("is") && !(Cur.Kind == Tk.Ident && PeekAt(1).Kind is Tk.LParen or Tk.Assign or Tk.Colon or Tk.Dot or Tk.Ident))
        {
            // truth same = hp 3;  Two values side by side only compare inside a question.
            diags.Add(Cur.Pos, "two values side by side only compare inside a question. Outside one, use either: truth same = either hp 3? true or not true;");
            throw new ParseStop();
        }
        if (AtWord("and") || (AtWord("or") && Cur.Pos.Line == Prev.Pos.Line))
        {
            diags.Add(Cur.Pos, AtWord("and") ? "Lust joins checks with &&, not and." : "inside a check, \"or\" is written ||.");
            throw new ParseStop();
        }
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
            if (Cur.Pos.Line != startLine && Cur.Kind == Tk.Ident && (Reserved.Contains(Cur.Text) || Hints.ContainsKey(Cur.Text)) && !Cur.Is("or") && !Cur.Is("then") && !Cur.Is("not") && !Cur.Is("true") && !Cur.Is("nothing") && !Cur.Is("new") && !Cur.Is("in") && !Cur.Is("like")) return;
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

    Program program = null!;

    Program ParseProgram()
    {
        program = new Program { File = file };
        bool ended = false, pastFetches = false;
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
            if ((AtWord("fetch") || AtWord("get")) && pastFetches)
                diags.Add(Cur.Pos, $"{Cur.Text} lines go at the top of the file, before anything else.");
            if (!AtWord("fetch") && !AtWord("get")) pastFetches = true;
            TopLevelItem();
        }
        if (!ended && !diags.Full)
            diags.Add(toks.Count > 1 ? Prev.End : Cur.Pos, $"{file.Display} never ends. Add end on its last line.", always: true);
        return program;
    }

    void TopLevelItem()
    {
        int start = p, line = Cur.Pos.Line;
        try
        {
            switch (Cur.Text)
            {
                case "class" or "object" when Cur.Kind == Tk.Ident: program.Classes.Add(ClassDeclaration()); return;
                case "do" when Cur.Kind == Tk.Ident && !DoIsType(): program.Funcs.Add(FunctionDeclaration(isMethod: false)); return;
                case "send" when Cur.Kind == Tk.Ident: program.Sends.Add(SendDeclaration()); return;
                case "fetch" or "get" when Cur.Kind == Tk.Ident: program.Fetches.Add(FetchDeclaration()); return;
                case "state" when Cur.Kind == Tk.Ident && IsStateDeclaration(): program.States.Add(StateDeclaration()); return;
                case "save" when Cur.Kind == Tk.Ident && PeekAt(1).Kind == Tk.Ident && PeekAt(2).Kind == Tk.Assign: program.Saves.Add(SaveDeclaration()); return;
                case "method" when Cur.Kind == Tk.Ident: throw Fail(Cur.Pos, "methods go inside a class or object. For a function on its own, use do.");
                case "has" when Cur.Kind == Tk.Ident: throw Fail(Cur.Pos, "a has block goes inside a class or object.");
                case "give" when Cur.Kind == Tk.Ident: throw Fail(Cur.Pos, "give only works inside a function or method.");
            }
        }
        catch (ParseStop)
        {
            if (p == start) Next();
            Recover(line);
            return;
        }
        var s = Statement();
        if (s != null) program.Stmts.Add(s);
    }

    // ----- declarations -----

    bool IsStateDeclaration() =>
        (PeekAt(1).Kind == Tk.Assign && PeekAt(2).Kind == Tk.LBrack) ||
        (PeekAt(1).Kind == Tk.Ident && PeekAt(2).Kind == Tk.Assign && PeekAt(3).Kind == Tk.LBrack);

    StateDecl StateDeclaration()
    {
        var start = Next().Pos; // state
        string name = "state";
        if (At(Tk.Ident)) name = ExpectName("a name for the state").Text;
        Expect(Tk.Assign, "expected = and the options in [ ].");
        Expect(Tk.LBrack, "expected [ and the options, like state = [idle, busy];");
        var decl = new StateDecl { Pos = start, Name = name };
        StateOptions(decl, "");
        if (decl.Options.Count == 0) diags.Add(start, "a state needs at least one option.");
        EndStatement();
        return decl;
    }

    /// The options inside [ ]. An option can hold its own options: fighting [attacking, blocking].
    /// They're stored by their full name, like fighting.blocking, parents before children.
    void StateOptions(StateDecl decl, string prefix)
    {
        while (!At(Tk.RBrack))
        {
            if (!At(Tk.Ident)) throw Fail(MissingAt, "a state's options are names, like [idle, walking, fighting].");
            var opt = Next();
            if (Reserved.Contains(opt.Text)) throw Fail(opt.Pos, $"{opt.Text} is one of Lust's own words, so it can't be an option.");
            var full = prefix + opt.Text;
            decl.Options.Add((full, opt.Pos));
            if (At(Tk.LBrack))
            {
                Next();
                if (At(Tk.RBrack)) { diags.Add(Cur.Pos, $"{opt.Text} [ ] is empty. Leave out the [ ] if {opt.Text} has no options inside it."); Next(); }
                else StateOptions(decl, full + ".");
            }
            if (At(Tk.Comma)) { Next(); continue; }
            if (!At(Tk.RBrack)) throw Fail(MissingAt, At(Tk.Ident) ? "expected , between options." : "expected , or ] here.");
        }
        Next();
    }

    /// An option written after is: idle, or fighting.blocking for one inside another.
    (string Text, Pos Pos) OptionPath(string what)
    {
        if (!At(Tk.Ident)) throw Fail(MissingAt, what);
        var first = Next();
        var text = first.Text;
        while (At(Tk.Dot) && PeekAt(1).Kind == Tk.Ident) { Next(); text += "." + Next().Text; }
        return (text, first.Pos);
    }

    SaveDecl SaveDeclaration()
    {
        var start = Next().Pos; // save
        var name = ExpectName("a name for the save");
        Next(); // =
        Expect(Tk.LBrack, $"list what the save holds in [ ], like save {name.Text} = [Farm, player];");
        var decl = new SaveDecl { Pos = start, Name = name.Text, NamePos = name.Pos };
        while (!At(Tk.RBrack))
        {
            if (!At(Tk.Ident)) throw Fail(MissingAt, $"a save holds objects and variables, by name: save {name.Text} = [Farm, player];");
            var t = Next();
            decl.Items.Add((t.Text, t.Pos));
            if (At(Tk.Comma)) { Next(); continue; }
            if (At(Tk.Dot)) throw Fail(Cur.Pos, $"a save holds whole objects and variables. Put {t.Text} itself in the list.");
            if (!At(Tk.RBrack)) throw Fail(MissingAt, "expected , or ] here.");
        }
        Next();
        if (decl.Items.Count == 0) diags.Add(start, $"the save {name.Text} holds nothing. List what goes in it: save {name.Text} = [Farm, player];");
        EndStatement();
        return decl;
    }

    SendDecl SendDeclaration()
    {
        var decl = new SendDecl { Pos = Next().Pos };
        do
        {
            if (At(Tk.Comma)) Next();
            if (!At(Tk.Ident)) throw Fail(MissingAt, "say what to send, like send Player;");
            var t = Next();
            decl.Names.Add((t.Text, t.Pos));
        } while (At(Tk.Comma));
        EndStatement();
        return decl;
    }

    FetchDecl FetchDeclaration()
    {
        var word = Next();
        bool isGet = word.Text == "get";
        if (!isGet && AtWord("cs") && PeekAt(1).Kind == Tk.Colon) return CsFetch(word);
        // A path: dice, shared/dice, ../game/dice
        var pathStart = Cur.Pos;
        string path = "";
        while (At(Tk.Dot) && PeekAt(1).Kind == Tk.Dot && PeekAt(2).Kind == Tk.Slash) { Next(); Next(); Next(); path += "../"; }
        if (At(Tk.Dot) && PeekAt(1).Kind == Tk.Slash) throw Fail(Cur.Pos, $"a file in the same folder needs no ./ in front: {word.Text} {(PeekAt(2).Kind == Tk.Ident ? PeekAt(2).Text : "dice")};");
        if (At(Tk.Dot)) throw Fail(Cur.Pos, $"to go up a folder, write ../ like {word.Text} ../shared/dice;");
        if (!At(Tk.Ident)) throw Fail(MissingAt, $"say which file to {word.Text} from, like {word.Text} dice:roll;");
        var module = Next();
        path += module.Text;
        while (At(Tk.Slash))
        {
            Next();
            if (!At(Tk.Ident)) throw Fail(MissingAt, $"a folder path goes folder/file, like {word.Text} shared/dice;");
            module = Next();
            path += "/" + module.Text;
        }
        if (At(Tk.Dot) && PeekAt(1).Is("lust")) throw Fail(Cur.Pos, $"leave out .lust: {word.Text} {path};");
        List<(string, Pos)>? names = null;
        if (At(Tk.Colon))
        {
            Next();
            names = new();
            if (At(Tk.LBrack))
            {
                Next();
                while (!At(Tk.RBrack))
                {
                    if (!At(Tk.Ident)) throw Fail(MissingAt, "expected a name here.");
                    var t = Next();
                    names.Add((t.Text, t.Pos));
                    if (At(Tk.Comma)) { Next(); continue; }
                    if (!At(Tk.RBrack)) throw Fail(MissingAt, "expected , or ] here.");
                }
                Next();
            }
            else
            {
                if (!At(Tk.Ident)) throw Fail(MissingAt, $"say what to {word.Text}, like {word.Text} {module.Text}:{(isGet ? "roll" : "Player")};");
                var t = Next();
                names.Add((t.Text, t.Pos));
                if (At(Tk.LBrack)) throw Fail(Cur.Pos, "fetching single methods isn't in Lust yet. Fetch the whole class.");
            }
        }
        else if (isGet) throw Fail(MissingAt, $"get brings in one function, like get {module.Text}:roll; To bring in everything, use fetch {module.Text};");
        EndStatement();
        return new FetchDecl { Pos = word.Pos, Module = module.Text, Path = path, ModulePos = path.Contains('/') ? pathStart : module.Pos, Names = names, IsGet = isGet };
    }

    /// fetch cs:System.Math;   fetch cs:Raylib_cs.Raylib from "Raylib-cs" 6.1.1;
    FetchDecl CsFetch(Token word)
    {
        Next(); // cs
        Next(); // :
        if (!At(Tk.Ident)) throw Fail(MissingAt, "say which C# type, with its namespace: fetch cs:System.Math;");
        var start = Cur.Pos;
        var name = Next().Text;
        while (At(Tk.Dot) && PeekAt(1).Kind == Tk.Ident) { Next(); name += "." + Next().Text; }
        if (At(Tk.LBrack)) throw Fail(Cur.Pos, "fetch one C# type per line: fetch cs:System.Math;");
        string? package = null, version = null;
        if (AtWord("from"))
        {
            Next();
            if (!At(Tk.String)) throw Fail(MissingAt, "after from comes the NuGet package's name in quotes, and its version: from \"Raylib-cs\" 6.1.1");
            package = Next().StringValue;
            // The version is written plainly (6.1.1), so it's put back together from the pieces.
            var v = new System.Text.StringBuilder();
            while (!At(Tk.Semi) && !At(Tk.Eof) && Cur.Pos.Line == Prev.Pos.Line && (v.Length == 0 || !Cur.SpaceBefore))
                v.Append(Next().Text);
            if (v.Length == 0) throw Fail(MissingAt, $"say which version of {package} to use, like from \"{package}\" 1.0.0");
            version = v.ToString().Trim('"');
        }
        EndStatement();
        return new FetchDecl { Pos = word.Pos, IsCs = true, CsType = name, ModulePos = start, Module = name[(name.LastIndexOf('.') + 1)..], Path = "cs", Package = package, Version = version };
    }

    ClassDecl ClassDeclaration()
    {
        var word = Next();
        bool isObject = word.Text == "object";
        var nameTok = ExpectName($"a name for the {word.Text}, like {word.Text} {(isObject ? "Game" : "Player")} {{ }}");
        string? parent = null;
        Pos parentPos = default;
        if (AtWord("like"))
        {
            Next();
            if (!At(Tk.Ident)) throw Fail(MissingAt, $"say what {nameTok.Text} is like: {word.Text} {nameTok.Text} like Player {{ }}");
            var pt = Next();
            parent = pt.Text;
            parentPos = pt.Pos;
        }
        var decl = new ClassDecl { Pos = word.Pos, Name = nameTok.Text, IsObject = isObject, Parent = parent, ParentPos = parentPos };
        var open = Expect(Tk.LBrace, $"expected {{ to start the {word.Text}.");
        bool seenHas = false, seenMethod = false;
        while (!At(Tk.RBrace))
        {
            if (At(Tk.Eof) || diags.Full || (AtWord("end") && PeekAt(1).Kind == Tk.Eof))
                throw Fail(open.Pos, "this { is never closed. Add a } where the class ends.");
            int start = p, line = Cur.Pos.Line;
            try
            {
                if (AtWord("has"))
                {
                    if (seenHas) diags.Add(Cur.Pos, "a class has one has block. Put all its fields in it.");
                    else if (seenMethod) diags.Add(Cur.Pos, "the has block goes first, above the methods.");
                    seenHas = true;
                    HasBlock(decl);
                }
                else if (AtWord("method"))
                {
                    seenMethod = true;
                    decl.Methods.Add(FunctionDeclaration(isMethod: true));
                }
                else if (AtWord("do")) throw Fail(Cur.Pos, "inside a class, functions are methods: method name() { }");
                else if (LooksLikeDeclaration() || (Cur.Kind == Tk.Ident && PeekAt(1).Kind == Tk.LBrack))
                    throw Fail(Cur.Pos, "fields go inside the has block: has { int[hp 20]; }");
                else throw Fail(Cur.Pos, $"a {word.Text} holds a has block and methods, nothing else.");
            }
            catch (ParseStop)
            {
                if (p == start) Next();
                Recover(line);
            }
        }
        Next();
        return decl;
    }

    void HasBlock(ClassDecl decl)
    {
        Next(); // has
        var open = Expect(Tk.LBrace, "expected { after has.");
        while (!At(Tk.RBrace))
        {
            if (At(Tk.Eof) || diags.Full || (AtWord("end") && PeekAt(1).Kind == Tk.Eof))
                throw Fail(open.Pos, "this { is never closed. Add a } where the has block ends.");
            int start = p, line = Cur.Pos.Line;
            try { HasEntry(decl); }
            catch (ParseStop)
            {
                if (p == start) Next();
                Recover(line);
            }
        }
        Next();
    }

    void HasEntry(ClassDecl decl)
    {
        var start = Cur;
        // A state the class is in: `state is idle;`, `season is spring;`, `farm.season is spring;`
        if (At(Tk.Ident) && (PeekAt(1).Is("is") || (PeekAt(1).Kind == Tk.Dot && PeekAt(2).Kind == Tk.Ident && PeekAt(3).Is("is"))))
        {
            var first = Next();
            string? module = null;
            string name = first.Text;
            if (At(Tk.Dot)) { Next(); module = first.Text; name = Next().Text; }
            Next(); // is
            var opt = OptionPath($"say which option it starts in: {name} is idle;");
            decl.Fields.Add(new FieldDecl
            {
                Pos = start.Pos, IsState = true, Name = name,
                Type = new TypeRef { Pos = start.Pos, Kind = TypeKind.Named, Name = name, Module = module },
                Default = new Name { Pos = opt.Pos, Id = opt.Text },
            });
            EndStatement();
            return;
        }
        if (!At(Tk.Ident)) throw Fail(Cur.Pos, "a has block holds fields, like int[hp 20, gold 0];");
        if (Hints.TryGetValue(Cur.Text, out var hint)) throw Fail(Cur.Pos, hint);
        var type = Type();
        if (At(Tk.LBrack))
        {
            // int[hp 20, gold 0,];
            Next();
            while (!At(Tk.RBrack))
            {
                var nameTok = ExpectName("a field name, like hp 20");
                if (At(Tk.Assign)) throw Fail(Cur.Pos, $"no = in a has block. Write the name and then its starting value: {nameTok.Text} {ExampleValue(type)}");
                if (At(Tk.Comma) || At(Tk.RBrack)) throw Fail(MissingAt, $"give {nameTok.Text} a starting value: {nameTok.Text} {ExampleValue(type)}");
                var value = Expression();
                decl.Fields.Add(new FieldDecl { Pos = nameTok.Pos, Type = type, Name = nameTok.Text, Default = value });
                if (At(Tk.Comma)) { Next(); continue; }
                if (!At(Tk.RBrack)) throw Fail(MissingAt, "expected , or ] here.");
            }
            Next();
            EndStatement();
            return;
        }
        // string array bag [];   cArray(string itemName, int amount) loot [];
        var fieldName = ExpectName("a field name");
        if (At(Tk.Assign)) throw Fail(Cur.Pos, $"no = in a has block. Write the name and then its starting value: {fieldName.Text} {ExampleValue(type)}");
        if (At(Tk.Semi)) throw Fail(MissingAt, $"give {fieldName.Text} a starting value: {type} {fieldName.Text} {ExampleValue(type)};");
        var dflt = Expression();
        decl.Fields.Add(new FieldDecl { Pos = fieldName.Pos, Type = type, Name = fieldName.Text, Default = dflt });
        EndStatement();
    }

    static string ExampleValue(TypeRef t) => t.Kind != TypeKind.Named ? "[]" : t.Name switch
    {
        "int" => "0", "float" => "1.5", "string" => "\"text\"", "truth" => "true", _ => "nothing",
    };

    FuncDecl FunctionDeclaration(bool isMethod)
    {
        var word = Next();
        TypeRef? returns = null;
        if (AtWord("do") || !(At(Tk.Ident) && PeekAt(1).Kind == Tk.LParen))
        {
            if (!At(Tk.Ident)) throw Fail(MissingAt, $"expected a name, like {word.Text} greet() {{ }}");
            if (AtWord("void")) throw Fail(Cur.Pos, Hints["void"]);
            returns = Type();
        }
        if (!At(Tk.Ident)) throw Fail(MissingAt, $"expected a name, like {word.Text} {(returns != null ? returns + " " : "")}greet() {{ }}");
        var nameTok = Next();
        CheckName(nameTok);
        var decl = new FuncDecl { Pos = word.Pos, Name = nameTok.Text, NamePos = nameTok.Pos, Returns = returns, IsMethod = isMethod };
        Expect(Tk.LParen, $"expected ( after {nameTok.Text}, even when it takes nothing: {nameTok.Text}()");
        while (!At(Tk.RParen))
        {
            if (!At(Tk.Ident)) throw Fail(MissingAt, "expected a parameter, like (int amount)");
            if (Cur.Kind == Tk.Ident && (PeekAt(1).Kind is Tk.Comma or Tk.RParen))
                throw Fail(Cur.Pos, $"say what type {Cur.Text} is: (int {Cur.Text})");
            var ptype = Type();
            var pname = ExpectName("a parameter name");
            decl.Params.Add(new ParamDecl(ptype, pname.Text, pname.Pos));
            if (At(Tk.Comma)) { Next(); continue; }
            if (!At(Tk.RParen)) throw Fail(MissingAt, "expected , or ) here.");
        }
        Next();
        decl.Body = BlockStmt();
        return decl;
    }

    // ----- types -----

    /// Could the tokens here start a variable declaration, like `int x =` or `Player bo =`?
    bool LooksLikeDeclaration()
    {
        if (Cur.Kind != Tk.Ident) return false;
        var t = Cur.Text;
        if (LType.Primitive(t) != null || t == "cArray") return true;
        if (t == "do") return DoIsType();
        if (PeekAt(1).Is("array") || PeekAt(1).Is("map")) return true;
        if (t == "state") return PeekAt(1).Kind == Tk.Ident && !PeekAt(1).Is("is");
        if (Reserved.Contains(t)) return false;
        if (Hints.ContainsKey(t) && !(PeekAt(1).Kind == Tk.Ident && PeekAt(2).Kind is Tk.Assign or Tk.Semi)) return false;
        if (PeekAt(1).Kind == Tk.Ident && !Reserved.Contains(PeekAt(1).Text) && PeekAt(2).Kind is Tk.Assign or Tk.Semi) return true;
        if (PeekAt(1).Kind == Tk.Dot && PeekAt(2).Kind == Tk.Ident && PeekAt(3).Kind == Tk.Ident && !Reserved.Contains(PeekAt(3).Text) && PeekAt(4).Kind == Tk.Assign) return true;
        if (PeekAt(1).Kind == Tk.Dot && PeekAt(2).Kind == Tk.Ident && (PeekAt(3).Is("array") || PeekAt(3).Is("map"))) return true;
        return false;
    }

    /// After do: a function type (do int(int) f = …;) rather than a function (do int double(int n) { })?
    /// A function's ( ) is followed by its { body }.
    bool DoIsType()
    {
        int i = p + 1;
        while (i < toks.Count && toks[i].Kind != Tk.LParen && toks[i].Kind is not (Tk.Semi or Tk.LBrace or Tk.Eof)) i++;
        if (i >= toks.Count || toks[i].Kind != Tk.LParen) return false;
        int depth = 0;
        for (; i < toks.Count; i++)
        {
            if (toks[i].Kind == Tk.LParen) depth++;
            else if (toks[i].Kind == Tk.RParen && --depth == 0) break;
            else if (toks[i].Kind is Tk.Semi or Tk.LBrace or Tk.Eof) return false;
        }
        return i + 1 < toks.Count && toks[i + 1].Kind != Tk.LBrace;
    }

    TypeRef Type()
    {
        var t = Cur;
        TypeRef type;
        if (AtWord("do"))
        {
            // do(int, string) takes an int and a string; do int(int) also gives back an int.
            Next();
            TypeRef? ret = null;
            if (!At(Tk.LParen)) ret = Type();
            Expect(Tk.LParen, "a function type lists what it takes in ( ), like do(int) or do int(int, int)");
            var prms = new List<ParamDecl>();
            while (!At(Tk.RParen))
            {
                var pt = Type();
                // A name is allowed, to say what it's for: do(int amount)
                if (At(Tk.Ident) && PeekAt(1).Kind is Tk.Comma or Tk.RParen) Next();
                prms.Add(new ParamDecl(pt, "", pt.Pos));
                if (At(Tk.Comma)) { Next(); continue; }
                if (!At(Tk.RParen)) throw Fail(MissingAt, "expected , or ) here.");
            }
            Next();
            type = new TypeRef { Pos = t.Pos, Kind = TypeKind.Func, Elem = ret, Columns = prms };
        }
        else if (AtWord("cArray"))
        {
            Next();
            Expect(Tk.LParen, "a cArray lists its columns: cArray(string itemName, int amount)");
            var cols = new List<ParamDecl>();
            while (!At(Tk.RParen))
            {
                if (Cur.Kind == Tk.Ident && (PeekAt(1).Kind is Tk.Comma or Tk.RParen))
                    throw Fail(Cur.Pos, $"say what type the column {Cur.Text} is: cArray(string {Cur.Text})");
                var ctype = Type();
                var cname = ExpectName("a column name");
                if (cname.Text == "id") throw Fail(cname.Pos, "every cArray already has an id column, so leave it out of the list.");
                cols.Add(new ParamDecl(ctype, cname.Text, cname.Pos));
                if (At(Tk.Comma)) { Next(); continue; }
                if (!At(Tk.RParen)) throw Fail(MissingAt, "expected , or ) here.");
            }
            Next();
            if (cols.Count == 0) throw Fail(t.Pos, "a cArray needs at least one column besides its id: cArray(string itemName)");
            type = new TypeRef { Pos = t.Pos, Kind = TypeKind.CArray, Columns = cols };
        }
        else
        {
            if (!At(Tk.Ident)) throw Fail(MissingAt, "expected a type here, like int or Player.");
            if (Hints.TryGetValue(Cur.Text, out var hint) && LType.Primitive(Cur.Text) == null) throw Fail(Cur.Pos, hint);
            Next();
            string? module = null;
            string name = t.Text;
            if (At(Tk.Dot) && PeekAt(1).Kind == Tk.Ident) { Next(); module = t.Text; name = Next().Text; }
            type = new TypeRef { Pos = t.Pos, Kind = TypeKind.Named, Name = name, Module = module };
        }
        while (AtWord("array") || AtWord("map"))
        {
            var kind = Next().Text == "map" ? TypeKind.Map : TypeKind.Array;
            type = new TypeRef { Pos = t.Pos, Kind = kind, Elem = type };
        }
        return type;
    }

    void CheckName(Token t)
    {
        if (t.Text is "false" or "null") throw Fail(t.Pos, Hints[t.Text]);
        if (Reserved.Contains(t.Text)) throw Fail(t.Pos, $"{t.Text} is one of Lust's own words, so it can't be a name.");
        if (Builtins.Contains(t.Text)) throw Fail(t.Pos, $"{t.Text} is built into Lust, so it can't be a name.");
    }

    // ----- statements -----

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
            case "give":
            {
                Next();
                Expr? value = At(Tk.Semi) ? null : Expression();
                EndStatement();
                return new Give { Pos = t.Pos, Value = value };
            }
            case "end": throw Fail(t.Pos, "end only goes on the file's last line. Is a } missing above?");
            case "then": throw Fail(t.Pos, "then comes after a question: is hp 0? then { }");
            case "or": throw Fail(t.Pos, "or goes right after the } of a question: } or { }");
            case "class" or "object": throw Fail(t.Pos, $"a {t.Text} goes at the top level of a file, not inside a block.");
            case "do" when !DoIsType(): throw Fail(t.Pos, "a function can't go inside another one. Put do at the top level of the file.");
            case "method": throw Fail(t.Pos, "a method can't go inside another one. Put it directly inside its class.");
            case "send" or "fetch" or "get": throw Fail(t.Pos, $"{t.Text} goes at the top level of a file.");
            case "has": throw Fail(t.Pos, "a has block goes inside a class or object.");
            case "save" or "load":
            {
                Next();
                if (!At(Tk.Ident)) throw Fail(MissingAt, $"say which save: {t.Text} game;");
                var n = Next();
                if (t.Text == "save" && At(Tk.Assign)) throw Fail(t.Pos, "a save is declared at the top level of a file, not inside a block.");
                if (At(Tk.LParen) || At(Tk.Dot) || At(Tk.Colon) || At(Tk.Ident) || At(Tk.String))
                    throw Fail(Cur.Pos, $"{t.Text} takes the name of a save, declared at the top of the file: save {n.Text} = [Farm, player]; then {t.Text} {n.Text};");
                EndStatement();
                return new SaveStmt { Pos = t.Pos, Name = n.Text, NamePos = n.Pos, IsLoad = t.Text == "load" };
            }
            case "map" or "array": throw Fail(t.Pos, $"say what the {t.Text} holds first, like int {t.Text} {(PeekAt(1).Kind == Tk.Ident ? PeekAt(1).Text : "list")} = [];");
            case "state" when IsStateDeclaration(): throw Fail(t.Pos, "a state is declared at the top level of a file.");
        }
        if (LooksLikeDeclaration()) return VarDeclStmt();
        if (HintApplies(t)) { diags.Add(t.Pos, Hints[t.Text]); throw new ParseStop(); }

        var target = Postfix();
        if (At(Tk.Assign))
        {
            if (target is TruthLit or NothingLit)
                throw Fail(t.Pos, $"{t.Text} is one of Lust's own words; it can't be changed.");
            if (target is not (Name or FieldGet or Index))
                throw Fail(t.Pos, "the left side of = must be something that can change, like hp or bo.hp.");
            if (target is Name { Id: var id } && Reserved.Contains(id) && id != "state")
                throw Fail(t.Pos, $"{id} is one of Lust's own words; it can't be changed.");
            Next();
            var value = Expression();
            EndStatement();
            return new Assign { Pos = t.Pos, Target = target, Value = value };
        }
        if (AtWord("is") && Cur.Pos.Line == Prev.Pos.Line)
        {
            Next();
            var opt = OptionPath($"say which option: {Snippet(target)} is idle;");
            if (At(Tk.LParen)) throw Fail(Cur.Pos, $"is sets a state to one of its options by name. To set it from a function, use =: {Snippet(target)} = {opt.Text}(…);");
            EndStatement();
            return new StateSet { Pos = t.Pos, Target = target, Option = opt.Text, OptionPos = opt.Pos };
        }
        if (target is MethodRef mr) throw Fail(mr.MethodPos, $"add () to call it: {Snippet(mr.Target)}:{mr.Method}()");
        if (target is not (Call or MethodCall or CallValue))
            throw Fail(t.Pos, "this line doesn't do anything. A line can declare, change a variable, call something, ask a question or loop.");
        EndStatement();
        return new ExprStmt { Pos = t.Pos, Expr = target };
    }

    Stmt VarDeclStmt()
    {
        var start = Cur;
        var type = Type();
        if (At(Tk.LBrack)) throw Fail(Cur.Pos, $"{type}[…] only goes inside a has block. Here, write {type} name = value;");
        if (!At(Tk.Ident)) throw Fail(MissingAt, $"give the {type} a name, like {type} {Example(type)};");
        var nameTok = Next();
        CheckName(nameTok);
        if (At(Tk.Semi) || At(Tk.Eof))
            throw Fail(nameTok.End, $"give {nameTok.Text} a starting value, like {type} {nameTok.Text} = {Example(type).Split(" = ")[1]};");
        if (!At(Tk.Assign)) throw Fail(Cur.Pos, $"expected = and a starting value after {nameTok.Text}.");
        Next();
        var init = Expression();
        EndStatement();
        return new VarDecl { Pos = start.Pos, TypeRef = type, Id = nameTok.Text, IdPos = nameTok.Pos, Init = init };
    }

    static string Example(TypeRef type) => type.Kind != TypeKind.Named ? "list = []" : type.Name switch
    {
        "int" => "lives = 3",
        "float" => "speed = 1.5",
        "string" => "name = \"Ada\"",
        "truth" => "alive = true",
        _ => "x = …",
    };

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
            if (!At(Tk.Question)) throw Fail(MissingAt, QuestionEndMessage("is hp 0? then { }"));
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

    string QuestionEndMessage(string example) =>
        AtWord("and") ? "Lust joins checks with &&, not and: is hp > 5 && hp < 10?"
        : AtWord("or") ? "inside a question, \"or\" is written ||: is hp 0 || hungry?"
        : AtWord("then") ? $"a question ends with ? before then: {example}"
        : $"a question ends with ?, like {example}";

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
        bool negated = false;
        if (AtWord("not") && PeekAt(1).Is("like")) { Next(); negated = true; }
        if (AtWord("like"))
        {
            Next();
            if (!At(Tk.Ident)) throw Fail(MissingAt, $"say which class: is {Snippet(left)} like Wizard?");
            var cls = Next();
            return new IsLike { Pos = left.Pos, Value = left, Class = cls.Text, ClassPos = cls.Pos, Negated = negated };
        }
        if (At(Tk.Assign)) throw Fail(Cur.Pos, $"questions don't use =. Put the two things side by side: is {Snippet(left)} 3?");
        if (AtWord("not"))
        {
            Next();
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
        Tk.Ident => (!Reserved.Contains(t.Text) && t.Text != "and") || t.Text is "true" or "nothing" or "either" or "new" or "saved",
        _ => false,
    };

    static string Snippet(Expr e) => e switch
    {
        Name n => n.Id,
        FieldGet f => $"{Snippet(f.Target)}.{f.Field}",
        _ => "x",
    };

    // ----- loop -----

    Stmt LoopStmt()
    {
        var start = Next().Pos;
        if (At(Tk.LBrace)) return new Loop { Pos = start, Kind = LoopKind.Forever, Body = BlockStmt() };
        if (At(Tk.LParen))
        {
            if (LoopHasIn())
            {
                Next(); // (
                var vars = new List<ParamDecl>();
                while (true)
                {
                    if (!At(Tk.Ident)) throw Fail(MissingAt, "expected a type and a name, like loop (int n in scores)");
                    if (PeekAt(1).Is("in") || PeekAt(1).Kind == Tk.Comma)
                        throw Fail(Cur.Pos, $"say what type {Cur.Text} is: loop (int {Cur.Text} in …)");
                    var vtype = Type();
                    var vname = ExpectName("a name");
                    vars.Add(new ParamDecl(vtype, vname.Text, vname.Pos));
                    if (At(Tk.Comma)) { Next(); continue; }
                    break;
                }
                ExpectWord("in", "expected in, like loop (int n in scores)");
                var source = Expression();
                Expect(Tk.RParen, "expected ) to close the loop.");
                return new Loop { Pos = start, Kind = LoopKind.Each, Vars = vars, Expr = source, Body = BlockStmt() };
            }
            Next();
            // The check works like a question's: loop (cmd not "quit") { }
            var cond = Check();
            if (!At(Tk.RParen)) throw Fail(MissingAt, AtWord("and") || AtWord("or") ? QuestionEndMessage("") : "expected ) to close the loop's check.");
            Next();
            return new Loop { Pos = start, Kind = LoopKind.While, Expr = cond, Body = BlockStmt() };
        }
        var count = Expression();
        if (!At(Tk.LBrace)) throw Fail(Cur.Pos, "expected { after the number of times to loop.");
        return new Loop { Pos = start, Kind = LoopKind.Times, Expr = count, Body = BlockStmt() };
    }

    /// Is there an `in` between this ( and its matching )?
    bool LoopHasIn()
    {
        int depth = 0;
        for (int i = p; i < toks.Count; i++)
        {
            var t = toks[i];
            if (t.Kind is Tk.LParen or Tk.LBrack) depth++;
            else if (t.Kind is Tk.RParen or Tk.RBrack) { if (--depth == 0) return false; }
            else if (t.Kind is Tk.LBrace or Tk.Semi or Tk.Eof) return false;
            else if (depth == 1 && t.Is("in")) return true;
        }
        return false;
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
        if (!At(Tk.Question)) throw Fail(MissingAt, AtWord("and") || AtWord("or") ? QuestionEndMessage("") : "either needs a question ending with ?: either hp > 0? \"up\" or \"down\"");
        Next();
        var yes = Expression();
        if (!AtWord("or")) throw Fail(MissingAt, "either needs an or: either hp > 0? \"up\" or \"down\"");
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
        if (At(Tk.Minus) && PeekAt(1).Kind is Tk.Int or Tk.Float && !PeekAt(1).SpaceBefore && PeekAt(2).Kind is Tk.Dot or Tk.Colon)
        {
            // -2.5.int(): the minus belongs to the number.
            var minus = Next();
            var num = Next();
            Expr lit = num.Kind == Tk.Int ? new IntLit { Pos = minus.Pos, Value = -num.IntValue } : new FloatLit { Pos = minus.Pos, Value = -num.FloatValue, Text = "-" + num.Text };
            return Postfix(lit);
        }
        if (At(Tk.Minus))
        {
            var t = Next();
            var operand = Negate();
            // -3 is just the number -3.
            if (operand is IntLit i) return new IntLit { Pos = t.Pos, Value = -i.Value };
            if (operand is FloatLit f) return new FloatLit { Pos = t.Pos, Value = -f.Value, Text = "-" + f.Text };
            return new Unary { Pos = t.Pos, Op = UnOp.Neg, Operand = operand };
        }
        if (At(Tk.Hash))
        {
            var t = Next();
            return new Length { Pos = t.Pos, Target = Postfix() };
        }
        return Postfix();
    }

    static readonly HashSet<string> Conversions = new() { "text", "int", "float", "isNumber", "lines" };

    Expr Postfix(Expr? start = null)
    {
        var e = start ?? Primary();
        while (true)
        {
            if (At(Tk.Dot))
            {
                Next();
                if (!At(Tk.Ident)) throw Fail(MissingAt, "expected a name after the dot, like bo.hp or x.text().");
                var m = Next();
                if (At(Tk.LParen) && !Cur.SpaceBefore)
                {
                    if (!Conversions.Contains(m.Text))
                        throw Fail(m.Pos, e is Name or FieldGet
                            ? $"methods are called with : instead of a dot: {Snippet(e)}:{m.Text}(…)"
                            : $"Lust doesn't have .{m.Text}(). The built-in ones are .text(), .int(), .float(), .isNumber() and .lines().");
                    Next();
                    if (!At(Tk.RParen)) throw Fail(Cur.Pos, $".{m.Text}() doesn't take anything between its ().");
                    Next();
                    e = new Conversion { Pos = e.Pos, Target = e, Method = m.Text, MethodPos = m.Pos };
                    continue;
                }
                e = new FieldGet { Pos = e.Pos, Target = e, Field = m.Text, FieldPos = m.Pos };
                continue;
            }
            if (At(Tk.Colon))
            {
                Next();
                if (!At(Tk.Ident)) throw Fail(MissingAt, "expected a method name after :, like bo:bark()");
                var m = Next();
                if (!At(Tk.LParen))
                {
                    // bo:bark without (): the method itself, to call later.
                    e = new MethodRef { Pos = e.Pos, Target = e, Method = m.Text, MethodPos = m.Pos };
                    continue;
                }
                var args = Arguments(m.Text);
                e = new MethodCall { Pos = e.Pos, Target = e, Method = m.Text, MethodPos = m.Pos, Args = args };
                continue;
            }
            if (At(Tk.LBrack) && (!Cur.SpaceBefore || !noSpacedCall))
            {
                var open = Next();
                bool saved = noSpacedCall;
                noSpacedCall = false;
                Expr key;
                try
                {
                    if (At(Tk.RBrack)) throw Fail(Cur.Pos, "say which item, like scores[0]");
                    key = Expression();
                    Expect(Tk.RBrack, "expected ] here.");
                }
                finally { noSpacedCall = saved; }
                e = new Index { Pos = e.Pos, Target = e, Key = key };
                continue;
            }
            if (At(Tk.LParen) && !Cur.SpaceBefore && e is Index or CallValue or Call or MethodCall)
            {
                // handlers[0](5): call the function kept there
                e = new CallValue { Pos = e.Pos, Target = e, Args = Arguments("it") };
                continue;
            }
            return e;
        }
    }

    List<Expr> Arguments(string callee)
    {
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
            Expect(Tk.RParen, $"expected ) to close {callee}(.");
        }
        finally { noSpacedCall = saved; }
        return args;
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
                bool saved = noSpacedCall;
                noSpacedCall = false;
                try
                {
                    // A whole check fits in ( ), so questions can group: is a && (b 1 || c 2)?
                    var e = AtWord("either") ? EitherExpr() : Check();
                    Expect(Tk.RParen, "expected ) here.");
                    return e;
                }
                finally { noSpacedCall = saved; }
            }
            case Tk.LBrack:
            {
                Next();
                var lit = new ArrayLit { Pos = t.Pos };
                bool saved = noSpacedCall;
                // ["apple" 3]: two values side by side are a map's key and value, so "apple" [1] isn't picking from "apple".
                noSpacedCall = true;
                try
                {
                    while (!At(Tk.RBrack))
                    {
                        var item = Expression();
                        if (StartsValue(Cur) && !At(Tk.LBrack) || At(Tk.LBrack) && Cur.SpaceBefore)
                        {
                            if (lit.Keys == null && lit.Items.Count > 0) throw Fail(Cur.Pos, "this looks like a map's key and value, but the items before it are plain. Write every item as key value, like [\"apple\" 3, \"seed\" 1].");
                            lit.Keys ??= new();
                            lit.Keys.Add(item);
                            item = Expression();
                        }
                        else if (lit.Keys != null) throw Fail(MissingAt, $"in a map every item is a key and a value, like \"apple\" 3.");
                        lit.Items.Add(item);
                        if (At(Tk.Comma)) { Next(); continue; }
                        if (!At(Tk.RBrack)) throw Fail(MissingAt, "expected , or ] here.");
                    }
                    Next();
                }
                finally { noSpacedCall = saved; }
                return lit;
            }
            case Tk.Eof: throw Fail(Prev.End, "the file stops in the middle of this line.");
            case Tk.Ident: break;
            default: throw Fail(t.Pos, $"expected a value here, but found {Describe(t)}.");
        }

        if (t.Text == "true") { Next(); return new TruthLit { Pos = t.Pos }; }
        if (t.Text == "nothing") { Next(); return new NothingLit { Pos = t.Pos }; }
        if (t.Text == "either") return EitherExpr();
        if (t.Text == "new") return NewExpr();
        if (t.Text == "saved")
        {
            Next();
            if (!At(Tk.Ident)) throw Fail(MissingAt, "say which save: is saved game?");
            var n = Next();
            return new SavedExpr { Pos = t.Pos, Name = n.Text, NamePos = n.Pos };
        }
        if (t.Text is "false" or "null" || (t.Text is "self" or "this" && PeekAt(1).Kind == Tk.Dot)) throw Fail(t.Pos, Hints[t.Text]);
        if (t.Text == "state") { Next(); return new Name { Pos = t.Pos, Id = "state" }; }
        if (Reserved.Contains(t.Text)) throw Fail(t.Pos, $"expected a value here, but found the word {t.Text}.");

        Next();
        bool call = At(Tk.LParen) && (!Cur.SpaceBefore || !noSpacedCall);
        if (!call) return new Name { Pos = t.Pos, Id = t.Text };
        return new Call { Pos = t.Pos, Callee = t.Text, Args = Arguments(t.Text) };
    }

    Expr NewExpr()
    {
        var start = Next().Pos; // new
        if (!At(Tk.Ident)) throw Fail(MissingAt, "say what to make: new Player");
        var cls = Next();
        // new Random(42): only C# types take arguments; the checker says so for Lust classes.
        var args = At(Tk.LParen) ? Arguments(cls.Text) : null;
        var e = new New { Pos = start, Class = cls.Text, ClassPos = cls.Pos, Args = args };
        if (!At(Tk.LBrace)) return e;
        Next();
        bool saved = noSpacedCall;
        noSpacedCall = false;
        try
        {
            while (!At(Tk.RBrace))
            {
                if (!At(Tk.Ident)) throw Fail(MissingAt, $"expected a field and its value, like new {cls.Text} {{ name \"Bo\" }}");
                var f = Next();
                if (At(Tk.Assign)) throw Fail(Cur.Pos, $"no = inside new {{ }}. Write the field and its value: {f.Text} …");
                if (At(Tk.Comma) || At(Tk.RBrace)) throw Fail(MissingAt, $"give {f.Text} a value: new {cls.Text} {{ {f.Text} … }}");
                var value = Expression();
                e.Inits.Add((f.Text, f.Pos, value));
                if (At(Tk.Comma)) { Next(); continue; }
                if (!At(Tk.RBrace)) throw Fail(MissingAt, "expected , or } here.");
            }
            Next();
        }
        finally { noSpacedCall = saved; }
        return e;
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
