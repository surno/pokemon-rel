-------------------------------------------------------------------
-- 1) all your socket setup stays the same
-------------------------------------------------------------------
local socket = require("socket")
local bit = bit32 or require("bit")  -- Lua 5.1: use luabitop if present
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

local first = true      -- print hex dump only on the very first valid frame

local capture = gui.gdscreenshot
print("[Lua] using capture: gdscreenshot (GD2)")

local function stream_frame()

    ------------------------------------------------------------------
    -- 1) finish any pending send from the previous frame
    ------------------------------------------------------------------

    if send_buf then
        if send_offset > #send_buf then
            send_buf = nil
            -- wait to receive 12-byte action before capturing next frame
            local action, recv_err = sock:receive(12)
            if not action then
                print("RECV-ACTION-ERR:", recv_err)
                sock:close()
                return
            end
        else
            local sent, err = sock:send(send_buf, send_offset)
            if sent then
                send_offset = send_offset + sent
                if send_offset > #send_buf then send_buf = nil end
            elseif err ~= "timeout" then
                print("SEND-ERR:", err)
                sock:close()
                return
            end
        end
        return
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

    -- capture top and bottom screens as separate GD2 blobs
    local top_blob = gui.gdscreenshot(1)
    local bot_blob = gui.gdscreenshot(0)
    if not top_blob or not bot_blob or #top_blob == 0 or #bot_blob == 0 then
        print("ERROR: failed to capture top or bottom screen")
        return
    end
    local blob = top_blob .. bot_blob
    local actual_size = #blob
    print(string.format("[Lua] blob length is %d bytes", actual_size))
    if first then
        local hex = blob:sub(1,16):gsub(".", function(c) return string.format("%02X ", c:byte()) end)
        print("[Lua] first 16 bytes (pixels):", hex)
        first = false
    end
    -- wrap both screens in our “GD2” + little‐endian length header
    local payload = "GD2" .. le32(actual_size) .. blob
    send_buf      = payload
    send_offset   = 1
    print(string.format("[Lua] about to send: 'GD2' | len=%d", actual_size))

end
-- 3) register once
gui.register(stream_frame)

-------------------------------------------------------------------
-- 4) tiny driver loop – just keep the Lua thread alive
-------------------------------------------------------------------
while true do
    emu.frameadvance()
end