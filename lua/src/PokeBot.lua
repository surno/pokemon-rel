-- PokeBot.lua – DeSmuME Lua client
local socket = require("socket")
local HOST, PORT = "192.168.1.23", 5555      -- ← MacBook IP + port
local sock = assert(socket.tcp())
assert(sock:connect(HOST, PORT))
sock:settimeout(0)


while true do
    -- grab the pixels of the top of the screen
    -- old (line ~9)
    -- local screenshot = screenshot()      -- nil!

    -- new
    local w, h, pixels = gui.gdscreenshot()   -- returns 256,192, <147,456-byte string>

    -- send to server 
    local ok, err = sock:send(pixels)
    if not ok then
        print("Error sending data: " .. err)
        break
    end
    -- receive the response from the server, {A, B, Select, Start, Up, Down, Left, Right, X, Y}
    local response = ""
    while #response < 12 do
        local chunk, err = sock:receive(12 - #response)
        if not chunk then
            print("Socket error: "..tostring(err))
            response = ("\0"):rep(12)     -- neutral action
            break
        end
        response = response .. chunk
    end
    -- parse the response
    joypad.set({
        A = response:byte(1) ~= 0,
        B = response:byte(2) ~= 0,
        Select = response:byte(3) ~= 0,
        Start = response:byte(4) ~= 0,
        Up = response:byte(5) ~= 0,  
        Down = response:byte(6) ~= 0,
        Left = response:byte(7) ~= 0,
        Right = response:byte(8) ~= 0,
        X = response:byte(9) ~= 0,
        Y = response:byte(10) ~= 0,
    })
    emulator.frameadvance()
    print("frame OK")
end