// Lust's UI runtime, on top of WinForms. The compiler copies this file into Lust
// programs that fetch ui. The class and member names match what Lust writes for
// ui.lust: C_Ui_Button, f_text, m_clicked and so on.
//
// How it works: a Lust program changes plain fields (button.text = "Go").
// After every click, Lust copies the fields of every widget onto the screen
// (Push). Before running a click, it copies what the person typed or ticked
// back into the fields (Pull). So Lust code never touches WinForms itself.
#nullable disable
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

public class C_Ui_Widget : LustObject
{
    public string f_text = "";
    public bool f_enabled = true;
    public bool f_visible = true;

    internal Control control;
    public override string LustName => "Widget";
    public override string LustShow() => $"{LustName} {{ text {LustRt.Show(f_text)}, enabled {LustRt.Text(f_enabled)}, visible {LustRt.Text(f_visible)} }}";

    internal virtual Control Make() => new Label { AutoSize = true };

    /// Lust's fields onto the screen.
    internal virtual void Push()
    {
        if (control.Text != (f_text ?? "")) control.Text = f_text ?? "";
        control.Enabled = f_enabled;
        control.Visible = f_visible;
    }

    /// What the person changed on screen back into Lust's fields.
    internal virtual void Pull() { }

    internal virtual IEnumerable<C_Ui_Widget> Children => Array.Empty<C_Ui_Widget>();
}

public class C_Ui_Label : C_Ui_Widget
{
    public override string LustName => "Label";
    internal override Control Make() => new Label { AutoSize = true, Margin = new Padding(0, 4, 0, 4), MaximumSize = new Size(600, 0) };
}

public class C_Ui_Button : C_Ui_Widget
{
    public override string LustName => "Button";
    public virtual void m_clicked() { }

    internal override Control Make()
    {
        var b = new Button { AutoSize = true, AutoSizeMode = AutoSizeMode.GrowAndShrink, Padding = new Padding(10, 3, 10, 3), Margin = new Padding(0, 4, 8, 4), UseVisualStyleBackColor = true };
        b.Click += (_, _) => LustUi.Event(m_clicked);
        return b;
    }
}

public class C_Ui_Input : C_Ui_Widget
{
    public override string LustName => "Input";
    internal override Control Make() => new TextBox { Width = 260, Margin = new Padding(0, 4, 8, 4) };
    internal override void Pull() => f_text = control.Text;
}

public class C_Ui_Toggle : C_Ui_Widget
{
    public bool f_on;
    bool pushing;
    public override string LustName => "Toggle";
    public override string LustShow() => $"Toggle {{ text {LustRt.Show(f_text)}, on {LustRt.Text(f_on)}, enabled {LustRt.Text(f_enabled)}, visible {LustRt.Text(f_visible)} }}";
    public virtual void m_toggled() { }

    internal override Control Make()
    {
        var c = new CheckBox { AutoSize = true, Margin = new Padding(0, 4, 8, 4) };
        c.CheckedChanged += (_, _) => { if (!pushing) LustUi.Event(m_toggled); };
        return c;
    }

    internal override void Push()
    {
        base.Push();
        pushing = true;
        ((CheckBox)control).Checked = f_on;
        pushing = false;
    }

    internal override void Pull() => f_on = ((CheckBox)control).Checked;
}

public class C_Ui_Row : C_Ui_Widget
{
    readonly List<C_Ui_Widget> kids = new();
    public override string LustName => "Row";
    internal override IEnumerable<C_Ui_Widget> Children => kids;

    internal override Control Make()
    {
        var p = new FlowLayoutPanel { FlowDirection = FlowDirection.LeftToRight, WrapContents = false, AutoSize = true, AutoSizeMode = AutoSizeMode.GrowAndShrink, Margin = new Padding(0) };
        foreach (var k in kids) p.Controls.Add(LustUi.Create(k));
        return p;
    }

    internal override void Push()
    {
        control.Enabled = f_enabled;
        control.Visible = f_visible;
    }

    public virtual void m_add(C_Ui_Widget w)
    {
        if (w == null) throw new LustError("can't add nothing to a row.");
        kids.Add(w);
        if (control != null) control.Controls.Add(LustUi.Create(w));
    }
}

public class C_Ui_Window : LustObject
{
    public string f_title = "Lust";
    public long f_width = 480, f_height = 360;

    internal Form form;
    FlowLayoutPanel panel;
    readonly List<C_Ui_Widget> widgets = new();
    long shownWidth = -1, shownHeight = -1;

    public override string LustName => "Window";
    public override string LustShow() => $"Window {{ title {LustRt.Show(f_title)}, width {f_width}, height {f_height} }}";

    internal IEnumerable<C_Ui_Widget> All() => widgets.SelectMany(Flatten);
    static IEnumerable<C_Ui_Widget> Flatten(C_Ui_Widget w) => new[] { w }.Concat(w.Children.SelectMany(Flatten));

    public virtual void m_build() { }

    /// Shows the window. The first window keeps the program here until it's closed;
    /// windows opened from a click appear next to it.
    public virtual void m_open()
    {
        if (form != null && !form.IsDisposed) { form.Activate(); return; }
        LustUi.Start();
        form = new Form { StartPosition = FormStartPosition.CenterScreen, Font = new Font("Segoe UI", 10f), BackColor = Color.White };
        panel = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.TopDown, WrapContents = false, AutoScroll = true, Padding = new Padding(16) };
        form.Controls.Add(panel);
        LustUi.Windows.Add(this);
        form.FormClosed += (_, _) => LustUi.Windows.Remove(this);
        LustUi.Event(Build);
        if (LustUi.Smoke) form.Shown += (_, _) => LustUi.SmokeTest(this);
        if (Application.MessageLoop) form.Show();
        else Application.Run(form);
    }

    void Build()
    {
        panel.SuspendLayout();
        panel.Controls.Clear();
        widgets.Clear();
        m_build();
        panel.ResumeLayout();
    }

    public virtual void m_add(C_Ui_Widget w)
    {
        if (w == null) throw new LustError("can't add nothing to a window.");
        widgets.Add(w);
        if (panel != null) panel.Controls.Add(LustUi.Create(w));
    }

    public virtual void m_label(string words) => m_add(new C_Ui_Label { f_text = words });

    /// Empties the window and runs build() again, for switching screens.
    public virtual void m_rebuild()
    {
        if (panel == null) return;
        Build();
    }

    public virtual void m_alert(string message)
    {
        if (LustUi.Smoke) { Console.WriteLine("[alert] " + message); return; }
        MessageBox.Show(form, message ?? "", f_title);
    }

    public virtual void m_close() => form?.Close();

    internal void Push()
    {
        if (form == null || form.IsDisposed) return;
        if (form.Text != f_title) form.Text = f_title ?? "";
        if (f_width != shownWidth || f_height != shownHeight)
        {
            form.ClientSize = new Size((int)Math.Clamp(f_width, 120, 4000), (int)Math.Clamp(f_height, 80, 4000));
            shownWidth = f_width;
            shownHeight = f_height;
        }
        foreach (var w in All()) if (w.control != null) w.Push();
    }

    internal void Pull()
    {
        foreach (var w in All()) if (w.control != null) w.Pull();
    }
}

static class LustUi
{
    internal static readonly List<C_Ui_Window> Windows = new();
    static bool started;

    internal static void Start()
    {
        if (started) return;
        started = true;
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        // A mistake while a window is open also shows in a message box, since the window closes.
        LustRt.OnFail = message => MessageBox.Show(message, "Lust stopped", MessageBoxButtons.OK, MessageBoxIcon.Error);
    }

    // ----- smoke test -----
    // With LUST_UI_SMOKE=1, a window clicks every button and ticks every box by itself
    // (three times over), prints what its labels say after each, then closes. For tests
    // on machines where nobody is there to click.
    internal static readonly bool Smoke = Environment.GetEnvironmentVariable("LUST_UI_SMOKE") == "1";

    internal static void SmokeTest(C_Ui_Window w)
    {
        try { SmokeSteps(w); }
        catch (Exception e) { Console.WriteLine("[smoke test crashed] " + e); }
        w.form.Close();
    }

    static void SmokeSteps(C_Ui_Window w)
    {
        Console.WriteLine($"[smoke] {w.All().Count()} widgets");
        void Dump(string what)
        {
            Console.WriteLine($"[{what}] title: {w.f_title}");
            foreach (var x in w.All()) if (x.control?.Parent != null && x is C_Ui_Label) Console.WriteLine("  " + x.f_text);
        }
        Dump("opened");
        for (int round = 0; round < 3; round++)
        {
            foreach (var x in w.All().ToList())
            {
                var c = x.control;
                if (c == null || c.Parent == null || !c.Visible || !c.Enabled) continue;
                if (c is TextBox t && t.Text == "") t.Text = "Smokey";
                else if (c is Button b) { b.PerformClick(); Dump("clicked " + x.f_text); }
                else if (c is CheckBox box) { box.Checked = !box.Checked; Dump("ticked " + x.f_text); }
            }
        }
    }

    internal static Control Create(C_Ui_Widget w)
    {
        w.control = w.Make();
        w.Push();
        return w.control;
    }

    /// Runs Lust code for a click or a tick: screen → fields, the code, fields → screen.
    internal static void Event(Action handler)
    {
        foreach (var win in Windows.ToList()) win.Pull();
        try { handler(); }
        catch (Exception e) { LustRt.Fail(e); }
        foreach (var win in Windows.ToList()) win.Push();
    }
}
