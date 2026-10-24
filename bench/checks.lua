-- Run by `cargo run --release -- --lua checks.lua` (CI does): the fast ways step 6 added give
-- exactly what the plain ways give, over many numbers.
assert(L.f32_slow, "Luau's vectors should round to f32 here")
local slow, fast = L.f32_slow, L.f32
local count = 0
math.randomseed(6)
local function check(x)
  local a, b = slow(x), fast(x)
  if not ((a == b and (a ~= 0 or 1 / a == 1 / b)) or (a ~= a and b ~= b)) then
    error(string.format("f32(%.17g): %.17g, but the fast way gives %.17g", x, a, b))
  end
  local v = L.vec(x, -x, x * 0.5)
  if v.x ~= a or v.y ~= slow(-x) or v.z ~= slow(x * 0.5) then
    error(string.format("vec(%.17g) isn't rounded the same way", x))
  end
  count = count + 1
end
-- Every power of two an f32 can reach and beyond, with numbers around each.
for e = -160, 140 do
  for _ = 1, 400 do
    local m = 1 + math.random()
    check(math.ldexp(m, e))
    check(-math.ldexp(m, e))
  end
  -- Halfway between two f32s: ties go to even.
  local base = math.ldexp(1, e)
  check(base * (1 + 2 ^ -24))
  check(base * (1 + 3 * 2 ^ -24))
end
for _, x in ipairs({ 0, -0.0, 1 / 0, -1 / 0, 3.4028235677973366e38, 3.4028235677973362e38, 1e-45, 7e-46, 1.401298464324817e-45, 0.7e-45 }) do
  check(x)
end
-- The same number, and the same sign when it's 0.
local function same(a, b) return a == b and (a ~= 0 or 1 / a == 1 / b) end
-- Turning around z only gives what the full rotation gives.
for _ = 1, 20000 do
  local d = (math.random() - 0.5) * 2000
  local a, b = L.rot(0, 0, d), L.rotz(d)
  if not (same(a.x, b.x) and same(a.y, b.y) and same(a.z, b.z) and same(a.w, b.w)) then
    error("rotz(" .. d .. ") differs from rot(0, 0, " .. d .. ")")
  end
  count = count + 1
end
print("fast ways checked: " .. count .. " numbers, all the same")
