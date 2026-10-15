namespace Lust;

/// How C# types are written: in the C# Lust writes (global::System.Random), and in messages (Random).
public static class CsNames
{
    public static string Of(System.Type t)
    {
        if (t.IsArray) return Of(t.GetElementType()!) + "[" + new string(',', t.GetArrayRank() - 1) + "]";
        if (t.IsGenericParameter) return t.Name;
        var name = t.IsNested ? Of(t.DeclaringType!) + "." + Plain(t.Name) : "global::" + (t.Namespace != null ? t.Namespace + "." : "") + Plain(t.Name);
        if (!t.IsGenericType) return name;
        // Generic arguments of a nested type belong to it, minus the ones its outer type already has.
        var args = t.GetGenericArguments();
        int outer = t.IsNested ? t.DeclaringType!.GetGenericArguments().Length : 0;
        var own = args.Skip(outer).ToArray();
        return own.Length == 0 ? name : $"{name}<{string.Join(", ", own.Select(Of))}>";
    }

    public static string Short(System.Type t)
    {
        if (t.IsArray) return Short(t.GetElementType()!) + "[]";
        var name = Plain(t.Name);
        if (t.IsGenericType) name += "<" + string.Join(", ", t.GetGenericArguments().Select(Short)) + ">";
        return name;
    }

    static string Plain(string name) => name.Contains('`') ? name[..name.IndexOf('`')] : name;
}
