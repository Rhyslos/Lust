// Saving and loading for Lust programs. The compiler copies this file into every Lust program.
//
// A save file is text that looks like Lust, so it can be read (and edited):
//
//   // Lust save "game" for valley.
//   Farm {
//       gold 120
//       season summer
//       crops [@1, @2]
//   }
//   bag [[1, "sword", 1], [2, "apple", 3]]
//   @1 Crop {
//       name "corn"
//   }
//
// Objects and variables come first, by name. Things made from classes are written once each,
// after them, as @1, @2…, so two fields that hold the same thing still hold the same thing after loading.
// An array, map or cArray that's in two places is written once as @3 [ … ], and then @3.
// Loading is strict: anything that doesn't match the program exactly is an error, and nothing is changed.
#nullable disable
using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;

public sealed class LustSave
{
    // ----- what the compiler tells us -----

    /// The program's name, for its folder of saves.
    public static string Program = "lust";
    static readonly Dictionary<Type, string> typeNames = new();
    static readonly Dictionary<string, Type> types = new();

    /// A class or object's name in save files.
    public static void Name(Type t, string name) { typeNames[t] = name; types[name] = t; }

    public abstract class Item { public string Name; }

    /// An object, filled in place when loading.
    public sealed class Obj : Item
    {
        public Func<LustObject> Get;
        public Obj(string name, Func<LustObject> get) { Name = name; Get = get; }
    }

    /// A variable from the top of the file the program runs.
    public sealed class Var : Item
    {
        public Type Type;
        public Func<object> Get;
        public Action<object> Set;
        public Var(string name, Type type, Func<object> get, Action<object> set) { Name = name; Type = type; Get = get; Set = set; }
    }

    readonly string name;
    readonly Item[] items;

    public LustSave(string name, params Item[] items) { this.name = name; this.items = items; }

    // ----- where -----

    static string Folder
    {
        get
        {
            var over = Environment.GetEnvironmentVariable("LUST_SAVES");
            if (!string.IsNullOrEmpty(over)) return over;
            return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Lust", Program);
        }
    }

    string FilePath => Path.Combine(Folder, name + ".save");

    public bool Exists() => File.Exists(FilePath);

    // ----- fields -----

    static readonly Dictionary<Type, FieldInfo[]> fieldCache = new();

    /// A class's Lust fields, the ones from the class it's like first. Widgets are left out:
    /// a window's build() makes them again.
    internal static FieldInfo[] Fields(Type t)
    {
        lock (fieldCache)
        {
            if (fieldCache.TryGetValue(t, out var cached)) return cached;
            var chain = new List<Type>();
            for (var c = t; c != null && c != typeof(LustObject) && c != typeof(object); c = c.BaseType) chain.Insert(0, c);
            var list = chain.SelectMany(c => c.GetFields(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly))
                .Where(f => f.Name.StartsWith("f_") && !IsWidget(f.FieldType))
                .ToArray();
            fieldCache[t] = list;
            return list;
        }
    }

    /// Window parts (yours too, like a Button you made) are left out of saves, and so is anything holding them.
    static bool IsWidget(Type t)
    {
        for (var c = t; c != null; c = c.BaseType) if (c.Name == "C_Ui_Widget") return true;
        if (t.IsGenericType && t.GetGenericArguments().Any(IsWidget)) return true;
        if (typeof(LustRow).IsAssignableFrom(t)) return t.GetFields(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly).Any(f => f.Name.StartsWith("c_") && IsWidget(f.FieldType));
        return false;
    }
    static bool IsObjectType(Type t) => t.GetField("Instance", BindingFlags.Public | BindingFlags.Static) != null;
    static string LustField(FieldInfo f) => f.Name.Substring(2);
    string TypeName(Type t) => typeNames.TryGetValue(t, out var n) ? n : t.Name;

    // ===== saving =====

    public void Save()
    {
        var w = new Writer(this);
        foreach (var item in items) w.Count(item is Obj o ? o.Get() : ((Var)item).Get(), item is Obj);
        var sb = w.Out;
        sb.Append($"// Lust save \"{name}\" for {Program}. You can change values here, carefully: when it doesn't match the program, loading stops with an error.\n");
        foreach (var item in items)
        {
            switch (item)
            {
                case Obj o:
                    sb.Append(item.Name).Append(' ');
                    w.Body(o.Get(), 0);
                    break;
                case Var v:
                    sb.Append(item.Name).Append(' ');
                    w.Value(v.Get(), v.Type, 0);
                    sb.Append('\n');
                    break;
            }
        }
        w.Rest();
        try
        {
            Directory.CreateDirectory(Folder);
            var tmp = FilePath + ".tmp";
            File.WriteAllText(tmp, sb.ToString());
            File.Move(tmp, FilePath, overwrite: true);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new LustError($"couldn't write the save {name} to {FilePath}: {e.Message}");
        }
    }

    sealed class Writer
    {
        public readonly StringBuilder Out = new();
        readonly LustSave save;
        readonly Dictionary<object, int> ids = new(ReferenceEqualityComparer.Instance);
        readonly Queue<object> todo = new();
        public Writer(LustSave save) { this.save = save; }

        // How many places each array, map and cArray is in, worked out before writing.
        readonly Dictionary<object, int> uses = new(ReferenceEqualityComparer.Instance);
        readonly HashSet<object> counted = new(ReferenceEqualityComparer.Instance);

        static bool IsCollection(object v) => v != null && v.GetType().IsGenericType &&
            (v.GetType().GetGenericTypeDefinition() is var d && (d == typeof(LustArray<>) || d == typeof(LustMap<>) || d == typeof(LustCArray<>)));

        /// Goes through everything the way writing will, counting the arrays, maps and cArrays met.
        public void Count(object v, bool isItem = false)
        {
            switch (v)
            {
                case null: return;
                case LustObject o:
                    if (!isItem && IsObjectType(o.GetType())) return;   // written by name
                    if (!counted.Add(o)) return;
                    foreach (var f in Fields(o.GetType())) Count(f.GetValue(o));
                    return;
            }
            if (!IsCollection(v)) return;
            uses[v] = uses.TryGetValue(v, out var n) ? n + 1 : 1;
            if (uses[v] > 1) return;
            var t = v.GetType();
            var def = t.GetGenericTypeDefinition();
            if (def == typeof(LustArray<>)) foreach (var x in (IEnumerable)v) Count(x);
            else if (def == typeof(LustMap<>)) foreach (var kv in (IEnumerable)t.GetMethod("Snapshot").Invoke(v, null)) Count(kv.GetType().GetProperty("Value").GetValue(kv));
            else
                foreach (var row in (LustRow[])t.GetMethod("Snapshot").Invoke(v, null))
                    foreach (var c in row.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly).Where(f => f.Name.StartsWith("c_"))) Count(c.GetValue(row));
        }

        public void Body(object o, int indent)
        {
            var fields = Fields(o.GetType());
            if (fields.Length == 0) { Out.Append("{ }\n"); return; }
            Out.Append("{\n");
            foreach (var f in fields)
            {
                Out.Append(' ', (indent + 1) * 4).Append(LustField(f)).Append(' ');
                Value(f.GetValue(o), f.FieldType, indent + 1);
                Out.Append('\n');
            }
            Out.Append(' ', indent * 4).Append("}\n");
        }

        /// The things made from classes, each once, after everything else.
        public void Rest()
        {
            while (todo.Count > 0)
            {
                var o = todo.Dequeue();
                Out.Append('@').Append(ids[o]).Append(' ').Append(save.TypeName(o.GetType())).Append(' ');
                Body(o, 0);
            }
        }

        public void Value(object v, Type type, int indent)
        {
            switch (v)
            {
                case null: Out.Append("nothing"); return;
                case long l: Out.Append(l.ToString(CultureInfo.InvariantCulture)); return;
                case double d:
                    Out.Append(double.IsNaN(d) ? "nan" : double.IsPositiveInfinity(d) ? "infinity" : double.IsNegativeInfinity(d) ? "-infinity" : LustRt.Text(d));
                    return;
                case bool b: Out.Append(b ? "true" : "not true"); return;
                case string s: Out.Append(Quote(s)); return;
                case Enum e: Out.Append(LustRt.Text(e)); return;
                case LustObject o:
                    if (IsObjectType(o.GetType())) { Out.Append(save.TypeName(o.GetType())); return; }
                    if (!ids.TryGetValue(o, out var id)) { ids[o] = id = ids.Count + 1; todo.Enqueue(o); }
                    Out.Append('@').Append(id);
                    return;
            }
            var t = v.GetType();
            // A list that's in more than one place: written once with its number, then by its number.
            if (IsCollection(v) && uses.TryGetValue(v, out var places) && places > 1)
            {
                if (ids.TryGetValue(v, out var seen)) { Out.Append('@').Append(seen); return; }
                ids[v] = ids.Count + 1;
                Out.Append('@').Append(ids[v]).Append(' ');
            }
            if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(LustArray<>))
            {
                var elem = t.GetGenericArguments()[0];
                Out.Append('[');
                bool first = true;
                foreach (var x in (IEnumerable)v) { if (!first) Out.Append(", "); first = false; Value(x, elem, indent); }
                Out.Append(']');
                return;
            }
            if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(LustMap<>))
            {
                var elem = t.GetGenericArguments()[0];
                Out.Append('[');
                bool first = true;
                foreach (var kv in (IEnumerable)t.GetMethod("Snapshot").Invoke(v, null))
                {
                    if (!first) Out.Append(", ");
                    first = false;
                    var kt = kv.GetType();
                    Out.Append(Quote((string)kt.GetProperty("Key").GetValue(kv))).Append(' ');
                    Value(kt.GetProperty("Value").GetValue(kv), elem, indent);
                }
                Out.Append(']');
                return;
            }
            if (t.IsGenericType && t.GetGenericTypeDefinition() == typeof(LustCArray<>))
            {
                var rowType = t.GetGenericArguments()[0];
                var cols = rowType.GetFields(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly).Where(f => f.Name.StartsWith("c_")).ToArray();
                var rows = (LustRow[])t.GetMethod("Snapshot").Invoke(v, null);
                Out.Append('[');
                for (int i = 0; i < rows.Length; i++)
                {
                    if (i > 0) Out.Append(", ");
                    Out.Append('[').Append(rows[i].id);
                    foreach (var c in cols) { Out.Append(", "); Value(c.GetValue(rows[i]), c.FieldType, indent); }
                    Out.Append(']');
                }
                Out.Append(']');
                // Ids are never reused, even after the biggest one was removed.
                long next = (long)t.GetProperty("NextId", BindingFlags.NonPublic | BindingFlags.Instance).GetValue(v);
                long expected = rows.Length == 0 ? 1 : rows.Max(r => r.id) + 1;
                if (next != expected) Out.Append(" next ").Append(next);
                return;
            }
            throw new LustError($"Lust doesn't know how to save {LustRt.Text(v)}.");
        }

        static string Quote(string s)
        {
            var sb = new StringBuilder("\"");
            foreach (var c in s)
                sb.Append(c switch { '"' => "\\\"", '\\' => "\\\\", '\n' => "\\n", '\t' => "\\t", '\r' => "\\r", _ => c.ToString() });
            return sb.Append('"').ToString();
        }
    }

    // ===== loading =====

    public void Load()
    {
        if (!Exists()) throw new LustError($"there's no save called {name} yet. Check first with: is saved {name}?");
        string text;
        try { text = File.ReadAllText(FilePath); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { throw new LustError($"couldn't read the save {name} ({e.Message})."); }

        var reader = new Reader(this, text);
        var file = reader.ReadFile();
        // Everything is worked out first, and only then put in place, so a broken save changes nothing.
        var apply = new List<Action>();
        var made = new Dictionary<int, object>();
        foreach (var (id, typeName, body, line) in file.Things)
        {
            if (!types.TryGetValue(typeName, out var t) || IsObjectType(t))
                throw reader.Mismatch(line, $"there's no class called {typeName} in the program.");
            made[id] = Activator.CreateInstance(t);
        }
        var resolve = new Resolver(this, reader, made, file.Labels);
        foreach (var (id, typeName, body, line) in file.Things) resolve.Fill(made[id], body, apply);

        var seen = new HashSet<string>();
        foreach (var (entryName, node, line) in file.Entries)
        {
            var item = items.FirstOrDefault(i => i.Name == entryName);
            if (item == null) throw reader.Mismatch(line, $"the save has {entryName}, which isn't in save {name} = [{string.Join(", ", items.Select(i => i.Name))}].");
            if (!seen.Add(entryName)) throw reader.Damaged(line, $"{entryName} is in the save twice.");
            switch (item)
            {
                case Obj o:
                    if (node is not Node.Body b) throw reader.Mismatch(line, $"{entryName} is an object, so its fields go in {{ }}.");
                    resolve.Fill(o.Get(), b, apply);
                    break;
                case Var v:
                    var value = resolve.Convert(node, v.Type, entryName);
                    apply.Add(() => v.Set(value));
                    break;
            }
        }
        var missing = items.FirstOrDefault(i => !seen.Contains(i.Name));
        if (missing != null) throw reader.Mismatch(0, $"{missing.Name} isn't in the save. It was probably added to save {name} after this save was made.");
        foreach (var a in apply) a();
    }

    // ----- reading the text -----

    abstract class Node
    {
        public int Line;
        public sealed class Word : Node { public string Text; }          // 42, -1.5, true, summer, Farm, nothing, fighting.blocking
        public sealed class Text : Node { public string Value; }        // "corn"
        public sealed class Ref : Node { public int Id; }               // @3
        public sealed class List : Node                                 // [1, 2] or ["a" 1] or [[1, "x"]] next 4
        {
            public int? Label;                                          // @3 [ … ]: in more than one place
            public List<Node> Items = new();
            public List<Node> Keys;
            public long? Next;
        }
        public sealed class Body : Node { public List<(string Field, Node Value, int Line)> Fields = new(); }
    }

    sealed class SaveFile
    {
        public List<(string Name, Node Node, int Line)> Entries = new();
        public List<(int Id, string Type, Node.Body Body, int Line)> Things = new();
        public Dictionary<int, Node.List> Labels = new();
    }

    sealed class Reader
    {
        readonly LustSave save;
        readonly string s;
        int i, line = 1;
        SaveFile file;
        public Reader(LustSave save, string text) { this.save = save; s = text.Replace("\r\n", "\n"); }

        string Where => save.FilePath;
        public LustError Damaged(int at, string message) => new($"the save {save.name} is damaged on line {at}: {message} ({Where})");
        public LustError Mismatch(int at, string message) =>
            new($"the save {save.name} doesn't match the program anymore{(at > 0 ? $" (line {at})" : "")}: {message} ({Where})");

        void Space()
        {
            while (i < s.Length)
            {
                if (s[i] == '\n') { line++; i++; }
                else if (char.IsWhiteSpace(s[i])) i++;
                else if (s[i] == '/' && i + 1 < s.Length && s[i + 1] == '/') { while (i < s.Length && s[i] != '\n') i++; }
                else break;
            }
        }

        bool At(char c) { Space(); return i < s.Length && s[i] == c; }

        void Expect(char c)
        {
            if (!At(c)) throw Damaged(line, i >= s.Length ? $"it ends where a {c} was expected." : $"expected {c} but found {s[i]}.");
            i++;
        }

        string Word()
        {
            Space();
            int start = i;
            while (i < s.Length && (char.IsLetterOrDigit(s[i]) || s[i] is '_' or '.' or '-' or '+')) i++;
            if (i == start) throw Damaged(line, i >= s.Length ? "it ends too early." : $"didn't expect {s[i]} here.");
            return s.Substring(start, i - start);
        }

        public SaveFile ReadFile()
        {
            file = new SaveFile();
            var thingIds = new HashSet<int>();
            while (true)
            {
                Space();
                if (i >= s.Length) break;
                int at = line;
                if (s[i] == '@')
                {
                    i++;
                    var idText = Word();
                    if (!int.TryParse(idText, NumberStyles.None, CultureInfo.InvariantCulture, out var id)) throw Damaged(at, $"@{idText} should be @ and a number.");
                    var typeName = Word();
                    if (!At('{')) throw Damaged(line, $"@{id} {typeName} needs its fields in {{ }}.");
                    if (!thingIds.Add(id) || file.Labels.ContainsKey(id)) throw Damaged(at, $"@{id} is there twice.");
                    file.Things.Add((id, typeName, ReadBody(), at));
                    continue;
                }
                var entry = Word();
                file.Entries.Add((entry, At('{') ? ReadBody() : ReadValue(), at));
            }
            return file;
        }

        Node.Body ReadBody()
        {
            var body = new Node.Body { Line = line };
            Expect('{');
            while (!At('}'))
            {
                if (i >= s.Length) throw Damaged(body.Line, "a { is never closed.");
                int at = line;
                var field = Word();
                body.Fields.Add((field, ReadValue(), at));
            }
            i++;
            return body;
        }

        Node ReadValue()
        {
            Space();
            int at = line;
            if (i >= s.Length) throw Damaged(line, "it ends where a value was expected.");
            char c = s[i];
            if (c == '"') return new Node.Text { Line = at, Value = ReadText() };
            if (c == '@')
            {
                i++;
                var idText = Word();
                if (!int.TryParse(idText, NumberStyles.None, CultureInfo.InvariantCulture, out var id)) throw Damaged(at, $"@{idText} should be @ and a number.");
                if (At('['))
                {
                    var labelled = (Node.List)ReadValue();
                    if (file.Labels.ContainsKey(id)) throw Damaged(at, $"@{id} is there twice.");
                    labelled.Label = id;
                    file.Labels[id] = labelled;
                    return labelled;
                }
                return new Node.Ref { Line = at, Id = id };
            }
            if (c == '[')
            {
                i++;
                var list = new Node.List { Line = at };
                while (!At(']'))
                {
                    if (i >= s.Length) throw Damaged(at, "a [ is never closed.");
                    var first = ReadValue();
                    Space();
                    if (i < s.Length && s[i] != ',' && s[i] != ']')
                    {
                        (list.Keys ??= new()).Add(first);
                        list.Items.Add(ReadValue());
                    }
                    else list.Items.Add(first);
                    if (At(',')) { i++; continue; }
                    if (!At(']')) throw Damaged(line, "expected , or ] here.");
                }
                i++;
                // A cArray can say which id comes next: [[1, "sword", 1]] next 4
                // It's on the same line, so a field that happens to be called next isn't mistaken for it.
                int j = i;
                while (j < s.Length && s[j] is ' ' or '\t') j++;
                if (s.AsSpan(j).StartsWith("next ") || s.AsSpan(j).StartsWith("next\t"))
                {
                    i = j + 4;
                    var n = Word();
                    if (!long.TryParse(n, NumberStyles.None, CultureInfo.InvariantCulture, out var next)) throw Damaged(line, $"next should be followed by a number, not {n}.");
                    list.Next = next;
                }
                return list;
            }
            var word = Word();
            // not true is two words
            if (word == "not")
            {
                var second = Word();
                if (second != "true") throw Damaged(at, $"not {second} isn't a value. Only not true is.");
                return new Node.Word { Line = at, Text = "not true" };
            }
            return new Node.Word { Line = at, Text = word };
        }

        string ReadText()
        {
            int at = line;
            i++; // "
            var sb = new StringBuilder();
            while (true)
            {
                if (i >= s.Length || s[i] == '\n') throw Damaged(at, "a piece of text never ends. Add the closing \".");
                char c = s[i++];
                if (c == '"') break;
                if (c == '\\' && i < s.Length)
                {
                    char e = s[i++];
                    sb.Append(e switch { 'n' => '\n', 't' => '\t', 'r' => '\r', '"' => '"', '\\' => '\\', _ => throw Damaged(at, $"\\{e} isn't something Lust knows inside text.") });
                    continue;
                }
                sb.Append(c);
            }
            return sb.ToString();
        }
    }

    // ----- turning the text into values -----

    sealed class Resolver
    {
        readonly LustSave save;
        readonly Reader reader;
        readonly Dictionary<int, object> made;
        readonly Dictionary<int, Node.List> labels;
        readonly Dictionary<int, object> lists = new();
        public Resolver(LustSave save, Reader reader, Dictionary<int, object> made, Dictionary<int, Node.List> labels) { this.save = save; this.reader = reader; this.made = made; this.labels = labels; }

        /// Remembers a list that's in more than one place as soon as it's made, so the other places get the very same one.
        void Made(Node.List list, object value) { if (list.Label is int l) lists[l] = value; }

        LustError Wrong(Node n, string where, string wanted) =>
            reader.Mismatch(n.Line, $"{where} should be {wanted}, but it's {Show(n)}.");

        static string Show(Node n) => n switch
        {
            Node.Word w => w.Text,
            Node.Text t => "\"" + t.Value + "\"",
            Node.Ref r => "@" + r.Id,
            Node.List => "a list in [ ]",
            _ => "{ … }",
        };

        public void Fill(object target, Node.Body body, List<Action> apply)
        {
            var type = target.GetType();
            var typeName = save.TypeName(type);
            var fields = Fields(type);
            foreach (var (field, value, line) in body.Fields)
            {
                var f = fields.FirstOrDefault(x => LustField(x) == field);
                if (f == null) throw reader.Mismatch(line, $"{typeName} has no field called {field}.");
                if (body.Fields.Count(x => x.Field == field) > 1) throw reader.Damaged(line, $"{field} is there twice.");
                var v = Convert(value, f.FieldType, $"{typeName}.{field}");
                apply.Add(() => f.SetValue(target, v));
            }
            var missing = fields.FirstOrDefault(f => !body.Fields.Any(x => x.Field == LustField(f)));
            if (missing != null) throw reader.Mismatch(body.Line, $"{typeName} has a field {LustField(missing)} that isn't in the save.");
        }

        public object Convert(Node n, Type t, string where)
        {
            if (t == typeof(long))
            {
                if (n is Node.Word w && long.TryParse(w.Text, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var l)) return l;
                throw Wrong(n, where, "a whole number (an int)");
            }
            if (t == typeof(double))
            {
                if (n is Node.Word w)
                {
                    if (w.Text == "nan") return double.NaN;
                    if (w.Text == "infinity") return double.PositiveInfinity;
                    if (w.Text == "-infinity") return double.NegativeInfinity;
                    if (double.TryParse(w.Text, NumberStyles.Float, CultureInfo.InvariantCulture, out var d)) return d;
                }
                throw Wrong(n, where, "a number (a float)");
            }
            if (t == typeof(bool))
            {
                if (n is Node.Word { Text: "true" }) return true;
                if (n is Node.Word { Text: "not true" }) return false;
                throw Wrong(n, where, "true or not true");
            }
            if (t == typeof(string))
            {
                if (n is Node.Text s) return s.Value;
                throw Wrong(n, where, "text in quotes");
            }
            if (t.IsEnum)
            {
                var options = t.GetFields(BindingFlags.Public | BindingFlags.Static)
                    .Select(f => (f, name: f.GetCustomAttribute<LustNameAttribute>()?.Name ?? f.Name)).ToList();
                if (n is Node.Word w)
                {
                    var hit = options.FirstOrDefault(o => o.name == w.Text);
                    if (hit.f != null) return hit.f.GetValue(null);
                }
                throw Wrong(n, where, "one of " + string.Join(", ", options.Select(o => o.name)));
            }
            if (typeof(LustObject).IsAssignableFrom(t))
            {
                switch (n)
                {
                    case Node.Word { Text: "nothing" }: return null;
                    case Node.Ref r:
                        if (!made.TryGetValue(r.Id, out var o)) throw reader.Damaged(r.Line, $"@{r.Id} is used, but there's no @{r.Id} further down.");
                        if (!t.IsInstanceOfType(o)) throw reader.Mismatch(r.Line, $"{where} should be a {save.TypeName(t)}, but @{r.Id} is a {save.TypeName(o.GetType())}.");
                        return o;
                    case Node.Word w when types.TryGetValue(w.Text, out var objType) && IsObjectType(objType):
                        var inst = objType.GetField("Instance").GetValue(null);
                        if (!t.IsInstanceOfType(inst)) throw Wrong(n, where, $"a {save.TypeName(t)}");
                        return inst;
                }
                throw Wrong(n, where, $"a {save.TypeName(t)} (written @ and a number) or nothing");
            }
            if (t.IsGenericType)
            {
                var def = t.GetGenericTypeDefinition();
                var arg = t.GetGenericArguments()[0];
                // @3: a list that's written out somewhere else in the save
                if (n is Node.Ref listRef)
                {
                    if (!lists.TryGetValue(listRef.Id, out var shared))
                    {
                        if (!labels.TryGetValue(listRef.Id, out var node)) throw reader.Damaged(listRef.Line, $"@{listRef.Id} is used, but there's no @{listRef.Id} [ … ] in the save.");
                        shared = Convert(node, t, where);
                    }
                    if (!t.IsInstanceOfType(shared)) throw reader.Mismatch(listRef.Line, $"{where} and the other place @{listRef.Id} is used hold different kinds of things.");
                    return shared;
                }
                if (n is Node.List { Label: int lb } && lists.TryGetValue(lb, out var already))
                {
                    if (!t.IsInstanceOfType(already)) throw reader.Mismatch(n.Line, $"{where} and the other place @{lb} is used hold different kinds of things.");
                    return already;
                }
                if (n is not Node.List list) throw Wrong(n, where, def == typeof(LustMap<>) ? "a map in [ ]" : "a list in [ ]");
                if (def == typeof(LustArray<>))
                {
                    if (list.Keys != null) throw Wrong(n, where, "a plain list, without keys");
                    var a = Activator.CreateInstance(t);
                    Made(list, a);
                    var inject = t.GetMethod("Inject");
                    for (int k = 0; k < list.Items.Count; k++) inject.Invoke(a, new[] { Convert(list.Items[k], arg, $"{where}[{k}]") });
                    return a;
                }
                if (def == typeof(LustMap<>))
                {
                    if (list.Keys == null && list.Items.Count > 0) throw Wrong(n, where, "a map, with a key before each value");
                    var m = Activator.CreateInstance(t);
                    Made(list, m);
                    var set = t.GetMethod("Set");
                    var has = t.GetMethod("Has");
                    for (int k = 0; k < list.Items.Count; k++)
                    {
                        if (list.Keys[k] is not Node.Text key) throw Wrong(list.Keys[k], $"a key in {where}", "text in quotes");
                        if ((bool)has.Invoke(m, new object[] { key.Value })) throw reader.Damaged(key.Line, $"\"{key.Value}\" is in {where} twice.");
                        set.Invoke(m, new[] { key.Value, Convert(list.Items[k], arg, $"{where}[\"{key.Value}\"]") });
                    }
                    return m;
                }
                if (def == typeof(LustCArray<>))
                {
                    var rowType = arg;
                    var cols = rowType.GetFields(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly).Where(f => f.Name.StartsWith("c_")).ToArray();
                    var ctor = rowType.GetConstructors()[0];
                    var c = Activator.CreateInstance(t, new object[] { Array.CreateInstance(rowType, 0) });
                    Made(list, c);
                    var inject = t.GetMethod("Inject");
                    foreach (var item in list.Items)
                    {
                        if (item is not Node.List row || row.Keys != null || row.Items.Count != cols.Length + 1)
                            throw Wrong(item, $"a row of {where}", $"[id, {string.Join(", ", cols.Select(x => x.Name.Substring(2)))}]");
                        var args = new object[cols.Length + 2];
                        args[0] = true;
                        args[1] = Convert(row.Items[0], typeof(long), $"a row's id in {where}");
                        for (int k = 0; k < cols.Length; k++) args[k + 2] = Convert(row.Items[k + 1], cols[k].FieldType, $"{where}[{args[1]}].{cols[k].Name.Substring(2)}");
                        try { inject.Invoke(c, new[] { ctor.Invoke(args) }); }
                        catch (TargetInvocationException e) when (e.InnerException is LustError le) { throw reader.Damaged(row.Line, le.Message); }
                    }
                    if (list.Next is long next)
                    {
                        var prop = t.GetProperty("NextId", BindingFlags.NonPublic | BindingFlags.Instance);
                        if (next < (long)prop.GetValue(c)) throw reader.Damaged(list.Line, $"next {next} is smaller than an id that's already used.");
                        prop.SetValue(c, next);
                    }
                    return c;
                }
            }
            throw reader.Mismatch(n.Line, $"Lust doesn't know how to load {where}.");
        }
    }
}
