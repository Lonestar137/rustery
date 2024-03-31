
import socket


def main():
    try:
        s = socket.socket(socket.AF_PACKET, socket.SOCK_RAW)
        print("Raw socket successfully created.")
    except socket.error as e:
        print(f"Error creating raw socket: {e}")


if __name__ == "__main__":
    main()
