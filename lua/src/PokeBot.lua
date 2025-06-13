-------------------------------------------------------------------
-- Robust Pokemon Bot Lua Script for DeSmuME
-- Automatically reconnects and handles errors for overnight running
-------------------------------------------------------------------
local socket = require("socket")
local HOST, PORT = "192.168.10.242", 3344
local sock = nil
local connection_attempts = 0
local max_reconnect_attempts = 5
local reconnect_delay = 2  -- seconds between reconnection attempts

-- Connection state tracking
local last_successful_frame = 0
local consecutive_errors = 0
local max_consecutive_errors = 10

-- wall‑clock timestamp of the last health report
local last_report_time = os.time()

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

-- helper for 16-bit little-endian
local function le16(n)
    n = tonumber(n) or 0
    return string.char(
        n % 256,
        math.floor(n / 256) % 256
    )
end

-- helper: convert 4‑byte little‑endian string → integer
local function le32_to_int(bytes)
    local b1, b2, b3, b4 = bytes:byte(1, 4)
    return b1 + b2 * 256 + b3 * 65536 + b4 * 16777216
end

-- Frame creation helper - matches Rust server's frame_reader expectations
local function create_frame(tag, data)
    local frame_data = string.char(tag) .. (data or "")
    local length = le32(#frame_data)  -- 4-byte little-endian length
    return length .. frame_data
end

-- Create handshake frame (useful for initial connection)
local function create_handshake_frame(version, name, program)
    version = version or 1
    name = name or "PokemonLuaBot"
    program = program or 0
    
    local version_bytes = le32(version)
    local name_length_bytes = le16(#name)
    local program_bytes = le16(program)
    
    local data = version_bytes .. name_length_bytes .. name .. program_bytes
    return create_frame(1, data)  -- tag 1 = handshake
end

-- Create ping frame
local function create_ping_frame()
    return create_frame(0)  -- tag 0 = ping, no data
end

-- Create shutdown frame
local function create_shutdown_frame()
    return create_frame(3)  -- tag 3 = shutdown, no data
end

local frame_counter = 0
local first_frame = true
local target_fps = 60  -- Target FPS
local frame_time_target = 1.0 / target_fps  -- Target time per frame
local last_frame_time = socket.gettime()

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
                
                -- -- Send handshake frame to identify this client
                -- local handshake = create_handshake_frame(1, "PokemonBot_DeSmuME", 1001)
                -- local handshake_result, handshake_err = sock:send(handshake)
                -- if handshake_result then
                --     print("[Lua] ✅ Handshake sent successfully!")
                -- else
                --     print(string.format("[Lua] ⚠️  Handshake failed: %s", handshake_err or "unknown"))
                -- end
                
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

-- Process received input from the server
local function process_input(input)
    if not input or #input < 12 then
        print("[Lua] ❌ Invalid input received, skipping frame")
        return nil
    end
    
    -- Extract action bytes from input
    local action = input:sub(1, 12)
    
    print(string.format("[Lua] Received action: %s", table.concat(action_bytes, " ")))

    -- Validate action length
    if #action ~= 12 then
        print(string.format("[Lua] ❌ Invalid action length: %d bytes", #action))
        return nil
    end

    -- send the action to joypad    

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

end

local function receive_input()
    sock:settimeout(0)  -- Non-blocking mode
    while true do
        local input, err = sock:receive(12)  -- Expecting 12-byte action response
        if input then
            return process_input(input)
        elseif err == "timeout" then
            return nil  -- No input available, continue processing
        elseif err == "closed" or err == "broken pipe" then
            print("[Lua] Connection lost during receive, attempting reconnection...")
            connect_to_server()
            return nil  -- Reconnection handled, no input to process
        else
            print(string.format("[Lua] Receive error: %s", err or "unknown"))
            return nil  -- Other errors, skip this frame
        end
    end 
end 

-- Robust frame sending with error recovery
local function send_frame_and_get_action()
    -- Frame rate limiting
    local current_time = socket.gettime()
    local time_since_last_frame = current_time - last_frame_time
    
    if time_since_last_frame < frame_time_target then
        -- Skip this frame to maintain target FPS
        return
    end
    
    last_frame_time = current_time
    frame_counter = frame_counter + 1
    
    -- Check connection health periodically
    if frame_counter % 100 == 0 and not is_connection_alive() then
        print("[Lua] ⚠️  Connection health check failed, attempting reconnection...")
        if not connect_to_server() then
            print("[Lua] ❌ Reconnection failed, skipping this frame")
            return
        end
    end
    
    local raw_buf = gui.gdscreenshot(0)      -- both screens, GD2 header + pixels
    local expected_len = 256 * 384 * 4       -- 393_216 bytes for one 256×384 RGBA frame

    -- Skip the 7‑byte GD2 header and take exactly the expected pixel payload.
    -- Any padding bytes after the payload are ignored.
    local screens_raw = raw_buf:sub(8, 8 + expected_len - 1)

    if #screens_raw ~= expected_len then
        print(string.format("[Lua] Error: sliced %d bytes, expected %d", #screens_raw, expected_len))
        return
    end
    if not screens_raw or #screens_raw == 0 then
        print("[Lua] ERROR: failed to capture screens")
        return
    end

    if #screens_raw % 4 ~= 0 then
        print(string.format("[Lua] Error: Raw screen data size is not a multiple of 4 (%d bytes)", #screens_raw))
        return
    end
    
    local screen_pixels
    
    if first_frame then
        print(string.format("[Lua] Raw screen data: %d bytes", #screens_raw))
        -- Save the image data to files for debugging
        local screen_file = io.open("screens.ppm", "wb")
        if screen_file then
            local width, height = 256, 384
            local rgb = {}
            -- peek the first pixel's four BGRA bytes

            for i = 1, #screens_raw, 4 do
                -- GD2 pixel order is A R G B; convert to R G B
                local r = screens_raw:byte(i + 1)         -- source order: B
                local g = screens_raw:byte(i + 2)     -- source order: G
                local b = screens_raw:byte(i + 3)     -- source order: R
                rgb[#rgb + 1] = string.char(r, g, b)  -- write as R G B
            end
            local rgb_blob = table.concat(rgb)
            screen_file:write(string.format("P6 %d %d 255\n", width, height))
            screen_file:write(rgb_blob)
            screen_file:close()
        else
            print("[Lua] ❌ Failed to write bottom screen data to file")
        end
        print("[Lua] ✅ Screens captured successfully, processing data...")
        print(string.format("[Lua] Screen size: %d bytes", #screens_raw))
        print("[Lua] Processing frames...")
        -- Calculate expected sizes to determine format
        local screen_pixels = 256 * 384 
        local bytes_per_pixel = #screens_raw / screen_pixels
        print(string.format("[Lua] %.1f bytes/pixel", bytes_per_pixel))

        print("[Lua] Converting raw pixel data to RGB format...")
    end
    
    -- Convert to RGB based on detected format
    local function convert_to_rgb_from_bgra(data)
        -- Use gsub for much faster bulk processing instead of loops
        local rgb_data = data:gsub("(.)(.)(.)(.)", function(a, r, g, b)
            return r .. g .. b  -- Convert BGRA to RGB, skip alpha
        end)
        return rgb_data
    end

    pixels = convert_to_rgb_from_bgra(screens_raw)
    
    -- Create appropriate frame based on detected format
    local width, height = 256, 384
    local image_data = le32(width) .. le32(height) .. pixels

    if first_frame then
        print(string.format("[Lua] After conversion - Top: %d bytes", #pixels))
        print(string.format("[Lua] Expected RGB: %d bytes", width * height * 3))
    end
    
    
    -- Determine frame type based on data format
    local is_gd2 = (#screens_raw >= 4 and screens_raw:sub(1,3) == "GD2")
    local frame_tag = is_gd2 and 4 or 2  -- tag 4 = GD2, tag 2 = RGB
    local blob = create_frame(frame_tag, image_data)
    
    local total_size = #blob
    if first_frame then
        local format_name = is_gd2 and "GD2" or "RGB"
        print(string.format("[Lua] 📸 Sending frame with tag %d (%s format)", frame_tag, format_name))
        print(string.format("[Lua] Frame %d: Total frame size=%d bytes", frame_counter, total_size))
        print(string.format("[Lua] Frame structure: length(4) + tag(1) + width(4) + height(4) + %s_data(%d)", format_name, #pixels))
        
        -- Validate final size for RGB data
        if not is_gd2 then
            local expected_rgb_size = 256 * 384 * 3  -- 294,912 bytes for RGB
            if #pixels > expected_rgb_size then
                print(string.format("[Lua] ⚠️  Warning: RGB data larger than expected (%d vs %d)", #pixels, expected_rgb_size))
            else
                print(string.format("[Lua] ✅ RGB data size looks correct (%d bytes)", #pixels))
            end
        end
        
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
        local result, err = sock:send(blob)
        
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
        print(string.format("[Lua] ✅ Sent frame %d (%d bytes)", frame_counter, #blob))
    end
    
    -- Wait for 12-byte action response with retry logic
    local receive_attempts = 0
    local max_receive_attempts = 3
    
    -- Debug: Show button presses and connection status occasionally
    if frame_counter % 500 == 0 then
        local status_msg = string.format("[Lua] Frame %d: Connection stable, %d consecutive errors",
                                        frame_counter, consecutive_errors)
        print(status_msg)
    end

    -- Periodic connection health report
    if frame_counter % 500 == 0 then
        local now = os.time()
        local uptime_seconds = now - last_report_time
        local uptime_frames  = frame_counter - last_successful_frame

        -- avoid division by zero
        local fps = uptime_seconds > 0 and (uptime_frames / uptime_seconds) or 0

        print(string.format(
            "[Lua] 🕒 Uptime: %d seconds, Frames since last success: %d, Errors: %d",
            uptime_seconds, uptime_frames, consecutive_errors))

        print(string.format(
            "[Lua] 🔋 Health Report - Frame %d, %d frames since last success, %d errors, fps: %.2f",
            frame_counter, uptime_frames, consecutive_errors, fps))

        last_report_time = now
        last_successful_frame = frame_counter
    end
end

-- wrap the one‑shot functions in an infinite coroutine loop
local receive_co = coroutine.create(function()
    while true do
        receive_input()
        coroutine.yield()
    end
end)

local send_co = coroutine.create(function()
    while true do
        send_frame_and_get_action()
        coroutine.yield()
    end
end)

-- Register the function to run every frame
gui.register(function()
    coroutine.resume(receive_co)   -- Receive input from server
    coroutine.resume(send_co)      -- Send frame and get action
end)

-------------------------------------------------------------------
-- Main loop - keep the emulator running with robust error handling
-------------------------------------------------------------------
print("[Lua] 🤖 Robust Pokemon Bot started!")
print("[Lua] 🛡️  Features: Auto-reconnect, Error recovery, Stability")

-- Initial connection
if connect_to_server() then
    print("[Lua] Bot ready for operation!")
else
    print("[Lua] ❌ Could not establish initial connection")
    print("[Lua] ⚠️  Bot will continue trying to connect during operation...")
end

-- Reset reconnection delay for ongoing operation
reconnect_delay = 2

print("[Lua] 🌙 Starting operation - Press Stop Script to quit")
while true do
    emu.frameadvance()
end