-- Lust's runtime for LustScript: the small library every Lust program carries along.
-- Plain Lua 5.1, so it runs in Garry's Mod (LuaJIT), Luau and standard Lua.
local L = {}
local floor, fmt, insert, remove = math.floor, string.format, table.insert, table.remove
local setmetatable, getmetatable, tostring, tonumber, type, error = setmetatable, getmetatable, tostring, tonumber, type, error
local ipairs = ipairs

-- ----- text -----

function L.itext(v) return fmt("%d", v) end
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
function L.qtext(v) return "\"" .. v .. "\"" end
function L.stext(v) return v end
function L.print(s) print(s) end

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
function L.toint(v)
  if v ~= v then error("this float isn't a number, so it can't be an int.", 2) end
  if v == math.huge or v == -math.huge then error(L.ftext(v) .. " is too big to be an int.", 2) end
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

local seeded = false
function L.random(lo, hi)
  if not seeded then
    seeded = true
    local seed = os and os.getenv and tonumber(os.getenv("LUST_SEED") or "")
    if seed then math.randomseed(seed) elseif os and os.time then math.randomseed(os.time()) end
  end
  if lo > hi then error("random's low number (" .. lo .. ") is bigger than its high one (" .. hi .. ").", 2) end
  return math.random(lo, hi)
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
function L.sin(a, b)
  return a == b or (a ~= nil and b ~= nil and a:sub(1, #b + 1) == b .. ".")
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
function L.files_makeFolder(folder)
  if not host then needs("Files:makeFolder") end
  local e = host.mkdir(folder)
  if e then error(e, 2) end
end

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
  if t == "boolean" then return "a truth" end
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
  if type(v) ~= "boolean" then misfit(at, "a truth (true or false)", v) end
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
function L.jr_fields(v, at, class, names, fs)
  if v == L.NULL then return nil end
  if type(v) ~= "table" or not v.__obj then misfit(at, "an object", v) end
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

-- try … error (string message): just the message, without Lua's file and line.
-- Lua's own messages are turned into plain words, the same ones lust run shows.
function L.errtext(e)
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
