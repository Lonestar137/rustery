import os
import sys
import mmap
import fcntl


def test_ipc_lock_capability():
    try:
        # Allocate a shared memory segment
        size = 4096
        shm_fd = os.memfd_create("test_shm", 0)
        os.ftruncate(shm_fd, size)

        # Map the shared memory segment into the process's address space
        shm = mmap.mmap(shm_fd, size)

        # Lock the shared memory segment using fcntl
        fcntl.flock(shm_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)

        print("IPC lock capability test successful")

        # Unlock and clean up
        fcntl.flock(shm_fd, fcntl.LOCK_UN)
        shm.close()
        os.close(shm_fd)

    except Exception as e:
        print(f"IPC lock capability test failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    test_ipc_lock_capability()
