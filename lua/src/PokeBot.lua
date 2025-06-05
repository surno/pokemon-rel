-- PokeBot.lua, a desmume lua script for Pokemon games
local sock = require("socket").tcp()
assert(sock:connect("127.0.0.1", 5555))
sock:settimeout(0)


while true do
    -- grab the pixels of the top of the screen
    local fb = gui.screenshot()
    local top = fb.sub(1, 256 * 192 * 3)
    -- send the pixels to the server
    local ok, err = sock:send(top)
    if not ok then
        print("Error sending data: " .. err)
        break
    end
    -- receive the response from the server, {A, B, Select, Start, Up, Down, Left, Right, X, Y}
    local response, err = sock:receive(12) or ("\0"):rep(12)
    if not response then
        print("Error receiving data: " .. err)
        break
    end
    -- parse the response
    joypad.set({
        A = act:byte(1) ~= 0,
        B = act:byte(2) ~= 0,
        Select = act:byte(3) ~= 0,
        Start = act:byte(4) ~= 0,
        Up = act:byte(5) ~= 0,  
        Down = act:byte(6) ~= 0,
        Left = act:byte(7) ~= 0,
        Right = act:byte(8) ~= 0,
        X = act:byte(9) ~= 0,
        Y = act:byte(10) ~= 0,
    })
    emulator.frameadvance()
    print("Sent screenshot and received action response.")
end