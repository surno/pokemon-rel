-------------------------------------------------------------------
-- 1) all your socket setup stays the same
-------------------------------------------------------------------
local socket = require("socket")
local HOST, PORT = "192.168.10.242", 5555
local sock = assert(socket.tcp())
assert(sock:connect(HOST, PORT))
sock:settimeout(2)

sock:settimeout(0)             -- non‑blocking I/O

-- state carried across frames
local send_buf     = nil       -- string to send (header+pixels)
local send_offset  = 1         -- next byte to write
local recv_buf     = ""        -- partial 12‑byte action buffer
local CHUNK        = 32768     -- bytes per send attempt

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

local function stream_frame()
    ------------------------------------------------------------------
    -- 1) finish any pending send from the previous frame
    ------------------------------------------------------------------
    if send_buf then
        local to_send = send_buf:sub(send_offset, send_offset + CHUNK - 1)
        local sent, err, partial = sock:send(to_send)
        if sent and sent > 0 then
            send_offset = send_offset + sent
        elseif err == "timeout" and partial and partial > 0 then
            send_offset = send_offset + partial
        end
        if send_offset > #send_buf then
            send_buf, send_offset = nil, 1
        else
            return                         -- wait next frame to continue
        end
    end

    ------------------------------------------------------------------
    -- 2) receive the 12‑byte action mask (non‑blocking)
    ------------------------------------------------------------------
    if #recv_buf < 12 then
        local chunk = sock:receive(12 - #recv_buf)
        if chunk and #chunk > 0 then
            recv_buf = recv_buf .. chunk
        end
        if #recv_buf == 12 then
            local a = recv_buf
            joypad.set{
                A=a:byte(1)~=0,  B=a:byte(2)~=0,
                Select=a:byte(3)~=0, Start=a:byte(4)~=0,
                Up=a:byte(5)~=0, Down=a:byte(6)~=0,
                Left=a:byte(7)~=0, Right=a:byte(8)~=0,
                X=a:byte(9)~=0,  Y=a:byte(10)~=0,
                L=a:byte(11)~=0, R=a:byte(12)~=0,
            }
            recv_buf = ""                  -- clear for next frame
        end
    end

    ------------------------------------------------------------------
    -- 3) if nothing pending, capture a new frame and start sending
    ------------------------------------------------------------------
    if send_buf then return end   -- still flushing old frame

    local w, h, pixels = capture()
    if type(w) == "string" and pixels == nil then
        pixels = w;  w, h = 256, (#pixels / (256*3) >= 384) and 384 or 192
    end
    if type(pixels) ~= "string" then return end

    if pixels:sub(1,3) == "GD" then
        pixels = pixels:sub(12)
    end
    if h > 192 then
        h = 192
        pixels = pixels:sub(1, 256*192*3)
    end
    if #pixels < 256*192*3 then return end

    send_buf    = le32(256) .. le32(192) .. pixels
    send_offset = 1
end

-- 3) register once
gui.register(stream_frame)

-------------------------------------------------------------------
-- 4) tiny driver loop – just keep the Lua thread alive
-------------------------------------------------------------------
while true do
    emu.frameadvance()
end