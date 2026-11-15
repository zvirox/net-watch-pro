// SPDX-License-Identifier: GPL-2.0
// eBPF spike loader — tries different attach type values for cgroup_skb
//
// Build:   gcc -o loader loader.c -lbpf
// Run:     sudo ./loader cgroup_skb.o /run/bpftune/cgroupv2/
//          (keep running to maintain attachment)

#define _GNU_SOURCE
#include <bpf/libbpf.h>
#include <linux/bpf.h>
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/resource.h>
#include <sys/syscall.h>
#include <signal.h>
#include <string.h>

#define BPF_PROG_ATTACH 5

static volatile sig_atomic_t running = 1;

void handle_signal(int sig) {
    running = 0;
}

int main(int argc, char **argv) {
    struct bpf_object *obj;
    struct bpf_program *prog;
    int cgroup_fd, prog_fd, err;
    const char *obj_file = argc > 1 ? argv[1] : "cgroup_skb.o";
    const char *cgroup_path = argc > 2 ? argv[2] : "/sys/fs/cgroup/";
    int attach_type = argc > 3 ? atoi(argv[3]) : 10;

    // Increase RLIMIT_MEMLOCK for eBPF
    struct rlimit rlim = {RLIM_INFINITY, RLIM_INFINITY};
    setrlimit(RLIMIT_MEMLOCK, &rlim);

    // Open cgroup v2 directory
    cgroup_fd = open(cgroup_path, O_RDONLY | O_DIRECTORY);
    if (cgroup_fd < 0) {
        perror("open cgroup");
        fprintf(stderr, "Tried to open: %s\n", cgroup_path);
        return 1;
    }
    printf("Opened cgroup fd: %d (path: %s)\n", cgroup_fd, cgroup_path);

    // Load object file
    obj = bpf_object__open(obj_file);
    if (!obj) {
        fprintf(stderr, "Failed to open object file: %s\n", obj_file);
        return 1;
    }

    // Load object into kernel
    if (bpf_object__load(obj)) {
        fprintf(stderr, "Failed to load object file\n");
        return 1;
    }
    printf("Object loaded successfully\n");

    // Attach ingress program
    prog = bpf_object__find_program_by_name(obj, "cgroup_skb_ingress");
    if (!prog) {
        fprintf(stderr, "Failed to find ingress program\n");
        return 1;
    }
    prog_fd = bpf_program__fd(prog);
    if (prog_fd < 0) {
        fprintf(stderr, "Failed to get ingress program fd\n");
        return 1;
    }
    
    err = syscall(__NR_bpf, BPF_PROG_ATTACH, prog_fd, cgroup_fd,
                  attach_type, 0);
    if (err < 0) {
        fprintf(stderr, "Failed to attach ingress (type=%d): %s\n",
                attach_type, strerror(-err));
    } else {
        printf("Ingress program attached (fd=%d, cgroup_fd=%d, type=%d)\n",
               prog_fd, cgroup_fd, attach_type);
    }

    // Attach egress program
    prog = bpf_object__find_program_by_name(obj, "cgroup_skb_egress");
    if (!prog) {
        fprintf(stderr, "Failed to find egress program\n");
        return 1;
    }
    prog_fd = bpf_program__fd(prog);
    if (prog_fd < 0) {
        fprintf(stderr, "Failed to get egress program fd\n");
        return 1;
    }
    
    err = syscall(__NR_bpf, BPF_PROG_ATTACH, prog_fd, cgroup_fd,
                  attach_type, 0);
    if (err < 0) {
        fprintf(stderr, "Failed to attach egress (type=%d): %s\n",
                attach_type, strerror(-err));
    } else {
        printf("Egress program attached (fd=%d, cgroup_fd=%d, type=%d)\n",
               prog_fd, cgroup_fd, attach_type);
    }

    printf("Programs loaded and attached. Press Ctrl+C to stop.\n");

    signal(SIGINT, handle_signal);
    signal(SIGTERM, handle_signal);

    while (running) {
        sleep(1);
    }

    printf("Cleaning up...\n");
    close(cgroup_fd);
    bpf_object__close(obj);
    return 0;
}
