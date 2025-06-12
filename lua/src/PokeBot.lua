-------------------------------------------------------------------
-- Robust Pokemon Bot Lua Script for DeSmuME
-- Automatically reconnects and handles errors for overnight running
-------------------------------------------------------------------
local socket = require("socket")
local HOST, PORT = "192.168.10.242", 5555
local sock = nil
local connection_attempts = 0
local max_reconnect_attempts = 5
local reconnect_delay = 2  -- seconds between reconnection attempts

-- Connection state tracking
local last_successful_frame = 0
local consecutive_errors = 0
local max_consecutive_errors = 10

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

local frame_counter = 0
local first_frame = true

-- Robust connection function with retry logic
local function connect_to_server()
    if sock then
        sock:close()
        sock = nil
    end
    
    local attempts = 0
    while attempts < max_reconnect_attempts do
        attempts = attempts + 1
        print(string.format("[Lua] Connection attempt %d/%d...", attempts, max_reconnect_attempts))
        
        sock = socket.tcp()
        if sock then
            sock:settimeout(15)  -- Increased timeout for more stability
            local success, err = sock:connect(HOST, PORT)
            if success then
                print("[Lua] ✅ Connected to server successfully!")
                connection_attempts = 0
                consecutive_errors = 0
                return true
            else
                print(string.format("[Lua] ❌ Connection failed: %s", err or "unknown error"))
                sock:close()
                sock = nil
            end
        end
        
        if attempts < max_reconnect_attempts then
            print(string.format("[Lua] Waiting %d seconds before retry...", reconnect_delay))
            socket.sleep(reconnect_delay)
            reconnect_delay = math.min(reconnect_delay * 1.5, 10)  -- Exponential backoff, max 10 seconds
        end
    end
    
    print("[Lua] ❌ Failed to connect after maximum attempts")
    return false
end

-- Check if connection is still alive
local function is_connection_alive()
    if not sock then return false end
    
    -- Try a simple send with minimal data to test connection
    local test_result, err = sock:send("")
    if test_result == nil and err ~= "timeout" then
        return false
    end
    return true
end

-- Robust frame sending with error recovery
local function send_frame_and_get_action()
    frame_counter = frame_counter + 1
    
    -- Check connection health periodically
    if frame_counter % 100 == 0 and not is_connection_alive() then
        print("[Lua] ⚠️  Connection health check failed, attempting reconnection...")
        if not connect_to_server() then
            print("[Lua] ❌ Reconnection failed, skipping this frame")
            return
        end
    end
    
    -- Capture both screens and combine them manually
    local top_raw = gui.gdscreenshot(1)  -- Top screen
    local bot_raw = gui.gdscreenshot(0)  -- Bottom screen
    
    if not top_raw or not bot_raw or #top_raw == 0 or #bot_raw == 0 then
        print("[Lua] ERROR: failed to capture screens")
        return
    end
    
    -- Remove GD2 headers from individual captures and get raw pixel data
    local top_pixels = top_raw:sub(8)  -- Skip "GD2" + 4-byte length
    local bot_pixels = bot_raw:sub(8)  -- Skip "GD2" + 4-byte length
    
    -- Combine pixel data: top screen first (192 lines), then bottom screen (192 lines)
    local combined_pixels = top_pixels .. bot_pixels
    local frame_tag = 2
    local frame_data = le32(256) .. le32(384) .. combined_pixels

    local blob = frame_tag .. le32(#frame_data) .. frame_data
    
    if first_frame then
        print(string.format("[Lua] Frame %d: Combined size=%d bytes", frame_counter, total_size))
        print(string.format("[Lua] Top pixels: %d bytes, Bot pixels: %d bytes", #top_pixels, #bot_pixels))
        local hex = combined_pixels:sub(1,16):gsub(".", function(c) return string.format("%02X ", c:byte()) end)
        print("[Lua] First 16 bytes of combined pixels:", hex)
        first_frame = false
    end
    
    -- Ensure we have a valid connection
    if not sock and not connect_to_server() then
        print("[Lua] ❌ No connection available, skipping frame")
        return
    end
    
    -- Send the complete GD2 blob with retry logic
    local send_attempts = 0
    local max_send_attempts = 3
    local sent = false
    
    while send_attempts < max_send_attempts and not sent do
        send_attempts = send_attempts + 1
        local result, err = sock:send(gd2_blob)
        
        if result then
            sent = true
            consecutive_errors = 0
        else
            print(string.format("[Lua] SEND ERROR (attempt %d/%d): %s", send_attempts, max_send_attempts, err or "unknown"))
            consecutive_errors = consecutive_errors + 1
            
            if err == "closed" or err == "broken pipe" then
                print("[Lua] Connection lost, attempting reconnection...")
                if connect_to_server() then
                    -- Try sending again with new connection - reset attempt counter
                    send_attempts = send_attempts - 1  -- Give it another try with new connection
                else
                    break
                end
            elseif send_attempts < max_send_attempts then
                socket.sleep(0.1)  -- Brief pause before retry
            end
        end
    end
    
    if not sent then
        print("[Lua] ❌ Failed to send frame after all attempts")
        if consecutive_errors >= max_consecutive_errors then
            print("[Lua] ⚠️  Too many consecutive errors, forcing reconnection...")
            connect_to_server()
        end
        return
    end
    
    if frame_counter % 100 == 0 then
        print(string.format("[Lua] ✅ Sent frame %d (%d bytes)", frame_counter, #gd2_blob))
    end
    
    -- Wait for 12-byte action response with retry logic
    local receive_attempts = 0
    local max_receive_attempts = 3
    local action = nil
    
    while receive_attempts < max_receive_attempts and not action do
        receive_attempts = receive_attempts + 1
        local result, recv_err = sock:receive(12)
        
        if result then
            action = result
            consecutive_errors = 0
            last_successful_frame = frame_counter
        else
            print(string.format("[Lua] RECV ERROR (attempt %d/%d): %s", receive_attempts, max_receive_attempts, recv_err or "unknown"))
            consecutive_errors = consecutive_errors + 1
            
            if recv_err == "closed" or recv_err == "broken pipe" then
                print("[Lua] Connection lost during receive, attempting reconnection...")
                if connect_to_server() then
                    -- Connection restored, but we've lost this frame's action
                    -- Send a "no action" and continue
                    action = string.char(0,0,0,0,0,0,0,0,0,0,0,0)
                    break
                else
                    break
                end
            elseif recv_err == "timeout" then
                print("[Lua] ⏰ Receive timeout - server might be busy with AI training")
                if receive_attempts < max_receive_attempts then
                    socket.sleep(0.5)  -- Longer pause for timeout
                end
            elseif receive_attempts < max_receive_attempts then
                socket.sleep(0.1)  -- Brief pause before retry
            end
        end
    end
    
    if not action then
        print("[Lua] ❌ Failed to receive action after all attempts, using default (no buttons)")
        action = string.char(0,0,0,0,0,0,0,0,0,0,0,0)  -- Default: no buttons pressed
        
        if consecutive_errors >= max_consecutive_errors then
            print("[Lua] ⚠️  Too many consecutive errors, forcing reconnection...")
            connect_to_server()
        end
    end
    
    -- Apply the action to joypad
    joypad.set{
        A=action:byte(1)~=0,      -- Button A
        B=action:byte(2)~=0,      -- Button B  
        Select=action:byte(3)~=0, -- Select
        Start=action:byte(4)~=0,  -- Start
        Up=action:byte(5)~=0,     -- D-pad Up
        Down=action:byte(6)~=0,   -- D-pad Down
        Left=action:byte(7)~=0,   -- D-pad Left
        Right=action:byte(8)~=0,  -- D-pad Right
        X=action:byte(9)~=0,      -- Button X
        Y=action:byte(10)~=0,     -- Button Y
        L=action:byte(11)~=0,     -- Left shoulder
        R=action:byte(12)~=0,     -- Right shoulder
    }
    
    -- Debug: Show button presses and connection status occasionally
    if frame_counter % 500 == 0 then
        local pressed = {}
        if action:byte(1)~=0 then table.insert(pressed, "A") end
        if action:byte(2)~=0 then table.insert(pressed, "B") end  
        if action:byte(4)~=0 then table.insert(pressed, "Start") end
        if action:byte(5)~=0 then table.insert(pressed, "Up") end
        if action:byte(6)~=0 then table.insert(pressed, "Down") end
        if action:byte(7)~=0 then table.insert(pressed, "Left") end
        if action:byte(8)~=0 then table.insert(pressed, "Right") end
        
        local status_msg = string.format("[Lua] Frame %d: Connection stable, %d consecutive errors", 
                                        frame_counter, consecutive_errors)
        if #pressed > 0 then
            status_msg = status_msg .. string.format(", Pressing: %s", table.concat(pressed, "+"))
        end
        print(status_msg)
    end
    
    -- Periodic connection health report
    if frame_counter % 2000 == 0 then
        local uptime_frames = frame_counter - last_successful_frame
        print(string.format("[Lua] 🔋 Health Report - Frame %d, %d frames since last success, %d errors", 
                           frame_counter, uptime_frames, consecutive_errors))
    end
end

-- Register the function to run every frame
gui.register(send_frame_and_get_action)

-------------------------------------------------------------------
-- Main loop - keep the emulator running with robust error handling
-------------------------------------------------------------------
print("[Lua] 🤖 Robust Pokemon Bot started!")
print("[Lua] 🛡️  Features: Auto-reconnect, Error recovery, Overnight stability")

-- Initial connection
if connect_to_server() then
    print("[Lua] 🚀 Bot ready for overnight operation!")
else
    print("[Lua] ❌ Could not establish initial connection")
    print("[Lua] ⚠️  Bot will continue trying to connect during operation...")
end

-- Reset reconnection delay for ongoing operation
reconnect_delay = 2

print("[Lua] 🌙 Starting overnight operation - Press Stop Script to quit")
while true do
    emu.frameadvance()
end