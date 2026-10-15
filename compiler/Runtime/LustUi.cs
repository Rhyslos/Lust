// Lust's UI runtime, on top of WinForms. The compiler copies this file into Lust
// programs that fetch ui. The class and member names match what Lust writes for
// ui.lust: C_Ui_Button, f_text, m_clicked and so on.
//
// How it works: a Lust program changes plain fields (button.text = "Go").
// After every click, tick or key, Lust copies the fields of every widget onto the screen
// (Push). Before running one, it copies what the person typed or ticked back into the
// fields (Pull). So Lust code never touches WinForms itself.
#nullable disable
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public class C_Ui_Widget : LustObject
{
    public string f_text = "";
    public bool f_enabled = true;
    public bool f_visible = true;
    public string f_color = "";
    public string f_background = "";
    public long f_textSize = 0;

    internal Control control;
    Color usualFore, usualBack;
    Font usualFont;
    string shownColor, shownBackground;
    long shownSize;

    public override string LustName => "Widget";
    public override string LustShow() => $"{LustName} {{ text {LustRt.Show(f_text)}, enabled {LustRt.Text(f_enabled)}, visible {LustRt.Text(f_visible)} }}";

    internal virtual Control Make() => new Label { AutoSize = true };

    internal void Made()
    {
        usualFore = control.ForeColor;
        usualBack = control.BackColor;
        usualFont = control.Font;
        shownColor = shownBackground = "";
        shownSize = 0;
    }

    /// Lust's fields onto the screen.
    internal virtual void Push()
    {
        if (control.Text != (f_text ?? "")) control.Text = f_text ?? "";
        control.Enabled = f_enabled;
        control.Visible = f_visible;
        PushLooks();
    }

    protected void PushLooks()
    {
        if ((f_color ?? "") != shownColor) { control.ForeColor = f_color == "" || f_color == null ? usualFore : LustUi.Color(f_color); shownColor = f_color ?? ""; }
        if ((f_background ?? "") != shownBackground) { control.BackColor = f_background == "" || f_background == null ? usualBack : LustUi.Color(f_background); shownBackground = f_background ?? ""; }
        if (f_textSize != shownSize)
        {
            if (f_textSize < 0 || f_textSize > 200) throw new LustError($"textSize {f_textSize} is too {(f_textSize < 0 ? "small" : "big")}. Use 0 for the usual size, or something like 8 to 72.");
            control.Font = f_textSize == 0 ? usualFont : new Font(usualFont.FontFamily, f_textSize);
            shownSize = f_textSize;
        }
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
        PushLooks();
    }

    public virtual void m_add(C_Ui_Widget w)
    {
        if (w == null) throw new LustError("can't add nothing to a row.");
        kids.Add(w);
        if (control != null) control.Controls.Add(LustUi.Create(w));
    }
}

public class C_Ui_Picture : C_Ui_Widget
{
    public string f_file = "";
    public long f_width, f_height;
    string shownFile;
    long shownW = -1, shownH = -1;

    public override string LustName => "Picture";
    public override string LustShow() => $"Picture {{ file {LustRt.Show(f_file)}, width {f_width}, height {f_height} }}";

    internal override Control Make() => new PictureBox { SizeMode = PictureBoxSizeMode.Zoom, Margin = new Padding(0, 4, 8, 4) };

    internal override void Push()
    {
        var box = (PictureBox)control;
        box.Enabled = f_enabled;
        box.Visible = f_visible;
        PushLooks();
        if (f_file != shownFile || f_width != shownW || f_height != shownH)
        {
            var image = string.IsNullOrEmpty(f_file) ? null : LustUi.Image(f_file);
            box.Image = image;
            box.Size = image == null ? new Size((int)Math.Max(f_width, 0), (int)Math.Max(f_height, 0))
                : new Size(f_width > 0 ? (int)f_width : image.Width, f_height > 0 ? (int)f_height : image.Height);
            shownFile = f_file; shownW = f_width; shownH = f_height;
        }
    }
}

public class C_Ui_Canvas : C_Ui_Widget
{
    public long f_width = 400, f_height = 300;
    readonly List<Action<Graphics>> drawings = new();

    public override string LustName => "Canvas";
    public override string LustShow() => $"Canvas {{ width {f_width}, height {f_height}, {drawings.Count} drawings }}";
    internal int Drawings => drawings.Count;

    sealed class Surface : Panel
    {
        public Surface() { DoubleBuffered = true; ResizeRedraw = true; }
    }

    internal override Control Make()
    {
        var s = new Surface { BackColor = Color.White, Margin = new Padding(0, 4, 8, 4), Cursor = Cursors.Hand };
        s.Paint += (_, e) =>
        {
            e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
            foreach (var d in drawings) d(e.Graphics);
        };
        s.MouseClick += (_, e) => LustUi.Event(() => m_clicked(e.X, e.Y));
        return s;
    }

    internal override void Push()
    {
        control.Enabled = f_enabled;
        control.Visible = f_visible;
        var size = new Size((int)Math.Clamp(f_width, 1, 4000), (int)Math.Clamp(f_height, 1, 4000));
        if (control.Size != size) control.Size = size;
        PushLooks();
    }

    void Draw(Action<Graphics> d)
    {
        drawings.Add(d);
        // A canvas that's drawn on again and again (in tick) keeps only its newest drawings.
        if (drawings.Count > 20000) drawings.RemoveRange(0, drawings.Count - 20000);
        control?.Invalidate();
    }

    public virtual void m_clear() { drawings.Clear(); control?.Invalidate(); }

    public virtual void m_rect(long x, long y, long w, long h, string color)
    {
        var c = LustUi.Color(color);
        Draw(g => { using var b = new SolidBrush(c); g.FillRectangle(b, x, y, w, h); });
    }

    public virtual void m_circle(long x, long y, long radius, string color)
    {
        var c = LustUi.Color(color);
        Draw(g => { using var b = new SolidBrush(c); g.FillEllipse(b, x - radius, y - radius, radius * 2, radius * 2); });
    }

    public virtual void m_line(long x1, long y1, long x2, long y2, string color)
    {
        var c = LustUi.Color(color);
        Draw(g => { using var p = new Pen(c, 2); g.DrawLine(p, x1, y1, x2, y2); });
    }

    public virtual void m_write(long x, long y, string words, string color)
    {
        var c = LustUi.Color(color);
        var size = f_textSize > 0 ? f_textSize : 11;
        Draw(g => { using var b = new SolidBrush(c); using var f = new Font("Segoe UI", size); g.DrawString(words ?? "", f, b, x, y); });
    }

    public virtual void m_picture(string file, long x, long y)
    {
        var image = LustUi.Image(file);
        Draw(g => g.DrawImage(image, x, y, image.Width, image.Height));
    }

    public virtual void m_clicked(long x, long y) { }
}

public class C_Ui_Window : LustObject
{
    public string f_title = "Lust";
    public long f_width = 480, f_height = 360;
    public string f_background = "";

    internal Form form;
    FlowLayoutPanel panel;
    readonly List<C_Ui_Widget> widgets = new();
    long shownWidth = -1, shownHeight = -1;
    string shownBackground = "";
    internal Timer timer;

    public override string LustName => "Window";
    public override string LustShow() => $"Window {{ title {LustRt.Show(f_title)}, width {f_width}, height {f_height} }}";

    internal IEnumerable<C_Ui_Widget> All() => widgets.SelectMany(Flatten);
    static IEnumerable<C_Ui_Widget> Flatten(C_Ui_Widget w) => new[] { w }.Concat(w.Children.SelectMany(Flatten));

    public virtual void m_build() { }
    public virtual void m_tick() { }
    public virtual void m_keyPressed(string key) { }

    /// A form that tells Lust about every key, even the arrow keys that buttons would otherwise keep.
    sealed class LustForm : Form
    {
        public Action<Keys> KeyDownAny;
        protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
        {
            const int WM_KEYDOWN = 0x100, WM_SYSKEYDOWN = 0x104;
            if (msg.Msg is WM_KEYDOWN or WM_SYSKEYDOWN) KeyDownAny?.Invoke(keyData & Keys.KeyCode);
            return base.ProcessCmdKey(ref msg, keyData);
        }
    }

    /// Shows the window. The first window keeps the program here until it's closed;
    /// windows opened from a click appear next to it.
    public virtual void m_open()
    {
        if (form != null && !form.IsDisposed) { form.Activate(); return; }
        LustUi.Start();
        var f = new LustForm { StartPosition = FormStartPosition.CenterScreen, Font = new Font("Segoe UI", 10f), BackColor = Color.White, KeyPreview = true };
        f.KeyDownAny = key => LustUi.Event(() => m_keyPressed(LustUi.KeyName(key)));
        form = f;
        panel = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.TopDown, WrapContents = false, AutoScroll = true, Padding = new Padding(16) };
        form.Controls.Add(panel);
        LustUi.Windows.Add(this);
        form.FormClosed += (_, _) => { timer?.Stop(); LustUi.Windows.Remove(this); };
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

    public virtual void m_startTimer(double seconds)
    {
        if (double.IsNaN(seconds) || seconds <= 0) throw new LustError($"startTimer needs a number of seconds above 0, like startTimer(0.05), not {LustRt.Text(seconds)}.");
        LustUi.Start();
        timer ??= MakeTimer();
        timer.Interval = (int)Math.Clamp(seconds * 1000, 10, int.MaxValue);
        if (!LustUi.Smoke) timer.Start();
        LustUi.TimerStarted = true;
    }

    Timer MakeTimer()
    {
        var t = new Timer();
        t.Tick += (_, _) => LustUi.Event(m_tick);
        return t;
    }

    public virtual void m_stopTimer() => timer?.Stop();

    public virtual bool m_isKeyDown(string key)
    {
        if (form == null || !form.ContainsFocus) return false;
        if (LustUi.KeyCode(key) is not Keys k) throw new LustError($"Lust doesn't know a key called \"{key}\". Try a letter (\"A\"), a number (\"7\"), \"Space\", \"Enter\", \"Left\", \"Right\", \"Up\" or \"Down\".");
        return (LustUi.GetAsyncKeyState((int)k) & 0x8000) != 0;
    }

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
        if ((f_background ?? "") != shownBackground)
        {
            form.BackColor = string.IsNullOrEmpty(f_background) ? Color.White : LustUi.Color(f_background);
            shownBackground = f_background ?? "";
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

    // ----- colours, pictures and keys -----

    internal static Color Color(string name)
    {
        var s = (name ?? "").Trim();
        if (s.StartsWith("#"))
        {
            var hex = s.Substring(1);
            if (hex.Length == 3) hex = string.Concat(hex.Select(c => $"{c}{c}"));
            if (hex.Length == 6 && int.TryParse(hex, System.Globalization.NumberStyles.HexNumber, null, out var rgb))
                return System.Drawing.Color.FromArgb(255, (rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255);
        }
        else if (s.Length > 0)
        {
            var c = System.Drawing.Color.FromName(s.Replace(" ", ""));
            if (c.IsKnownColor) return c;
        }
        throw new LustError($"Lust doesn't know the colour \"{name}\". Use a name like \"red\" or \"skyblue\", or a code like \"#ff3355\".");
    }

    static readonly Dictionary<string, Image> images = new();

    internal static Image Image(string file)
    {
        var path = LustRt.FullPath(file);
        if (images.TryGetValue(path, out var cached)) return cached;
        if (!System.IO.File.Exists(path)) throw new LustError($"there's no picture called {file} in {System.IO.Path.GetDirectoryName(path)}.");
        try
        {
            // Read into memory, so the file isn't kept locked while the program runs.
            using var stream = new System.IO.MemoryStream(System.IO.File.ReadAllBytes(path));
            var image = new Bitmap(stream);
            images[path] = image;
            return image;
        }
        catch (ArgumentException) { throw new LustError($"{file} isn't a picture Lust can show. Use png, jpg, gif or bmp."); }
    }

    static readonly Dictionary<string, Keys> keyNames = new(StringComparer.OrdinalIgnoreCase)
    {
        ["Space"] = Keys.Space, ["Enter"] = Keys.Enter, ["Escape"] = Keys.Escape, ["Tab"] = Keys.Tab,
        ["Backspace"] = Keys.Back, ["Delete"] = Keys.Delete, ["Shift"] = Keys.ShiftKey, ["Control"] = Keys.ControlKey,
        ["Alt"] = Keys.Menu, ["Left"] = Keys.Left, ["Right"] = Keys.Right, ["Up"] = Keys.Up, ["Down"] = Keys.Down,
    };

    internal static string KeyName(Keys k)
    {
        if (k >= Keys.D0 && k <= Keys.D9) return ((int)(k - Keys.D0)).ToString();
        if (k >= Keys.NumPad0 && k <= Keys.NumPad9) return ((int)(k - Keys.NumPad0)).ToString();
        foreach (var (name, key) in keyNames) if (key == k) return name;
        if (k == Keys.Return) return "Enter";
        return k.ToString();
    }

    internal static Keys? KeyCode(string name)
    {
        if (string.IsNullOrEmpty(name)) return null;
        if (keyNames.TryGetValue(name, out var k)) return k;
        if (name.Length == 1 && char.IsDigit(name[0])) return Keys.D0 + (name[0] - '0');
        if (name.Length == 1 && char.IsLetter(name[0])) return (Keys)char.ToUpperInvariant(name[0]);
        return Enum.TryParse<Keys>(name, true, out var parsed) ? parsed : null;
    }

    [DllImport("user32.dll")]
    internal static extern short GetAsyncKeyState(int key);

    // ----- smoke test -----
    // With LUST_UI_SMOKE=1, a window clicks every button and ticks every box by itself
    // (three times over), runs its timer and presses a key, prints what its labels say
    // after each, then closes. For tests on machines where nobody is there to click.
    internal static readonly bool Smoke = Environment.GetEnvironmentVariable("LUST_UI_SMOKE") == "1";
    internal static bool TimerStarted;

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
            foreach (var x in w.All())
            {
                if (x.control?.Parent == null) continue;
                if (x is C_Ui_Label) Console.WriteLine("  " + x.f_text);
                if (x is C_Ui_Canvas c) Console.WriteLine($"  canvas: {c.Drawings} drawings");
            }
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
                else if (x is C_Ui_Canvas canvas) { Event(() => canvas.m_clicked(10, 10)); Dump("clicked the canvas"); }
            }
        }
        Event(() => w.m_keyPressed("Space"));
        Dump("pressed Space");
        if (TimerStarted)
        {
            for (int i = 0; i < 3; i++) Event(w.m_tick);
            Dump("ticked 3 times");
        }
    }

    internal static Control Create(C_Ui_Widget w)
    {
        w.control = w.Make();
        w.Made();
        w.Push();
        return w.control;
    }

    /// Runs Lust code for a click, a tick or a key: screen → fields, the code, fields → screen.
    internal static void Event(Action handler)
    {
        foreach (var win in Windows.ToList()) win.Pull();
        try { handler(); }
        catch (Exception e) { LustRt.Fail(e); }
        foreach (var win in Windows.ToList()) win.Push();
    }
}
