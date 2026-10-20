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
    local parts = { fmt("%d", r.id) }
    for i, c in ipairs(cols) do parts[i + 1] = (r[c] == nil) and "nothing" or fs[i](r[c]) end
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

-- ----- cArrays: rows with ids, found by id -----

local function addrow(c, values, level)
  local ncols = #c.cols
  local id
  local first = 1
  if values.n == ncols + 1 then
    id = values[1]
    first = 2
    if c.byId[id] then error(fmt("there's already a row with id %d.", id), level) end
  else
    id = c.next
  end
  local row = { id = id, __cols = c.cols }
  for i, col in ipairs(c.cols) do row[col] = values[first + i - 1] end
  insert(c.rows, row)
  c.byId[id] = row
  if id >= c.next then c.next = id + 1 end
end
function L.carr(cols, rows)
  local c = { cols = cols, rows = {}, byId = {}, next = 1 }
  for _, r in ipairs(rows) do addrow(c, r, 4) end
  return c
end
function L.rows(c)
  local copy = {}
  for i, r in ipairs(c.rows) do copy[i] = r end
  return copy
end
local function ids(c)
  local list = {}
  for i, r in ipairs(c.rows) do list[i] = fmt("%d", r.id) end
  if #list == 0 then return "The cArray is empty." end
  return "The ids are " .. table.concat(list, ", ") .. "."
end
function L.cget(c, id)
  local r = c.byId[id]
  if not r then error(fmt("there's no row with id %d. %s", id, ids(c)), 2) end
  return r
end
function L.cinject(c, values) addrow(c, values, 3) end
function L.chas(c, id) return c.byId[id] ~= nil end
function L.cremove(c, id)
  if not c.byId[id] then error(fmt("there's no row with id %d to remove. %s", id, ids(c)), 2) end
  c.byId[id] = nil
  for i, r in ipairs(c.rows) do
    if r.id == id then remove(c.rows, i) break end
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
function L.readText(path)
  if not io or not io.open then error("readText isn't available here.", 2) end
  local f, e = io.open(path, "r")
  if not f then error("can't read " .. path .. ": " .. tostring(e), 2) end
  local s = f:read("*a")
  f:close()
  return s
end
function L.writeText(path, text)
  if not io or not io.open then error("writeText isn't available here.", 2) end
  local f, e = io.open(path, "w")
  if not f then error("can't write " .. path .. ": " .. tostring(e), 2) end
  f:write(text)
  f:close()
end
function L.wait(s)
  if s < 0 or s ~= s then error("can't wait " .. L.ftext(s) .. " seconds.", 2) end
  if not os or not os.clock then error("wait isn't available here. Use the host program's timers.", 2) end
  local stop = os.clock() + s
  while os.clock() < stop do end
end

-- try … error (string message): just the message, without Lua's file and line.
function L.errtext(e)
  local s = tostring(e)
  return (s:gsub("^[^:]*:%d+: ", ""))
end

return L
