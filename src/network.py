"""
Network communication utilities for Pokemon Shiny Bot
Handles TCP socket connections with the Lua script
"""

import socket
import struct
import time
import sys
from config import (
    LISTEN_PORT, MAX_CONNECTION_ATTEMPTS, CONNECTION_TIMEOUT, 
    SYNC_TIMEOUT, MAX_SEND_ATTEMPTS
)

# Global shutdown handling
shutdown_requested = False
shutdown_start_time = None


class NetworkManager:
    """Manages TCP connection with the Lua script."""
    
    def __init__(self):
        self.listener = None
        self.conn = None
        self._sync_buffer = b""
        self._connection_start_time = None
        self._last_successful_communication = None
        self._consecutive_failures = 0
        self._max_consecutive_failures = 20
        
        self._setup_listener()
    
    def _setup_listener(self):
        """Set up the TCP listener socket."""
        self.listener = socket.socket()
        self.listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.listener.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
        self.listener.bind(("0.0.0.0", LISTEN_PORT))
        self.listener.listen(1)
        print(f"[Network] 🚀 Listening on 0.0.0.0:{LISTEN_PORT}")
    
    def wait_for_connection(self):
        """Wait for bot connection with robust error handling."""
        connection_attempts = 0
        
        while connection_attempts < MAX_CONNECTION_ATTEMPTS and not shutdown_requested:
            try:
                connection_attempts += 1
                print(f"[Network] 🔄 Waiting for bot connection (attempt {connection_attempts}/{MAX_CONNECTION_ATTEMPTS})...")
                
                # Set timeout to allow checking for shutdown
                self.listener.settimeout(SYNC_TIMEOUT)
                
                try:
                    self.conn, addr = self.listener.accept()
                    print(f"[Network] ✅ Bot connected from {addr}")
                    
                    # Configure socket for robustness
                    self.conn.settimeout(CONNECTION_TIMEOUT)
                    self.conn.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
                    
                    self._sync_buffer = b""
                    self._connection_start_time = time.time()
                    self._last_successful_communication = time.time()
                    self._consecutive_failures = 0
                    
                    self._synchronize_connection()
                    return True
                    
                except socket.timeout:
                    print("[Network] ⏰ Connection timeout, retrying...")
                    continue
                    
            except Exception as e:
                print(f"[Network] ❌ Connection error: {e}")
                if connection_attempts < MAX_CONNECTION_ATTEMPTS:
                    print(f"[Network] ⏳ Waiting 5 seconds before retry...")
                    time.sleep(5)
        
        if shutdown_requested:
            print("[Network] 🛑 Shutdown requested during connection")
            return False
        else:
            raise ConnectionError("Failed to establish connection after maximum attempts")
    
    def _synchronize_connection(self):
        """Synchronize the connection by finding the next GD2 header."""
        print("[Network] 🔄 Synchronizing connection...")
        
        # Send initial action to kickstart the Lua script
        try:
            initial_action = bytes([0] * 12)
            self.conn.sendall(initial_action)
        except Exception as e:
            print(f"[Network] ❌ Failed to send initial action: {e}")
            return
        
        # Look for GD2 header in incoming data stream
        search_buffer = bytearray()
        max_search_bytes = 2000000
        bytes_searched = 0
        
        while bytes_searched < max_search_bytes and not shutdown_requested:
            try:
                chunk = self.conn.recv(1024)
                if not chunk:
                    raise ConnectionError("Connection closed while searching for GD2 header")
                
                search_buffer.extend(chunk)
                bytes_searched += len(chunk)
                
                # Look for GD2 pattern
                gd2_pos = search_buffer.find(b'GD2')
                if gd2_pos != -1:
                    # Remove everything before the GD2 header
                    search_buffer = search_buffer[gd2_pos:]
                    self._sync_buffer = bytes(search_buffer)
                    print(f"[Network] ✅ Synchronized! Ready to receive frames.")
                    self._last_successful_communication = time.time()
                    return
                
                # Keep only last 2 bytes to avoid missing GD2 across boundaries
                if len(search_buffer) > 2048:
                    search_buffer = search_buffer[-2:]
                    
            except socket.timeout:
                print("[Network] ⏰ Timeout while searching for GD2 - retrying...")
                continue
            except Exception as e:
                print(f"[Network] ❌ Error during synchronization: {e}")
                break
        
        print("[Network] ⚠️  Could not find GD2 header - connection may be unstable")
        self._sync_buffer = b""
    
    def receive_exact_bytes(self, n):
        """
        Receive exactly n bytes from the connection.
        
        Args:
            n: Number of bytes to receive
            
        Returns:
            bytes: Exactly n bytes of data
            
        Raises:
            ConnectionError: If connection fails or shutdown is requested
        """
        buf = bytearray()
        
        # First, use any data from sync buffer
        if self._sync_buffer:
            bytes_to_take = min(len(self._sync_buffer), n)
            buf.extend(self._sync_buffer[:bytes_to_take])
            self._sync_buffer = self._sync_buffer[bytes_to_take:]
        
        # Receive the rest normally
        while len(buf) < n and not shutdown_requested:
            remaining = n - len(buf)
            try:
                # Short timeout for responsive shutdown
                self.conn.settimeout(1.0)
                chunk = self.conn.recv(remaining)
                self.conn.settimeout(CONNECTION_TIMEOUT)  # Reset timeout
            except socket.timeout:
                if shutdown_requested:
                    raise ConnectionError("Shutdown requested during receive")
                continue
            except Exception as e:
                raise ConnectionError(f"Network error: {e}")
            
            if not chunk:
                raise ConnectionError("Bot closed connection mid‑frame")
            buf.extend(chunk)
        
        if shutdown_requested:
            raise ConnectionError("Shutdown requested")
        
        return bytes(buf)
    
    def send_action(self, action_bytes):
        """
        Send action bytes to the bot with retry logic.
        
        Args:
            action_bytes: 12-byte action array
            
        Returns:
            bool: True if sent successfully, False otherwise
        """
        send_attempts = 0
        
        while send_attempts < MAX_SEND_ATTEMPTS and not shutdown_requested:
            try:
                self.conn.sendall(action_bytes)
                self._last_successful_communication = time.time()
                self._consecutive_failures = 0
                return True
            except Exception as e:
                send_attempts += 1
                print(f"[Network] ⚠️  Send attempt {send_attempts}/{MAX_SEND_ATTEMPTS} failed: {e}")
                if send_attempts < MAX_SEND_ATTEMPTS:
                    time.sleep(0.1)
        
        return False
    
    def receive_gd2_frame(self):
        """
        Receive a complete GD2 frame from the bot.
        
        Returns:
            bytes: GD2 frame data
            
        Raises:
            ConnectionError: If frame reception fails
        """
        # Read GD2 tag (3 bytes)
        tag3 = self.receive_exact_bytes(3)
        if tag3 != b"GD2":
            raise ConnectionError(f"Expected 'GD2' tag, got {tag3!r}")
        
        # Read length (4 bytes)
        length = struct.unpack("<I", self.receive_exact_bytes(4))[0]
        if length < 20:
            raise ConnectionError(f"GD2 length too short: {length}")
        
        # Read the actual GD2 blob
        gd_blob = self.receive_exact_bytes(length)
        if len(gd_blob) < length:
            raise ConnectionError(f"Truncated GD2 blob ({len(gd_blob)} vs {length})")
        
        return gd_blob
    
    def handle_connection_failure(self):
        """Handle connection failures and attempt reconnection."""
        self._consecutive_failures += 1
        print(f"[Network] ❌ Connection failure #{self._consecutive_failures}")
        
        if self.conn:
            try:
                self.conn.close()
            except:
                pass
            self.conn = None
        
        # Attempt to reconnect
        try:
            if self.wait_for_connection():
                return True
        except Exception as e:
            print(f"[Network] 💥 Reconnection failed: {e}")
        
        return False
    
    def get_connection_stats(self):
        """Get connection health statistics."""
        uptime = 0
        last_comm = 0
        
        if self._connection_start_time:
            uptime = time.time() - self._connection_start_time
            
        if self._last_successful_communication:
            last_comm = time.time() - self._last_successful_communication
        
        return {
            'uptime': uptime,
            'last_communication': last_comm,
            'consecutive_failures': self._consecutive_failures
        }
    
    def close(self):
        """Clean up network resources."""
        print("[Network] 🧹 Cleaning up network resources...")
        if self.conn:
            self.conn.close()
        if self.listener:
            self.listener.close()


def setup_shutdown_handler():
    """Set up graceful shutdown handling."""
    import signal
    
    def signal_handler(sig, frame):
        global shutdown_requested, shutdown_start_time
        
        if shutdown_requested:
            # Second Ctrl+C - force quit
            print("\n[Network] 💥 FORCE QUIT! Exiting immediately...")
            import os
            os._exit(1)
        else:
            # First Ctrl+C - graceful shutdown
            print("\n[Network] 🛑 Shutdown requested... Cleaning up...")
            print("[Network] ⚡ Press Ctrl+C again to force quit immediately")
            shutdown_requested = True
            shutdown_start_time = time.time()
    
    signal.signal(signal.SIGINT, signal_handler) 