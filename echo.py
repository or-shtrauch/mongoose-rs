import socket

HOST = "0.0.0.0"  # Listen on all available network interfaces
PORT = 1234      # Choose a port number

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server_socket:
    server_socket.bind((HOST, PORT))
    server_socket.listen()

    print(f"Echo server listening on {HOST}:{PORT}")

    while True:
        conn, addr = server_socket.accept()
        print(f"Connected by {addr}")

        with conn:
            while True:
                data = conn.recv(1024)  # Receive up to 1024 bytes
                if not data:
                    break  # Exit loop if client closes connection
                print(f"got {data}")
                conn.sendall(data)  # Send the received data back

        print(f"Connection closed by {addr}")
