-------------------------------------------------------------------
-- 1) all your socket setup stays the same
-------------------------------------------------------------------
local socket = require("socket")
local HOST, PORT = "192.168.10.242", 5555
local sock = assert(socket.tcp())
assert(sock:connect(HOST, PORT))
sock:settimeout(2)

-- helper for 32-bit little-endian
local function le32(n)
    n = tonumber(n) or 0
    return string.char(
        n        % 256,
        math.floor(n / 256)       % 256,
        math.floor(n / 65536)     % 256,
        math.floor(n / 16777216)  % 256
    )
end

-------------------------------------------------------------------
-- 2) frame-handler: gets called automatically every frame
-------------------------------------------------------------------
-- pick the first capture function available in this build
local capture = gui.gdscreenshotRaw or gui.gdscreenshotVRAM or gui.gdscreenshot
print(string.format("[Lua] using capture: %s", debug.getinfo(capture).name or "unknown"))

local first_frame = true
local function stream_frame()
    local w, h, pixels = capture()
    -- Some capture variants return only the pixel string (no w/h)
    if type(w) == "string" and pixels == nil then
        pixels = w
        w, h = 256, (#pixels / (256 * 3) >= 384) and 384 or 192
    end
    if type(pixels) ~= "string" then return end               -- still blank

    -- if we fell back to gd‑encoded string, strip 11‑byte GD header
    if pixels:sub(1,3) == "GD" then
        pixels = pixels:sub(12)
    end

    -- many builds return both screens (256×384); crop to top 192‑pixel panel
    if h and h > 192 then
        h = 192
        pixels = pixels:sub(1, 256*192*3)
    end

    -- sanity‑check length
    if not h or #pixels < 256*192*3 then return end

    -- 1️⃣ send 8‑byte header (little‑endian width & height)
    sock:send(le32(256) .. le32(192))

    -- 2️⃣ send raw RGB data in chunks
    local offset = 1
    while offset <= #pixels do
        local sent, err, partial = sock:send(pixels, offset)
        if sent and sent > 0 then
            offset = offset + sent
        elseif err == "timeout" and partial and partial > 0 then
            -- partial bytes were sent before timeout; advance pointer
            offset = offset + partial
        elseif err ~= "timeout" then
            print("SEND-ERR:", err or "unknown")
            return
        end
    end

    if first_frame then
        print(string.format("[Lua] first frame sent (%d bytes)", #pixels))
        first_frame = false
    end

    -- 3️⃣ receive 12‑byte action mask (neutral fallback)
    local act = sock:receive(12) or ("\0"):rep(12)
    joypad.set{
        A=act:byte(1)~=0,  B=act:byte(2)~=0,
        Select=act:byte(3)~=0, Start=act:byte(4)~=0,
        Up=act:byte(5)~=0, Down=act:byte(6)~=0,
        Left=act:byte(7)~=0, Right=act:byte(8)~=0,
        X=act:byte(9)~=0,  Y=act:byte(10)~=0,
        L=act:byte(11)~=0, R=act:byte(12)~=0,
    }
end

-- 3) register once
gui.register(stream_frame)

-------------------------------------------------------------------
-- 4) tiny driver loop – just keep the Lua thread alive
-------------------------------------------------------------------
while true do
    emu.frameadvance()
end