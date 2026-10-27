-- Lust's runtime for LustScript: the small library every Lust program carries along.
-- Plain Lua 5.1, so it runs in Garry's Mod (LuaJIT), Luau and standard Lua.
local L = {}
local floor, fmt, insert, remove = math.floor, string.format, table.insert, table.remove
local setmetatable, getmetatable, tostring, tonumber, type, error = setmetatable, getmetatable, tostring, tonumber, type, error
local ipairs = ipairs

-- ----- text -----

function L.ftext(v)
  if v ~= v then return "not a number" end
  if v == math.huge then return "infinity" end
  if v == -math.huge then return "-infinity" end
  if v == floor(v) and v > -1e15 and v < 1e15 then return fmt("%.1f", v) end
  local s = fmt("%.15g", v)
  if not s:find("[%.e]") then s = s .. ".0" end
  return s
end
function L.btext(v) if v then return "true" else return "false" end end

-- ----- vectors and rotations -----
-- A vec3 is {x, y, z} and a rotation {x, y, z, w} (a quaternion), each number kept as an f32,
-- as engines keep them. In Luau a vec3 is Luau's own vector instead, which keeps f32s itself.
-- They're never changed in place, so sharing one is like copying it.
-- Every step is worked out with Lua's numbers (doubles) and rounded to f32, the same way the
-- Rust runtime does it, so both give exactly the same numbers.
local frexp, ldexp, sqrt = math.frexp, math.ldexp, math.sqrt
local F32_MAX = 3.4028234663852886e38
local function f32(x)
  if x ~= x or x == 0 or x == math.huge or x == -math.huge then return x end
  local m, e = frexp(x)
  -- 24 bits for a normal f32; fewer for the tiniest ones.
  local bits = 24
  if e < -125 then bits = 24 - (-125 - e) end
  if bits < 0 then return 0 * x end
  local s = ldexp(m, bits)
  local f = floor(s)
  local d = s - f
  if d > 0.5 or (d == 0.5 and f % 2 ~= 0) then f = f + 1 end
  local y = ldexp(f, e - bits)
  if y > F32_MAX then return math.huge end
  if y < -F32_MAX then return -math.huge end
  return y
end
-- Luau keeps a vector's numbers as f32s, so making one rounds a number the same way, in C,
-- many times faster. It's used only when it gives exactly what f32 above gives.
do
  local vector = rawget(_G, "vector")
  local make = type(vector) == "table" and vector.create
  if make then
    local function fast(x) return make(x, 0, 0).x end
    local ok = pcall(function()
      for _, x in ipairs({ 0.1, -9.8, 1 / 3, 16777217, 3.4028235677973366e38, 3.4028234663852886e38, 1e-45, 7e-46, 1.5e-39, -0.0, 1e300 }) do
        if fast(x) ~= f32(x) then error("different") end
      end
    end)
    if ok then L.f32_slow = f32; f32 = fast end
  end
end
L.f32 = f32
local vec = function(x, y, z) return { x = f32(x), y = f32(y), z = f32(z) } end
local quat = function(x, y, z, w) return { x = f32(x), y = f32(y), z = f32(z), w = f32(w) } end
-- In Luau, a vec3 is a Luau vector, and a rotation's first three numbers are rounded with one.
L.vectors = "table"
do
  local vector = rawget(_G, "vector")
  local make = type(vector) == "table" and vector.create
  if make and L.f32_slow then
    L.vectors = "luau"
    vec = make
    quat = function(x, y, z, w)
      local r = make(x, y, z)
      return { x = r.x, y = r.y, z = r.z, w = f32(w) }
    end
  end
end
L.vec = vec
function L.vwith(v, f, n)
  local w = { x = v.x, y = v.y, z = v.z }
  w[f] = f32(n)
  return w
end
function L.vadd(a, b) return vec(a.x + b.x, a.y + b.y, a.z + b.z) end
function L.vsub(a, b) return vec(a.x - b.x, a.y - b.y, a.z - b.z) end
function L.vmul(a, n) return vec(a.x * n, a.y * n, a.z * n) end
function L.vdiv(a, n) return vec(a.x / n, a.y / n, a.z / n) end
function L.vneg(a) return { x = -a.x, y = -a.y, z = -a.z } end
function L.veq(a, b) return a.x == b.x and a.y == b.y and a.z == b.z end
function L.vlength(a) return sqrt(a.x * a.x + a.y * a.y + a.z * a.z) end
function L.vnormalized(a)
  local l = sqrt(a.x * a.x + a.y * a.y + a.z * a.z)
  if l == 0 then return a end
  return vec(a.x / l, a.y / l, a.z / l)
end
function L.vdot(a, b) return a.x * b.x + a.y * b.y + a.z * b.z end
function L.vcross(a, b) return vec(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x) end
function L.vdistance(a, b)
  local dx, dy, dz = a.x - b.x, a.y - b.y, a.z - b.z
  return sqrt(dx * dx + dy * dy + dz * dz)
end
function L.vlerp(a, b, t) return vec(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t) end
-- Luau vectors add, take away and turn negative in f32 themselves, which gives exactly what
-- working it out in doubles and rounding does (doubles have more than twice f32's bits).
-- Multiplying by a number still goes through doubles: Luau would round the number first.
if L.vectors == "luau" then
  local make = vec
  function L.vwith(v, f, n)
    if f == "x" then return make(n, v.y, v.z) end
    if f == "y" then return make(v.x, n, v.z) end
    return make(v.x, v.y, n)
  end
  function L.vadd(a, b) return a + b end
  function L.vsub(a, b) return a - b end
  function L.vneg(a) return -a end
end
local function q64(ax, ay, az, aw, bx, by, bz, bw)
  return aw * bx + ax * bw + ay * bz - az * by,
    aw * by - ax * bz + ay * bw + az * bx,
    aw * bz + ax * by - ay * bx + az * bw,
    aw * bw - ax * bx - ay * by - az * bz
end
local RAD = math.pi / 180
-- Degrees around x, y and z, turned in the order z, x, y (as Solis and glam's EulerRot::ZXY do).
function L.rot(dx, dy, dz)
  local hx, hy, hz = dx * RAD / 2, dy * RAD / 2, dz * RAD / 2
  -- q64 twice, written out: the same steps in the same order, without the calls.
  local ax, ay, az, aw = 0, 0, math.sin(hz), math.cos(hz)
  local bx, bw = math.sin(hx), math.cos(hx)
  local x = aw * bx + ax * bw + ay * 0 - az * 0
  local y = aw * 0 - ax * 0 + ay * bw + az * bx
  local z = aw * 0 + ax * 0 - ay * bx + az * bw
  local w = aw * bw - ax * bx - ay * 0 - az * 0
  local cy, cw = math.sin(hy), math.cos(hy)
  return quat(w * 0 + x * cw + y * 0 - z * cy,
    w * cy - x * 0 + y * cw + z * 0,
    w * 0 + x * cy - y * 0 + z * cw,
    w * cw - x * 0 - y * cy - z * 0)
end
-- L.rot(0, 0, dz), with sin(0) and cos(0) known: 0 and 1, exactly.
function L.rotz(dz)
  local hz = dz * RAD / 2
  local ax, ay, az, aw = 0, 0, math.sin(hz), math.cos(hz)
  local bx, bw = 0, 1
  local x = aw * bx + ax * bw + ay * 0 - az * 0
  local y = aw * 0 - ax * 0 + ay * bw + az * bx
  local z = aw * 0 + ax * 0 - ay * bx + az * bw
  local w = aw * bw - ax * bx - ay * 0 - az * 0
  local cy, cw = 0, 1
  return quat(w * 0 + x * cw + y * 0 - z * cy,
    w * cy - x * 0 + y * cw + z * 0,
    w * 0 + x * cy - y * 0 + z * cw,
    w * cw - x * 0 - y * cy - z * 0)
end
function L.qmul(a, b)
  local ax, ay, az, aw, bx, by, bz, bw = a.x, a.y, a.z, a.w, b.x, b.y, b.z, b.w
  return quat(aw * bx + ax * bw + ay * bz - az * by,
    aw * by - ax * bz + ay * bw + az * bx,
    aw * bz + ax * by - ay * bx + az * bw,
    aw * bw - ax * bx - ay * by - az * bz)
end
function L.qmulv(q, v)
  local tx = 2 * (q.y * v.z - q.z * v.y)
  local ty = 2 * (q.z * v.x - q.x * v.z)
  local tz = 2 * (q.x * v.y - q.y * v.x)
  return vec(v.x + q.w * tx + (q.y * tz - q.z * ty), v.y + q.w * ty + (q.z * tx - q.x * tz), v.z + q.w * tz + (q.x * ty - q.y * tx))
end
function L.qinverse(q) return { x = -q.x, y = -q.y, z = -q.z, w = q.w } end
function L.qeq(a, b) return a.x == b.x and a.y == b.y and a.z == b.z and a.w == b.w end
function L.qdegrees(q)
  local m21 = 2 * (q.y * q.z + q.x * q.w)
  if m21 > 1 then m21 = 1 elseif m21 < -1 then m21 = -1 end
  local x = math.asin(m21)
  local z = math.atan2(-(2 * (q.x * q.y - q.z * q.w)), 1 - 2 * (q.x * q.x + q.z * q.z))
  local y = math.atan2(-(2 * (q.x * q.z - q.y * q.w)), 1 - 2 * (q.x * q.x + q.y * q.y))
  return vec(x / RAD, y / RAD, z / RAD)
end
-- A vec3's numbers, to 7 digits, as an f32 holds.
local function f32text(x) return L.ftext(tonumber(fmt("%.7g", x)) + 0) end
-- What a host's Lua function gives back for a native method, made into Lust's own values: its
-- numbers rounded to f32 (a Luau vector works too, since it has x, y and z), and a plain list
-- given its length.
function L.hvec(v) return vec(v.x, v.y, v.z) end
function L.hrot(q) return quat(q.x, q.y, q.z, q.w) end
function L.harr(item)
  return function(t)
    local out = { n = t.n or #t }
    for i = 1, out.n do
      if item then out[i] = item(t[i]) else out[i] = t[i] end
    end
    return out
  end
end
-- An entity: the 13 letters and digits of its ID (Lua's numbers can't hold all 64 bits), or nil.
function L.etext(e) if e == nil then return "nothing" end return e end
function L.vtext(v) return "[" .. f32text(v.x) .. ", " .. f32text(v.y) .. ", " .. f32text(v.z) .. "]" end
function L.rtext(q) return L.vtext(L.qdegrees(q)) end
function L.qtext(v) return "\"" .. v .. "\"" end
function L.stext(v) return v end
-- Each line goes out straight away, as in a built program, even into a pipe or a file.
local flush = io and io.stdout and function() io.stdout:flush() end or function() end
function L.print(s) print(s) flush() end

function L.show(v)
  if v == nil then return "nothing" end
  if type(v) == "function" then return "hook" end
  local mt = getmetatable(v)
  if mt and mt.__name then return mt.__name end
  return tostring(v)
end

function L.ashow(f)
  return function(a)
    local parts = {}
    for i = 1, a.n do parts[i] = (a[i] == nil) and "nothing" or f(a[i]) end
    return "[" .. table.concat(parts, ", ") .. "]"
  end
end
function L.mshow(f)
  return function(m)
    local parts = {}
    for i, k in ipairs(m.keys) do parts[i] = "\"" .. k .. "\" " .. f(m.vals[k]) end
    return "[" .. table.concat(parts, ", ") .. "]"
  end
end
function L.rshow(fs)
  return function(r, cols)
    cols = cols or r.__cols
    local parts = {}
    for i, c in ipairs(cols) do parts[i] = (r[c] == nil) and "nothing" or fs[i](r[c]) end
    return "[" .. table.concat(parts, ", ") .. "]"
  end
end
function L.trshow(fs)
  return function(r)
    local parts = {}
    for i, c in ipairs(r.q.cols) do
      local v = L.r_get(r, c)
      parts[i] = (v == nil) and "nothing" or fs[i](v)
    end
    return "[" .. table.concat(parts, ", ") .. "]"
  end
end
function L.cshow(fs)
  local row = L.rshow(fs)
  return function(c)
    local parts = {}
    for i, r in ipairs(c.rows) do parts[i] = row(r, c.cols) end
    return "[" .. table.concat(parts, ", ") .. "]"
  end
end

-- Text length in letters, not bytes.
function L.slen(s)
  local _, n = s:gsub("[^\128-\191]", "")
  return n
end

-- ----- numbers -----

function L.idiv(a, b)
  if b == 0 then error("dividing a whole number by zero doesn't work.", 2) end
  local q = a / b
  if q >= 0 then return floor(q) else return -floor(-q) end
end
function L.imod(a, b)
  if b == 0 then error("the remainder after dividing by zero doesn't exist.", 2) end
  return math.fmod(a, b)
end
function L.fmod(a, b) return math.fmod(a, b) end
-- Programs (.lst) run by lust check that whole numbers stay ones Lua holds exactly: up to 2^53 - 1.
-- Built programs go up to 9223372036854775807. LustScript (.lsc) numbers are plain Lua numbers.
local EXACT = 9007199254740991
local function too_big(level)
  error("this whole number is too big for lust run, which holds ints exactly only up to 9007199254740991. "
    .. "A built program (lust build) holds them up to 9223372036854775807.", level + 1)
end
function L.I(x)
  if x > EXACT or x < -EXACT then too_big(2) end
  return x
end
-- Whole numbers are checked when they become text (print, .text()), which costs nothing extra, so
-- one that has grown past what Lua holds exactly never shows up wrong.
-- An int going into a sized number (int8 hp): it has to fit.
function L.narrow(v, lo, hi, name)
  if v < lo or v > hi then
    local a = name:sub(1, 1) == "u" and "a" or "an"
    error(L.itext(v) .. " doesn't fit in " .. a .. " " .. name .. ", which holds " .. lo .. " to " .. hi .. ".", 2)
  end
  return v
end
function L.jr_narrow(lo, hi, name)
  return function(v, at) return L.narrow(L.jr.int(v, at), lo, hi, name) end
end

function L.itext(v)
  if L.exact and (v > EXACT or v < -EXACT) then too_big(2) end
  return fmt("%d", v)
end
function L.toint(v)
  if v ~= v then error("this float isn't a number, so it can't be an int.", 2) end
  if v == math.huge or v == -math.huge then error(L.ftext(v) .. " is too big to be an int.", 2) end
  if L.exact and (v > EXACT or v < -EXACT) then too_big(2) end
  if v >= 0 then return floor(v) else return -floor(-v) end
end
function L.isnum(s)
  local v = tonumber((s:gsub("^%s+", ""):gsub("%s+$", "")))
  return v ~= nil and v == v and v ~= math.huge and v ~= -math.huge and not s:find("[xX]")
end
function L.sint(s)
  if not L.isnum(s) then error("\"" .. s .. "\" isn't a number. Check first with .isNumber().", 2) end
  return L.toint(tonumber(s))
end
function L.sfloat(s)
  if not L.isnum(s) then error("\"" .. s .. "\" isn't a number. Check first with .isNumber().", 2) end
  return tonumber(s) + 0.0
end
function L.lines(s)
  local out = { n = 0 }
  for line in (s .. "\n"):gmatch("(.-)\r?\n") do
    out.n = out.n + 1
    out[out.n] = line
  end
  if out.n > 0 and out[out.n] == "" and s:sub(-1) == "\n" then out[out.n] = nil; out.n = out.n - 1 end
  return out
end

-- Under lust, Random uses the very same generator as built programs, so LUST_SEED rolls the same
-- numbers both ways. Elsewhere (Garry's Mod, say) Lua's own is used, seeded from the time.
local RHOST = rawget(_G, "LUST_HOST")
local seeded = false
local function seed()
  if not seeded then
    seeded = true
    local s = os and os.getenv and tonumber(os.getenv("LUST_SEED") or "")
    if s then math.randomseed(s)
    elseif os and os.time then math.randomseed(os.time() + math.floor((os.clock and os.clock() or 0) * 1000003)) end
  end
end
function L.random(lo, hi)
  if lo > hi then error("Random's lowest number (" .. fmt("%d", lo) .. ") is bigger than its highest (" .. fmt("%d", hi) .. ").", 2) end
  if RHOST and RHOST.random then return RHOST.random(lo, hi) end
  seed()
  return math.random(lo, hi)
end
-- A decimal from lo up to (not quite) hi.
function L.frandom(lo, hi)
  if lo > hi then error("Random's lowest number (" .. L.ftext(lo) .. ") is bigger than its highest (" .. L.ftext(hi) .. ").", 2) end
  if RHOST and RHOST.frandom then return RHOST.frandom(lo, hi) end
  seed()
  return lo + math.random() * (hi - lo)
end

-- ----- arrays: { n = count, … }, counted from 0 in Lust -----

function L.arr(items, n) items.n = n; return items end
function L.acopy(a)
  local c = { n = a.n }
  for i = 1, a.n do c[i] = a[i] end
  return c
end
local function where(a, i, level)
  if a.n == 0 then error(fmt("the array is empty, so there's no item [%d].", i), level) end
  if i < 0 or i >= a.n or i ~= floor(i) then
    error(fmt("there's no item [%d]. The array has %d item%s, numbered 0 to %d.", i, a.n, a.n == 1 and "" or "s", a.n - 1), level)
  end
end
function L.aget(a, i) where(a, i, 3); return a[i + 1] end
function L.aset(a, i, v) where(a, i, 3); a[i + 1] = v end
function L.ainject(a, v) a.n = a.n + 1; a[a.n] = v end
function L.aremove(a, i)
  where(a, i, 3)
  remove(a, i + 1)
  a.n = a.n - 1
end

-- ----- maps: keys in the order they were added -----

function L.map(keys, vals, n)
  local m = { keys = {}, vals = {} }
  for i = 1, n do
    local k = keys[i]
    if m.vals[k] == nil then insert(m.keys, k) end
    m.vals[k] = vals[i]
  end
  return m
end
function L.keys(m)
  local c = {}
  for i, k in ipairs(m.keys) do c[i] = k end
  return c
end
function L.mget(m, k)
  local v = m.vals[k]
  if v == nil then
    local found = false
    for _, x in ipairs(m.keys) do if x == k then found = true break end end
    if not found then error("there's no key \"" .. k .. "\" in this map. Check first with :has(\"" .. k .. "\").", 2) end
  end
  return v
end
function L.mset(m, k, v)
  local found = false
  for _, x in ipairs(m.keys) do if x == k then found = true break end end
  if not found then insert(m.keys, k) end
  m.vals[k] = v
end
function L.mhas(m, k)
  for _, x in ipairs(m.keys) do if x == k then return true end end
  return false
end
function L.mremove(m, k)
  for i, x in ipairs(m.keys) do
    if x == k then remove(m.keys, i); m.vals[k] = nil; return end
  end
end

-- ----- cMaps: rows found by their key, the first column -----

local function keytext(c, k)
  if c.textkey then return "\"" .. k .. "\"" end
  return fmt("%d", k)
end
local function keys(c)
  local list = {}
  for i, r in ipairs(c.rows) do list[i] = keytext(c, r[c.key]) end
  if #list == 0 then return "The cMap is empty." end
  return "The " .. c.key .. "s are " .. table.concat(list, ", ") .. "."
end
local function addrow(c, values, level)
  local ncols = #c.cols
  local k
  local first = 1
  if values.n == ncols then
    k = values[1]
    first = 2
    if c.byKey[k] ~= nil then error("there's already a row with " .. c.key .. " " .. keytext(c, k) .. ".", level) end
  elseif c.maker then
    local tries = 0
    repeat
      k = c.maker()
      tries = tries + 1
      if tries > 100 then error("the key-maker keeps making " .. c.key .. "s that are already taken.", level) end
    until c.byKey[k] == nil
  elseif c.textkey then
    error("this cMap's key (" .. c.key .. ") is text, so each new row needs it first.", level)
  else
    k = c.next
  end
  local row = { __cols = c.cols }
  row[c.key] = k
  for i = 2, ncols do row[c.cols[i]] = values[first + i - 2] end
  insert(c.rows, row)
  c.byKey[k] = row
  if not c.textkey and k >= c.next then c.next = k + 1 end
end
function L.carr(cols, textkey, rows, maker)
  local c = { cols = cols, key = cols[1], textkey = textkey, rows = {}, byKey = {}, next = 1 }
  if maker ~= nil then L.cmaker(c, maker) end
  for _, r in ipairs(rows) do addrow(c, r, 4) end
  return c
end
-- Gives a cMap its key-maker, and gives the cMap back.
function L.cmaker(c, maker)
  c.maker = maker or function() error("the key-maker is nothing, so it can't make keys.", 0) end
  return c
end
function L.rows(c)
  local copy = {}
  for i, r in ipairs(c.rows) do copy[i] = r end
  return copy
end
function L.cget(c, k)
  local r = c.byKey[k]
  if not r then error("there's no row with " .. c.key .. " " .. keytext(c, k) .. ". " .. keys(c), 2) end
  return r
end
function L.cinject(c, values) addrow(c, values, 3) end
-- Puts a list in order by one value each item has. Items with the same value keep their order.
local function stable(list, key)
  local d = {}
  for i, x in ipairs(list) do d[i] = { key(x), i, x } end
  table.sort(d, function(a, b)
    if a[1] ~= b[1] then return a[1] < b[1] end
    return a[2] < b[2]
  end)
  for i, x in ipairs(d) do list[i] = x[3] end
end
function L.csort(c, col) stable(c.rows, function(r) return r[col] end) end
function L.creverse(c)
  local n = #c.rows
  for i = 1, floor(n / 2) do c.rows[i], c.rows[n - i + 1] = c.rows[n - i + 1], c.rows[i] end
end
function L.asort_by(a, field)
  local t = {}
  for i = 1, a.n do t[i] = a[i] end
  stable(t, function(o)
    if o == nil then error("this is nothing, so it has no fields or methods. Check first: is it !== nothing?", 0) end
    return o[field]
  end)
  for i = 1, a.n do a[i] = t[i] end
end
function L.chas(c, k) return c.byKey[k] ~= nil end
function L.cremove(c, k)
  if not c.byKey[k] then error("there's no row with " .. c.key .. " " .. keytext(c, k) .. " to remove. " .. keys(c), 2) end
  c.byKey[k] = nil
  for i, r in ipairs(c.rows) do
    if r[c.key] == k then remove(c.rows, i) break end
  end
end

-- ----- states: options are their full names, like "ActiveGame.Combat" -----

function L.state(name, list) return { name = name, list = list } end
-- Is a the option b, or inside it?
-- == on state options: the same option, or one inside the other, whichever side it's written on.
local function inside(a, b) return a:sub(1, #b + 1) == b .. "." end
function L.sin(a, b)
  return a == b or (a ~= nil and b ~= nil and (inside(a, b) or inside(b, a)))
end
-- match's when: the option a is b, or one inside b (not the other way round).
function L.within(a, b) return a == b or inside(a, b) end

-- ----- native methods: their bodies are outside Lust -----

-- The program using Lust gives native methods' bodies as Lua functions, by the method's name
-- (Class_method for a class's): in a table called LUST_NATIVES, or as globals. A method on its
-- own has the same global name as Lust's own function for it, so for those LUST_NATIVES is the
-- place. lust run has no such program, so it says where the body lives.
L.wrappers = setmetatable({}, { __mode = "k" })
local function host_fn(global)
  local natives = rawget(_G, "LUST_NATIVES")
  local f = type(natives) == "table" and natives[global] or nil
  if f == nil then
    f = rawget(_G, global)
    if L.wrappers[f] then f = nil end
  end
  if type(f) == "function" then return f end
  return nil
end
function L.native(global, shown, ...)
  local f = host_fn(global)
  if not f then
    if rawget(_G, "LUST_HOST") then
      error(shown .. " is native: its body is in the Rust program that uses this Lust as a library (lust build --library), so it can't run on its own.", 3)
    end
    error(shown .. " is native, but there's no Lua function called " .. global .. " to run it.", 3)
  end
  return f(...)
end

-- ----- events -----

-- send Hit(…) to target: the engine's Lua offers send_event(to, name, values), and queues the
-- event for the entity's components. values is a list with its count in n.
function L.send(to, name, values)
  local f = host_fn("send_event")
  if not f then
    if rawget(_G, "LUST_HOST") then
      error("send " .. name .. "(…) goes to the engine using this Lust as a library (lust build --library), so it can't run on its own.", 2)
    end
    error("send " .. name .. "(…) needs the engine's Lua to offer a function called send_event.", 2)
  end
  f(to, name, values)
end

-- ----- for an engine -----

local function same_schema(a, b)
  a, b = a or {}, b or {}
  if #a ~= #b then return false end
  for i = 1, #a do
    local x, y = a[i], b[i]
    if x[1] ~= y[1] or x[3] ~= y[3] then return false end
    local ox, oy = x[4] or {}, y[4] or {}
    if #ox ~= #oy then return false end
    for j = 1, #ox do if ox[j] ~= oy[j] then return false end end
  end
  return true
end

-- What an engine's chunk gives back: its components, the events, and how to make, run and swap
-- them. components: name -> class. events: name -> { { key, name, kind }, … }.
function L.engine(version, components, events)
  -- vectors: "luau" when a vec3 is a Luau vector, "table" in plain Lua.
  local api = { version = version, components = components, events = events, vectors = L.vectors }
  -- A new component with its fields' starting values, on an entity.
  function api.new(name, me)
    local c = components[name]
    if c == nil then error("there's no component called " .. tostring(name) .. ".", 2) end
    local o = L.new(c)
    o.me = me
    return o
  end
  -- Gives a component made by an earlier chunk this chunk's code, keeping what it can, and
  -- gives back notes on what started over. nil and a note if this chunk has no such component.
  function api.swap(o)
    local old = getmetatable(o)
    local name = old and old.__name
    local c = name and components[name]
    if c == nil then return nil, { tostring(name) .. " isn't in the code any more." } end
    return L.swap(o, c)
  end
  -- What changed since an earlier chunk, for an inspector: components whose fields changed
  -- (names, kinds, a state's options or their order), ones removed and ones added, each sorted.
  function api.changes(before)
    local changed, removed, added = {}, {}, {}
    for name, c in pairs(before.components) do
      local now = components[name]
      if now == nil then
        removed[#removed + 1] = name
      elseif not same_schema(c.__schema, now.__schema) then
        changed[#changed + 1] = name
      end
    end
    for name in pairs(components) do
      if before.components[name] == nil then added[#added + 1] = name end
    end
    table.sort(changed)
    table.sort(removed)
    table.sort(added)
    return { changed = changed, removed = removed, added = added }
  end
  return api
end

local function shown_value(v, kind)
  if v == nil then return "nothing" end
  if kind == "vec3" then return L.vtext(v) end
  if kind == "rotation" then return L.rtext(v) end
  if kind == "string" then return '"' .. v .. '"' end
  return tostring(v)
end

-- Fields are kept by name. A field that's gone goes; a new one starts at its starting value; one
-- whose kind changed starts at its starting value, with a note. The entity it's on stays.
function L.swap(o, c)
  local old = getmetatable(o) or {}
  local before = {}
  for _, f in ipairs(old.__schema or {}) do before[f[1]] = f end
  local saved = {}
  for k, v in pairs(o) do saved[k] = v end
  for k in pairs(saved) do o[k] = nil end
  setmetatable(o, c)
  c.__fields(o)
  o.me = saved.me
  local notes = {}
  for _, f in ipairs(c.__schema or {}) do
    local key, name, kind = f[1], f[2], f[3]
    local was = before[key]
    if was then
      local v = saved[key]
      if was[3] ~= kind then
        notes[#notes + 1] = c.__name .. "'s " .. name .. " is " .. kind .. " now, not " .. was[3] .. ", so it starts at " .. shown_value(o[key], kind) .. "."
      elseif kind == "state" and v ~= nil then
        local found = false
        for _, opt in ipairs(f[4]) do if opt == v then found = true end end
        if found then
          o[key] = v
        else
          notes[#notes + 1] = c.__name .. "'s " .. name .. " was " .. v .. ", which isn't one of its options now, so it starts at " .. shown_value(o[key], kind) .. "."
        end
      else
        o[key] = v
      end
    end
  end
  return o, notes
end

-- ----- classes -----

function L.class(name, parent)
  local c = { __name = name, __parent = parent }
  c.__index = c
  if parent then setmetatable(c, { __index = parent }) end
  return c
end
function L.new(c, ...)
  local o = setmetatable({}, c)
  c.__fields(o)
  local n = select("#", ...)
  for i = 1, n, 2 do
    local k, v = select(i, ...)
    o[k] = v
  end
  return o
end
function L.like(o, c)
  if o == nil then return false end
  local mt = getmetatable(o)
  while mt do
    if mt == c then return true end
    mt = mt.__parent
  end
  return false
end
function L.bind(o, name)
  if o == nil then error("this is nothing, so it has no methods.", 2) end
  return function(...) return o[name](o, ...) end
end

-- ----- things made in a state end with it -----
-- owner -> { thing = the owner's state option when it was made }. Weak, so nothing is held on to.
local owners = setmetatable({}, { __mode = "k" })
function L.own(owner, thing, opt)
  if thing == nil or opt == nil then return thing end
  local list = owners[owner]
  if not list then
    list = setmetatable({}, { __mode = "k" })
    owners[owner] = list
  end
  list[thing] = opt
  return thing
end
function L.keep(thing)
  if thing == nil then return end
  for _, list in pairs(owners) do list[thing] = nil end
end
-- The thing is gone: its fields are let go, and using it says why.
local function finish(thing, msg)
  local name = (getmetatable(thing) or {}).__name
  for k in pairs(thing) do thing[k] = nil end
  local function gone() error(msg, 2) end
  setmetatable(thing, { __name = name, __index = gone, __newindex = gone, __ended = msg })
  local list = owners[thing]
  owners[thing] = nil
  if list then
    for t in pairs(list) do finish(t, msg) end
  end
end
L.finish = finish
function L.ended(state)
  return "this was made during " .. state .. ", and ended with it. To hold onto something made in a state, keep it, like keep sword in player.bag;"
end
function L.setstate(o, new)
  if o == nil then error("this is nothing, so it has no fields or methods. Check first: is it !== nothing?", 2) end
  o.state = new
  local list = owners[o]
  if not list then return end
  local over = {}
  for thing, opt in pairs(list) do
    if not L.sin(new, opt) then over[#over + 1] = { thing, opt } end
  end
  for _, x in ipairs(over) do
    list[x[1]] = nil
    finish(x[1], L.ended(x[2]))
  end
end

-- ----- nodes: objects that open and close -----

local function shut(n, class)
  for k in pairs(n) do n[k] = nil end
  local msg = class.__node .. " is closed. Open it first: " .. class.__node .. ":open();"
  local function gone() error(msg, 2) end
  setmetatable(n, { __index = gone, __newindex = gone, __name = class.__name, __class = class, __closed = true })
end
function L.node(class, name)
  class.__node = name
  local n = {}
  shut(n, class)
  return n
end
function L.node_open(n)
  local mt = getmetatable(n)
  if not mt.__closed then error(mt.__node .. " is already open.", 2) end
  local class = mt.__class
  setmetatable(n, class)
  class.__fields(n)
  if class.onOpen then n:onOpen() end
end
function L.node_close(n)
  local class = getmetatable(n)
  if class.__closed then error(class.__class.__node .. " is already closed.", 2) end
  if class.onClose then n:onClose() end
  local msg = "this was made in " .. class.__node .. ", and ended when it closed. To hold onto something made in a node, keep it."
  local list = owners[n]
  owners[n] = nil
  if list then
    for t in pairs(list) do finish(t, msg) end
  end
  shut(n, class)
end
function L.node_isopen(n) return not getmetatable(n).__closed end

-- ----- built-ins that need the outside world -----

function L.ask(q)
  if not io or not io.read then error("ask needs a console, and this program doesn't have one.", 2) end
  io.write(q .. " ")
  io.flush()
  return io.read("*l") or ""
end
function L.wait(s)
  if s < 0 or s ~= s then error("can't wait " .. L.ftext(s) .. " seconds.", 2) end
  if not os or not os.clock then error("wait isn't available here. Use the host program's timers.", 2) end
  local stop = os.clock() + s
  while os.clock() < stop do end
end

-- ----- the standard library -----
-- The same functions, with the same messages, are in native_rt.rs for built programs.

local host = rawget(_G, "LUST_HOST")

-- Text, counted in letters, not bytes.
local function letters(s)
  local list = {}
  for ch in s:gmatch("[%z\1-\127\194-\244][\128-\191]*") do list[#list + 1] = ch end
  return list
end
local function plain(s) return (s:gsub("[%^%$%(%)%%%.%[%]%*%+%-%?]", "%%%0")) end
-- a–z and the accented letters of Latin-1 (à to þ): the same in Lua and in built programs.
function L.text_upper(s)
  s = s:upper()
  return (s:gsub("\195([\160-\190])", function(c)
    if c == "\183" then return "\195" .. c end
    return "\195" .. string.char(c:byte() - 32)
  end))
end
function L.text_lower(s)
  s = s:lower()
  return (s:gsub("\195([\128-\158])", function(c)
    if c == "\151" then return "\195" .. c end
    return "\195" .. string.char(c:byte() + 32)
  end))
end
function L.text_trim(s) return (s:gsub("^[ \t\r\n]+", ""):gsub("[ \t\r\n]+$", "")) end
function L.text_contains(s, x) return s:find(x, 1, true) ~= nil end
function L.text_startsWith(s, x) return s:sub(1, #x) == x end
function L.text_endsWith(s, x) return x == "" or s:sub(-#x) == x end
function L.text_find(s, x)
  local i = s:find(x, 1, true)
  if not i then return -1 end
  return L.slen(s:sub(1, i - 1))
end
function L.text_replace(s, old, new)
  if old == "" then error("replace needs some text to look for.", 2) end
  return (s:gsub(plain(old), (new:gsub("%%", "%%%%"))))
end
function L.text_split(s, sep)
  local out = { n = 0 }
  if sep == "" then
    for _, ch in ipairs(letters(s)) do out.n = out.n + 1; out[out.n] = ch end
    return out
  end
  local start = 1
  while true do
    local i, j = s:find(sep, start, true)
    out.n = out.n + 1
    if not i then out[out.n] = s:sub(start); return out end
    out[out.n] = s:sub(start, i - 1)
    start = j + 1
  end
end
function L.text_slice(s, from, to)
  local list = letters(s)
  if from < 0 or to > #list or from > to then
    error(fmt("can't slice from %d to %d: the text has %d letter%s.", from, to, #list, #list == 1 and "" or "s"), 2)
  end
  return table.concat(list, "", from + 1, to)
end

-- Arrays.
function L.array_join(a, sep) return table.concat(a, sep, 1, a.n) end
function L.array_has(a, x)
  for i = 1, a.n do if a[i] == x then return true end end
  return false
end
function L.array_find(a, x)
  for i = 1, a.n do if a[i] == x then return i - 1 end end
  return -1
end
function L.array_sort(a)
  local t = {}
  for i = 1, a.n do t[i] = a[i] end
  table.sort(t, function(x, y) return x < y end)
  for i = 1, a.n do a[i] = t[i] end
end
function L.array_reverse(a)
  local n = a.n
  for i = 1, floor(n / 2) do a[i], a[n - i + 1] = a[n - i + 1], a[i] end
end

-- Maths.
function L.maths_abs_int(x) return math.abs(x) end
L.maths_abs_float = L.maths_abs_int
function L.maths_min_int(a, b) if b < a then return b end return a end
L.maths_min_float = L.maths_min_int
function L.maths_max_int(a, b) if b > a then return b end return a end
L.maths_max_float = L.maths_max_int
function L.maths_clamp_int(x, lo, hi)
  if lo > hi then error("clamp's low (" .. tostring(lo) .. ") is bigger than its high (" .. tostring(hi) .. ").", 2) end
  if x < lo then return lo elseif x > hi then return hi end
  return x
end
function L.maths_clamp_float(x, lo, hi)
  if lo > hi then error("clamp's low (" .. L.ftext(lo) .. ") is bigger than its high (" .. L.ftext(hi) .. ").", 2) end
  if x < lo then return lo elseif x > hi then return hi end
  return x
end
function L.maths_round(x)
  if x >= 0 then return L.toint(floor(x + 0.5)) end
  return L.toint(-floor(-x + 0.5))
end
function L.maths_floor(x) return L.toint(floor(x)) end
function L.maths_ceil(x) return L.toint(math.ceil(x)) end
function L.maths_sqrt(x)
  if x < 0 then error("there's no square root of a number below zero (" .. L.ftext(x) .. ").", 2) end
  return math.sqrt(x)
end
function L.maths_power(a, b) return a ^ b end
L.maths_sin, L.maths_cos, L.maths_tan = math.sin, math.cos, math.tan
function L.maths_pi() return math.pi end

-- Files.
local function needs(what)
  error(what .. " needs a full computer to run on, and isn't available here.", 3)
end
function L.files_read(path)
  if not io or not io.open then needs("Files:read") end
  local f, e = io.open(path, "rb")
  if not f then
    if tostring(e):find("No such file") then error("can't read " .. path .. ": there's no such file.", 2) end
    error("can't read " .. path .. ".", 2)
  end
  local s = f:read("*a")
  f:close()
  return s
end
local function put(path, text, mode, what)
  if not io or not io.open then needs("Files:" .. what) end
  local f = io.open(path, mode)
  if not f then error("can't " .. what .. " " .. path .. ".", 3) end
  f:write(text)
  f:close()
end
function L.files_write(path, text) put(path, text, "wb", "write") end
function L.files_append(path, text) put(path, text, "ab", "append") end
function L.files_exists(path)
  if host then return host.exists(path) end
  if not io or not io.open then needs("Files:exists") end
  local f = io.open(path, "rb")
  if f then f:close() return true end
  return false
end
function L.files_remove(path)
  if host then
    local e = host.remove(path)
    if e then error(e, 2) end
    return
  end
  if not os or not os.remove then needs("Files:remove") end
  local ok = os.remove(path)
  if not ok then error("can't remove " .. path .. ".", 2) end
end
function L.files_list(folder)
  if not host then needs("Files:list") end
  local names, e = host.list(folder)
  if not names then error(e, 2) end
  local out = { n = #names }
  for i, n in ipairs(names) do out[i] = n end
  return out
end
function L.files_chooseOpen(title, types)
  if not host or not host.choose_open then needs("Files:chooseOpen") end
  local list = {}
  for i = 1, (types.n or #types) do list[i] = types[i] end
  return host.choose_open(title, list)
end
function L.files_chooseSave(title, name)
  if not host or not host.choose_save then needs("Files:chooseSave") end
  return host.choose_save(title, name)
end
function L.files_chooseFolder(title)
  if not host or not host.choose_folder then needs("Files:chooseFolder") end
  return host.choose_folder(title)
end
function L.files_makeFolder(folder)
  if not host then needs("Files:makeFolder") end
  local e = host.mkdir(folder)
  if e then error(e, 2) end
end

-- Text patterns and the web client come from lust, which runs them with the same code as built programs.
local function hostcall(what, name, ...)
  if not host or not host[name] then needs(what) end
  local r, e = host[name](...)
  if r == nil then error(e, 3) end
  return r
end
local function strings(list)
  local out = { n = #list }
  for i, s in ipairs(list) do out[i] = s end
  return out
end
function L.text_matches(s, p) return hostcall(":matches", "pat_matches", s, p) end
function L.text_findAll(s, p) return strings(hostcall(":findAll", "pat_find_all", s, p)) end
function L.text_capture(s, p) return strings(hostcall(":capture", "pat_capture", s, p)) end
function L.text_replaceAll(s, p, w) return hostcall(":replaceAll", "pat_replace_all", s, p, w) end
function L.web_get(url) return hostcall("Web:get", "http", url) end
function L.web_post(url, body) return hostcall("Web:post", "http", url, body) end

-- Dates are text like "2026-10-21". Days are counted from 1 January 1970.
local function days_from(y, m, d)
  if m <= 2 then y = y - 1 end
  local era = floor(y / 400)
  local yoe = y - era * 400
  local mp = m > 2 and m - 3 or m + 9
  local doy = floor((153 * mp + 2) / 5) + d - 1
  local doe = yoe * 365 + floor(yoe / 4) - floor(yoe / 100) + doy
  return era * 146097 + doe - 719468
end
local function civil(z)
  z = z + 719468
  local era = floor(z / 146097)
  local doe = z - era * 146097
  local yoe = floor((doe - floor(doe / 1460) + floor(doe / 36524) - floor(doe / 146096)) / 365)
  local y = yoe + era * 400
  local doy = doe - (365 * yoe + floor(yoe / 4) - floor(yoe / 100))
  local mp = floor((5 * doy + 2) / 153)
  local d = doy - floor((153 * mp + 2) / 5) + 1
  local m = mp < 10 and mp + 3 or mp - 9
  if m <= 2 then y = y + 1 end
  return y, m, d
end
local function dayof(s)
  local y, m, d = s:match("^(%d%d%d%d)%-(%d%d)%-(%d%d)$")
  if not y then return nil end
  y, m, d = tonumber(y), tonumber(m), tonumber(d)
  local z = days_from(y, m, d)
  local y2, m2, d2 = civil(z)
  if y2 ~= y or m2 ~= m or d2 ~= d then return nil end
  return z
end
local function needday(s)
  local z = dayof(s)
  if not z then error("\"" .. s .. "\" isn't a date. Dates are written like \"2026-10-21\".", 3) end
  return z
end
local function datetext(z)
  local y, m, d = civil(z)
  return fmt("%04d-%02d-%02d", y, m, d)
end
function L.time_isDate(s) return dayof(s) ~= nil end
function L.time_addDays(s, n) return datetext(needday(s) + n) end
function L.time_daysBetween(a, b) local x = needday(a); return needday(b) - x end
local weekdays = { "Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday" }
function L.time_weekday(s) return weekdays[needday(s) % 7 + 1] end

-- Time.
local started = os and os.clock and os.clock() or 0
function L.time_now()
  if host then return host.now() end
  if os and os.time then return os.time() + 0.0 end
  needs("Time:now")
end
function L.time_clock()
  if host then return host.clock() end
  return os.clock() - started
end
function L.time_date() return os.date("%Y-%m-%d") end
function L.time_time() return os.date("%H:%M:%S") end

-- ----- JSON -----

L.NULL = setmetatable({}, { __tostring = function() return "null" end })

local function jfail(text, i, what)
  local line = 1
  for _ in text:sub(1, i - 1):gmatch("\n") do line = line + 1 end
  error(fmt("this isn't valid JSON (line %d): %s.", line, what), 0)
end
local function junexpected(text, i)
  if i > #text then jfail(text, i, "the text ends too early") end
  local c = text:sub(i, i)
  if c:byte() >= 128 then jfail(text, i, "a letter is out of place") end
  jfail(text, i, "\"" .. c .. "\" is out of place")
end
local function skip(text, i)
  return text:find("[^ \t\r\n]", i) or (#text + 1)
end
local function utf8char(n)
  if n < 128 then return string.char(n) end
  if n < 2048 then return string.char(192 + floor(n / 64), 128 + n % 64) end
  if n < 65536 then return string.char(224 + floor(n / 4096), 128 + floor(n / 64) % 64, 128 + n % 64) end
  return string.char(240 + floor(n / 262144), 128 + floor(n / 4096) % 64, 128 + floor(n / 64) % 64, 128 + n % 64)
end
local jvalue
local escapes = { ['"'] = '"', ["\\"] = "\\", ["/"] = "/", b = "\b", f = "\f", n = "\n", r = "\r", t = "\t" }
local function jstring(text, i)
  -- text:sub(i, i) is the opening quote
  local parts = {}
  local j = i + 1
  while true do
    local c = text:sub(j, j)
    if c == "" then jfail(text, j, "the text ends too early") end
    if c == '"' then return table.concat(parts), j + 1 end
    if c == "\\" then
      local e = text:sub(j + 1, j + 1)
      if escapes[e] then
        parts[#parts + 1] = escapes[e]
        j = j + 2
      elseif e == "u" then
        local hex = text:sub(j + 2, j + 5)
        if not hex:find("^%x%x%x%x$") then jfail(text, j, "a \\ in text is followed by something JSON doesn't allow") end
        local n = tonumber(hex, 16)
        j = j + 6
        if n >= 0xD800 and n <= 0xDBFF and text:sub(j, j + 1) == "\\u" then
          local lo = tonumber(text:sub(j + 2, j + 5), 16)
          if lo and lo >= 0xDC00 and lo <= 0xDFFF then
            n = 0x10000 + (n - 0xD800) * 1024 + (lo - 0xDC00)
            j = j + 6
          end
        end
        parts[#parts + 1] = utf8char(n)
      else
        jfail(text, j, "a \\ in text is followed by something JSON doesn't allow")
      end
    elseif c:byte() < 32 then
      jfail(text, j, "text can't have a line break or tab inside it; write \\n or \\t")
    else
      parts[#parts + 1] = c
      j = j + 1
    end
  end
end
jvalue = function(text, i)
  i = skip(text, i)
  local c = text:sub(i, i)
  if c == "{" then
    local o = { __obj = true, keys = {}, vals = {} }
    i = skip(text, i + 1)
    if text:sub(i, i) == "}" then return o, i + 1 end
    while true do
      i = skip(text, i)
      if text:sub(i, i) ~= '"' then junexpected(text, i) end
      local k
      k, i = jstring(text, i)
      i = skip(text, i)
      if text:sub(i, i) ~= ":" then junexpected(text, i) end
      local v
      v, i = jvalue(text, i + 1)
      if o.vals[k] == nil then o.keys[#o.keys + 1] = k end
      o.vals[k] = v
      i = skip(text, i)
      local d = text:sub(i, i)
      if d == "}" then return o, i + 1 end
      if d ~= "," then junexpected(text, i) end
      i = i + 1
    end
  elseif c == "[" then
    local a = { __arr = true, n = 0 }
    i = skip(text, i + 1)
    if text:sub(i, i) == "]" then return a, i + 1 end
    while true do
      local v
      v, i = jvalue(text, i)
      a.n = a.n + 1
      a[a.n] = v
      i = skip(text, i)
      local d = text:sub(i, i)
      if d == "]" then return a, i + 1 end
      if d ~= "," then junexpected(text, i) end
      i = i + 1
    end
  elseif c == '"' then
    return jstring(text, i)
  elseif text:sub(i, i + 3) == "true" then
    return true, i + 4
  elseif text:sub(i, i + 4) == "false" then
    return false, i + 5
  elseif text:sub(i, i + 3) == "null" then
    return L.NULL, i + 4
  else
    local num = text:match("^-?%d+%.?%d*[eE]?[-+]?%d*", i)
    if not num or num == "" or num == "-" then junexpected(text, i) end
    if num:find("^-?0%d") then junexpected(text, i + (num:sub(1, 1) == "-" and 2 or 1)) end
    if num:find("%.$") or num:find("%.[eE]") or num:find("[eE][-+]?$") then junexpected(text, i + #num) end
    return tonumber(num), i + #num
  end
end
local function parse(text)
  local v, i = jvalue(text, 1)
  i = skip(text, i)
  if i <= #text then junexpected(text, i) end
  return v
end

local function kind(v)
  if v == L.NULL then return "nothing (null)" end
  local t = type(v)
  if t == "boolean" then return "a bool" end
  if t == "number" then return "a number" end
  if t == "string" then return "text" end
  if v.__arr then return "an array" end
  return "an object"
end
local function misfit(at, want, v)
  local where = at == "" and "the JSON" or ("the JSON at " .. at)
  error(where .. " should be " .. want .. ", but it's " .. kind(v) .. ".", 0)
end
local function child(at, k) if at == "" then return k end return at .. "." .. k end

L.jr = {}
function L.jr.int(v, at)
  if type(v) ~= "number" then misfit(at, "an int", v) end
  if v ~= floor(v) or v ~= v or v > 9007199254740992 or v < -9007199254740992 then
    local where = at == "" and "the JSON" or ("the JSON at " .. at)
    error(where .. " should be an int, but it's a number with a fraction.", 0)
  end
  return v
end
function L.jr.float(v, at)
  if type(v) ~= "number" then misfit(at, "a number", v) end
  return v + 0.0
end
function L.jr.string(v, at)
  if type(v) ~= "string" then misfit(at, "text", v) end
  return v
end
function L.jr.truth(v, at)
  if type(v) ~= "boolean" then misfit(at, "a bool (true or false)", v) end
  return v
end
function L.jr_state(opts)
  return function(v, at)
    for _, o in ipairs(opts) do if v == o then return v end end
    local where = at == "" and "the JSON" or ("the JSON at " .. at)
    local shown = type(v) == "string" and ("\"" .. v .. "\"") or kind(v)
    error(where .. " should be one of " .. table.concat(opts, ", ") .. ", but it's " .. shown .. ".", 0)
  end
end
function L.jr_array(f)
  return function(v, at)
    if type(v) ~= "table" or not v.__arr then misfit(at, "an array", v) end
    local out = { n = v.n }
    for i = 1, v.n do out[i] = f(v[i], fmt("%s[%d]", at, i - 1)) end
    return out
  end
end
function L.jr_map(f)
  return function(v, at)
    if type(v) ~= "table" or not v.__obj then misfit(at, "an object", v) end
    local m = { keys = {}, vals = {} }
    for _, k in ipairs(v.keys) do L.mset(m, k, f(v.vals[k], child(at, k))) end
    return m
  end
end
function L.jr_cmap(cols, textkey, fs)
  return function(v, at)
    if type(v) ~= "table" or not v.__arr then misfit(at, "an array", v) end
    local rows = {}
    for i = 1, v.n do
      local r = v[i]
      local rat = fmt("%s[%d]", at, i - 1)
      if type(r) ~= "table" or not r.__obj then misfit(rat, "an object", r) end
      local values = { n = 0 }
      for c, name in ipairs(cols) do
        local x = r.vals[name]
        if x == nil then
          if c > 1 then error("the JSON at " .. rat .. " is missing " .. name .. ".", 0) end
        else
          values.n = values.n + 1
          values[values.n] = fs[c](x, child(rat, name))
        end
      end
      rows[i] = values
    end
    return L.carr(cols, textkey, rows)
  end
end
-- A thing made from a class: its fields that are in the JSON, the rest keep their starting values.
function L.jr_fields(v, at, class, names, fs, required)
  if v == L.NULL then return nil end
  if type(v) ~= "table" or not v.__obj then misfit(at, "an object", v) end
  for _, r in ipairs(required or {}) do
    if v.vals[r] == nil then
      error((at == "" and "the JSON" or ("the JSON at " .. at)) .. " is missing " .. r .. ".", 0)
    end
  end
  local o = L.new(class)
  for i, name in ipairs(names) do
    local x = v.vals[name]
    if x ~= nil then o[name] = fs[i](x, child(at, name)) end
  end
  return o
end
-- Errors from deep inside are passed on as if from the Lust line that called Json.
function L.json_read(text, f)
  local ok, r = pcall(function() return f(parse(text), "") end)
  if not ok then error(r, 2) end
  return r
end

local function jquote(s)
  return '"' .. s:gsub('[%c"\\]', function(c)
    if c == '"' then return '\\"' end
    if c == "\\" then return "\\\\" end
    if c == "\n" then return "\\n" end
    if c == "\r" then return "\\r" end
    if c == "\t" then return "\\t" end
    if c == "\b" then return "\\b" end
    if c == "\f" then return "\\f" end
    if c:byte() == 127 then return c end
    return fmt("\\u%04x", c:byte())
  end) .. '"'
end
local function deeper(d) if d then return d + 1 end end
local function block(open, close, parts, d)
  if #parts == 0 then return open .. close end
  if not d then return open .. table.concat(parts, ",") .. close end
  local pad = string.rep("  ", d + 1)
  return open .. "\n" .. pad .. table.concat(parts, ",\n" .. pad) .. "\n" .. string.rep("  ", d) .. close
end
local function pair(k, v, d) if d then return jquote(k) .. ": " .. v end return jquote(k) .. ":" .. v end
L.jw = {}
function L.jw.int(v) return fmt("%d", v) end
function L.jw.float(v)
  if v ~= v or v == math.huge or v == -math.huge then error("JSON can't hold " .. L.ftext(v) .. ".", 0) end
  return L.ftext(v)
end
function L.jw.string(v) return jquote(v) end
function L.jw.truth(v) return L.btext(v) end
function L.jw_array(f)
  return function(a, d)
    local parts = {}
    for i = 1, a.n do parts[i] = f(a[i], deeper(d)) end
    return block("[", "]", parts, d)
  end
end
function L.jw_map(f)
  return function(m, d)
    local parts = {}
    for i, k in ipairs(m.keys) do parts[i] = pair(k, f(m.vals[k], deeper(d)), deeper(d)) end
    return block("{", "}", parts, d)
  end
end
function L.jw_cmap(fs)
  return function(c, d)
    local rows = {}
    local inner = deeper(d)
    for i, r in ipairs(c.rows) do
      local parts = {}
      for j, name in ipairs(c.cols) do parts[j] = pair(name, fs[j](r[name], deeper(inner)), deeper(inner)) end
      rows[i] = block("{", "}", parts, inner)
    end
    return block("[", "]", rows, d)
  end
end
local jdepth = 0
function L.jw_obj(o, d)
  if o == nil then return "null" end
  jdepth = jdepth + 1
  if jdepth > 500 then error("this holds itself (somewhere inside it, something points back to it), so it can't be JSON.", 0) end
  local s = getmetatable(o).__jw(o, d)
  jdepth = jdepth - 1
  return s
end
function L.jw_fields(o, d, names, fs)
  local parts = {}
  for i, name in ipairs(names) do
    local v = o[name]
    parts[i] = pair(name, v == nil and "null" or fs[i](v, deeper(d)), deeper(d))
  end
  return block("{", "}", parts, d)
end
function L.json_write(v, f, pretty)
  jdepth = 0
  local ok, r = pcall(f, v, pretty and 0 or nil)
  if not ok then error(r, 2) end
  return r
end

-- ----- web: routes answered by methods -----

local routes = {}
local WebFail = { __tostring = function(e) return e.message end }
local function segments(path)
  local out = {}
  for s in path:gmatch("[^/]+") do out[#out + 1] = s end
  return out
end
local function unescape(s)
  return (s:gsub("%%(%x%x)", function(h) return string.char(tonumber(h, 16)) end))
end
function L.route(method, path, f)
  routes[#routes + 1] = { method = method, parts = segments(path), f = f }
end
function L.web_fail(status, msg)
  error(setmetatable({ status = status, message = msg }, WebFail), 2)
end
local function bad(msg) error(setmetatable({ status = 400, message = msg }, WebFail), 0) end
function L.web_param(p, name, kind)
  local v = p[name]
  if kind == "string" then return v end
  if not L.isnum(v) then bad("the path's " .. name .. " should be a number, but it's \"" .. v .. "\".") end
  if kind == "int" then
    local n = tonumber(v)
    if n ~= floor(n) then bad("the path's " .. name .. " should be a whole number, but it's \"" .. v .. "\".") end
    return n
  end
  return tonumber(v) + 0.0
end
function L.web_body(body, f)
  local ok, r = pcall(function() return f(parse(body), "") end)
  if not ok then
    if getmetatable(r) == WebFail then error(r, 0) end
    bad("the request's body doesn't fit: " .. L.errtext(r))
  end
  return r
end
local function errjson(msg) return '{"error":' .. jquote(msg) .. '}' end
function L.web_handle(method, path, body)
  path = path:gsub("%?.*$", "")
  local parts = segments(path)
  local allowed = {}
  for _, r in ipairs(routes) do
    if #r.parts == #parts then
      local p, fits = {}, true
      for i, s in ipairs(r.parts) do
        if s:sub(1, 1) == ":" then p[s:sub(2)] = unescape(parts[i])
        elseif s ~= parts[i] then fits = false break end
      end
      if fits then
        if r.method == method then
          local ok, res = pcall(r.f, p, body)
          if ok then
            if res == nil then return 204, "" end
            return 200, res
          end
          if getmetatable(res) == WebFail then return res.status, errjson(res.message) end
          return 500, errjson(L.errtext(res))
        end
        allowed[#allowed + 1] = r.method
      end
    end
  end
  if #allowed > 0 then
    return 405, errjson(path .. " answers " .. table.concat(allowed, " and ") .. ", not " .. method .. ".")
  end
  return 404, errjson("there's nothing at " .. path .. ".")
end
function L.web_request(method, path, body)
  local status, text = L.web_handle(method:upper(), path, body)
  if text == "" then return fmt("%d", status) end
  return fmt("%d", status) .. " " .. text
end
function L.web_serve(port)
  if not host or not host.serve then needs("Web:serve") end
  local e = host.serve(port, L.web_handle)
  if e then error(e, 2) end
end

-- ----- views: lust draws them, from the tables a view makes -----

-- Pieces are kept between redraws: by the piece they're in, where they're written, and their
-- turn there. A piece that isn't drawn again is forgotten. Each open window has its own.
local stores = {}

function L.view_open(class)
  if not host or not host.ui_open then needs("A view") end
  local o = L.new(class)
  local st = {items = {}, seen = {}, turns = {}}
  stores[#stores + 1] = st
  local e = host.ui_open(o:__settings(), function()
    st.seen, st.turns = {}, {}
    local v = o:__view()
    for parent, made in pairs(st.items) do
      for k, p in pairs(made) do
        if not st.seen[p] then made[k] = nil end
      end
      if next(made) == nil then st.items[parent] = nil end
    end
    return v
  end)
  stores[#stores] = nil
  if e then error(e, 0) end
end

function L.piece(parent, site, class, rowkey)
  local st = stores[#stores]
  local turns = st.turns[parent]
  if not turns then turns = {} st.turns[parent] = turns end
  local n = (turns[site] or 0) + 1
  turns[site] = n
  local made = st.items[parent]
  if not made then made = {} st.items[parent] = made end
  local key = site .. ":" .. (rowkey and ("k" .. rowkey) or n)
  local p = made[key]
  if not p then
    p = L.new(class)
    made[key] = p
  end
  st.seen[p] = true
  return p
end

-- ----- saves: readable JSON files, name.save -----

function L.save_exists(name)
  if not io or not io.open then return false end
  local f = io.open(name .. ".save", "rb")
  if f then f:close() return true end
  return false
end
function L.save_write(name, names, encs, values)
  local parts = {}
  for i, n in ipairs(names) do
    local ok, r = pcall(encs[i], values[i], 1)
    if not ok then error(r, 2) end
    parts[i] = pair(n, r, 1)
  end
  if not io or not io.open then needs("save") end
  local f = io.open(name .. ".save", "wb")
  if not f then error("can't write the save " .. name .. ".save.", 2) end
  f:write(block("{", "}", parts, 0) .. "\n")
  f:close()
end
function L.save_read(name)
  if not io or not io.open then needs("load") end
  local f = io.open(name .. ".save", "rb")
  if not f then error("there's no saved " .. name .. " yet. Check first: is saved " .. name .. "?", 2) end
  local text = f:read("*a")
  f:close()
  local ok, r = pcall(parse, text)
  if not ok then error(name .. ".save isn't a save Lust can read: " .. r, 2) end
  return r
end
function L.save_get(d, name, entry, f)
  local v = d.vals and d.vals[entry]
  if v == nil then error(name .. "'s save doesn't fit this program, so nothing was loaded: it has no " .. entry .. ".", 2) end
  local ok, r = pcall(f, v, entry)
  if not ok then error(name .. "'s save doesn't fit this program, so nothing was loaded: " .. r, 2) end
  return r
end
function L.copy_fields(o, from)
  for k, v in pairs(from) do o[k] = v end
end

-- ----- databases, on SQLite through lust -----

local function dbcall(what, ...)
  if not host or not host.db_open then error("databases need lust run, or a built program.", 3) end
  local r, e = host[what](...)
  if r == nil and e then error(e, 3) end
  return r
end
local function sqlv(v)
  if v == true then return 1 elseif v == false then return 0 end
  return v
end
function L.db_open(path, tables)
  local id = dbcall("db_open", path, tables)
  local db = {}
  for _, t in ipairs(tables) do
    local cols, kinds = {}, {}
    for i, c in ipairs(t[2]) do cols[i] = c[1]; kinds[i] = c[2] end
    db[t[1]] = { id = id, table = t[1], shown = t[3], cols = cols, kinds = kinds, textkey = kinds[1] == "string", wheres = {}, params = {} }
  end
  return db
end
local function qcopy(q)
  local n = {}
  for k, v in pairs(q) do n[k] = v end
  n.wheres = { unpack(q.wheres) }
  n.params = { unpack(q.params) }
  return n
end
local function keyof(q) return q.cols[1] end
local function ktext(q, k)
  if type(k) == "string" then return '"' .. k .. '"' end
  return L.itext(k)
end
local function qselect(q, what)
  local s = 'SELECT ' .. what .. ' FROM "' .. q.table .. '"'
  if #q.wheres > 0 then
    local w = {}
    for i, x in ipairs(q.wheres) do w[i] = "(" .. x .. ")" end
    s = s .. " WHERE " .. table.concat(w, " AND ")
  end
  local dir = q.desc and " DESC" or ""
  if q.order and q.order ~= keyof(q) then
    s = s .. ' ORDER BY "' .. q.order .. '"' .. dir .. ', "' .. keyof(q) .. '"' .. dir
  else
    s = s .. ' ORDER BY "' .. keyof(q) .. '"' .. dir
  end
  if q.limit then s = s .. " LIMIT " .. fmt("%d", q.limit) end
  return s
end
local function convert(q, r)
  for i, kind in ipairs(q.kinds) do
    if kind == "truth" then r[i] = r[i] ~= 0
    elseif kind == "float" then r[i] = r[i] + 0.0 end
  end
  return r
end
function L.q_where(q, sql, params)
  local n = qcopy(q)
  n.wheres[#n.wheres + 1] = sql
  for _, p in ipairs(params) do n.params[#n.params + 1] = sqlv(p) end
  return n
end
function L.q_order(q, col) local n = qcopy(q); n.order = col; n.desc = false; return n end
function L.q_reverse(q) local n = qcopy(q); n.desc = not n.desc; return n end
function L.q_first(q, count)
  if count < 0 then error(fmt("first needs 0 or more, not %d.", count), 2) end
  local n = qcopy(q); n.limit = count; return n
end
function L.q_rows(q)
  local cols = {}
  for i, c in ipairs(q.cols) do cols[i] = '"' .. c .. '"' end
  local rows = dbcall("db_rows", q.id, qselect(q, table.concat(cols, ", ")), q.params)
  for _, r in ipairs(rows) do convert(q, r) end
  return rows
end
function L.q_count(q)
  local rows = dbcall("db_rows", q.id, "SELECT COUNT(*) FROM (" .. qselect(q, '"' .. keyof(q) .. '"') .. ")", q.params)
  return rows[1][1]
end
function L.q_delete(q)
  dbcall("db_exec", q.id, 'DELETE FROM "' .. q.table .. '" WHERE "' .. keyof(q) .. '" IN (' .. qselect(q, '"' .. keyof(q) .. '"') .. ")", q.params)
end
function L.q_cmap(q)
  local rows = L.q_rows(q)
  for _, r in ipairs(rows) do r.n = #q.cols end
  return L.carr(q.cols, q.textkey, rows)
end
function L.t_has(q, k)
  local rows = dbcall("db_rows", q.id, 'SELECT 1 FROM "' .. q.table .. '" WHERE "' .. keyof(q) .. '" = ?', { k })
  return #rows > 0
end
function L.t_inject(q, values)
  local with_key = values.n == #q.cols
  if with_key and L.t_has(q, values[1]) then
    error("there's already a row with " .. keyof(q) .. " " .. ktext(q, values[1]) .. " in " .. q.shown .. ".", 2)
  end
  local cols, marks, params = {}, {}, {}
  local first = with_key and 1 or 2
  for i = first, #q.cols do
    cols[#cols + 1] = '"' .. q.cols[i] .. '"'
    marks[#marks + 1] = "?"
    params[#params + 1] = sqlv(values[i - first + 1])
  end
  dbcall("db_exec", q.id, 'INSERT INTO "' .. q.table .. '" (' .. table.concat(cols, ", ") .. ") VALUES (" .. table.concat(marks, ", ") .. ")", params)
end
function L.t_remove(q, k)
  local n = dbcall("db_exec", q.id, 'DELETE FROM "' .. q.table .. '" WHERE "' .. keyof(q) .. '" = ?', { k })
  if n == 0 then error("there's no row with " .. keyof(q) .. " " .. ktext(q, k) .. " in " .. q.shown .. " to remove.", 2) end
end
function L.t_get(q, k)
  if not L.t_has(q, k) then error("there's no row with " .. keyof(q) .. " " .. ktext(q, k) .. " in " .. q.shown .. ".", 2) end
  return { q = q, k = k }
end
local function rowgone(r)
  error("this row of " .. r.q.shown .. " (" .. keyof(r.q) .. " " .. ktext(r.q, r.k) .. ") has been removed.", 3)
end
function L.r_get(r, col)
  local rows = dbcall("db_rows", r.q.id, 'SELECT "' .. col .. '" FROM "' .. r.q.table .. '" WHERE "' .. keyof(r.q) .. '" = ?', { r.k })
  if #rows == 0 then rowgone(r) end
  for i, c in ipairs(r.q.cols) do
    if c == col then
      local v = rows[1][1]
      if r.q.kinds[i] == "truth" then return v ~= 0 end
      if r.q.kinds[i] == "float" then return v + 0.0 end
      return v
    end
  end
end
function L.r_set(r, col, v)
  local n = dbcall("db_exec", r.q.id, 'UPDATE "' .. r.q.table .. '" SET "' .. col .. '" = ? WHERE "' .. keyof(r.q) .. '" = ?', { sqlv(v), r.k })
  if n == 0 then rowgone(r) end
end

-- try … error (string message): just the message, without Lua's file and line.
-- Lua's own messages are turned into plain words, the same ones lust run shows.
function L.errtext(e)
  if type(e) == "table" and e.message then return e.message end
  local s = (tostring(e):gsub("^[^:]*:%d+: ", ""))
  if s:find("attempt to index") and s:find("nil value") then
    return "this is nothing, so it has no fields or methods. Check first: is it !== nothing?"
  end
  if s:find("attempt to call") and s:find("nil value") then
    return "this hook is nothing, so it can't be called. Check first: is it !== nothing?"
  end
  if s:find("stack overflow") then
    return "methods called each other too deeply. That usually means one calls itself with no way to stop."
  end
  return s
end

return L
