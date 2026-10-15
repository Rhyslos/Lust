using System.Globalization;
using System.Text;

namespace Lust;

public enum Tk
{
    Ident, Int, Float, String,
    Semi, Comma, Dot, Colon, LParen, RParen, LBrace, RBrace, LBrack, RBrack, Question, Hash,
    Plus, Minus, Star, Slash, Percent, Assign, Lt, Gt, Le, Ge, AndAnd, OrOr,
    Eof,
}

public sealed record Token(Tk Kind, string Text, Pos Pos, bool SpaceBefore)
{
    public long IntValue { get; init; }
    public double FloatValue { get; init; }
    public string StringValue { get; init; } = "";

    /// Where this token ends, for "missing ; here" messages.
    public Pos End => Pos with { Col = Pos.Col + Text.Length };

    public bool Is(string word) => Kind == Tk.Ident && Text == word;
}

/// Chops a file into tokens: words, numbers, text and symbols. Comments and spaces disappear here.
public static class Lexer
{
    public static List<Token> Lex(SourceFile file, Diagnostics diags)
    {
        var tokens = new List<Token>();
        string s = file.Text;
        int i = 0, line = 1, col = 1;
        bool space = true;

        Pos Here() => new(file, line, col);
        void Advance(int n = 1)
        {
            for (int k = 0; k < n && i < s.Length; k++, i++)
            {
                if (s[i] == '\n') { line++; col = 1; } else col++;
            }
        }
        void Add(Tk kind, int length, Pos pos)
        {
            tokens.Add(new Token(kind, s.Substring(i, length), pos, space));
            Advance(length);
            space = false;
        }

        while (i < s.Length)
        {
            char c = s[i];
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r') { Advance(); space = true; continue; }
            if (c == '/' && Peek(1) == '/') { while (i < s.Length && s[i] != '\n') Advance(); space = true; continue; }
            if (c == '/' && Peek(1) == '*') { diags.AddSymbol(Here(), "Lust comments start with // and run to the end of the line. There are no /* */ comments."); Advance(2); continue; }

            var pos = Here();
            if (char.IsLetter(c) || c == '_')
            {
                int j = i;
                while (j < s.Length && (char.IsLetterOrDigit(s[j]) || s[j] == '_')) j++;
                Add(Tk.Ident, j - i, pos);
                continue;
            }
            if (c >= '0' && c <= '9')
            {
                int j = i;
                while (j < s.Length && s[j] >= '0' && s[j] <= '9') j++;
                bool isFloat = j + 1 < s.Length && s[j] == '.' && s[j + 1] >= '0' && s[j + 1] <= '9';
                if (isFloat) { j++; while (j < s.Length && s[j] >= '0' && s[j] <= '9') j++; }
                string text = s.Substring(i, j - i);
                if (j < s.Length && (char.IsLetter(s[j]) || s[j] == '_'))
                {
                    diags.AddSymbol(pos, $"a name can't start with a number.");
                }
                if (isFloat)
                {
                    var v = double.Parse(text, CultureInfo.InvariantCulture);
                    if (double.IsInfinity(v)) diags.AddSymbol(pos, $"this number is too big for a float.");
                    tokens.Add(new Token(Tk.Float, text, pos, space) { FloatValue = v });
                }
                else
                {
                    if (!long.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture, out var v))
                        diags.AddSymbol(pos, $"{text} is too big for an int.");
                    tokens.Add(new Token(Tk.Int, text, pos, space) { IntValue = v });
                }
                Advance(j - i);
                space = false;
                continue;
            }
            if (c == '"')
            {
                var sb = new StringBuilder();
                int j = i + 1;
                bool closed = false;
                while (j < s.Length && s[j] != '\n')
                {
                    if (s[j] == '"') { closed = true; break; }
                    if (s[j] == '\\' && j + 1 < s.Length)
                    {
                        char e = s[j + 1];
                        switch (e)
                        {
                            case 'n': sb.Append('\n'); break;
                            case 't': sb.Append('\t'); break;
                            case '"': sb.Append('"'); break;
                            case '\\': sb.Append('\\'); break;
                            default:
                                diags.AddSymbol(new Pos(file, line, col + (j - i)), $"\\{e} isn't something Lust knows inside text. Use \\n, \\t, \\\" or \\\\.");
                                sb.Append(e); break;
                        }
                        j += 2;
                        continue;
                    }
                    sb.Append(s[j]);
                    j++;
                }
                if (!closed) diags.AddSymbol(pos, "this text never ends. Add a closing \" on the same line.");
                int len = (closed ? j + 1 : j) - i;
                tokens.Add(new Token(Tk.String, s.Substring(i, len), pos, space) { StringValue = sb.ToString() });
                Advance(len);
                space = false;
                continue;
            }
            if (c == '\'') { diags.AddSymbol(pos, "text goes in double quotes, like \"hello\"."); Advance(); continue; }

            string two = i + 1 < s.Length ? s.Substring(i, 2) : "";
            switch (two)
            {
                case "<=": Add(Tk.Le, 2, pos); continue;
                case ">=": Add(Tk.Ge, 2, pos); continue;
                case "&&": Add(Tk.AndAnd, 2, pos); continue;
                case "||": Add(Tk.OrOr, 2, pos); continue;
                case "==": diags.AddSymbol(pos, "Lust has no ==. To check if two things are the same, ask: is a b?"); Advance(2); continue;
                case "!=": diags.AddSymbol(pos, "Lust has no !=. To check if two things differ, ask: is a not b?"); Advance(2); continue;
                case "+=": case "-=": case "*=": case "/=":
                {
                    var v = tokens.Count > 0 && tokens[^1].Kind == Tk.Ident ? tokens[^1].Text : "hp";
                    diags.AddSymbol(pos, $"Lust has no {two} yet. Write it out: {v} = {v} {two[0]} …;"); Advance(2); continue;
                }
                case "++": case "--":
                {
                    var v = tokens.Count > 0 && tokens[^1].Kind == Tk.Ident ? tokens[^1].Text : "n";
                    diags.AddSymbol(pos, $"Lust has no {two}. Write it out: {v} = {v} {two[0]} 1;"); Advance(2); continue;
                }
            }
            Tk? one = c switch
            {
                ';' => Tk.Semi, ',' => Tk.Comma, '.' => Tk.Dot, ':' => Tk.Colon,
                '(' => Tk.LParen, ')' => Tk.RParen, '{' => Tk.LBrace, '}' => Tk.RBrace,
                '[' => Tk.LBrack, ']' => Tk.RBrack, '?' => Tk.Question, '#' => Tk.Hash,
                '+' => Tk.Plus, '-' => Tk.Minus, '*' => Tk.Star, '/' => Tk.Slash, '%' => Tk.Percent,
                '=' => Tk.Assign, '<' => Tk.Lt, '>' => Tk.Gt,
                _ => null,
            };
            if (one is Tk k) { Add(k, 1, pos); continue; }
            if (c == '!') { diags.AddSymbol(pos, "Lust uses the word not instead of !."); Advance(); continue; }
            if (c == '&' || c == '|') { diags.AddSymbol(pos, $"use {c}{c} to join checks."); Advance(); continue; }
            diags.AddSymbol(pos, $"Lust doesn't know what {c} means here.");
            Advance();
        }
        tokens.Add(new Token(Tk.Eof, "", Here(), space));
        return tokens;

        char Peek(int n) => i + n < s.Length ? s[i + n] : '\0';
    }
}
