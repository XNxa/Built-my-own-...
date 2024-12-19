#!/bin/bash

# Define the command for your runtime
RUNTIME="./ccrun"

# Helper function to display test results
function run_test {
    echo "Running Test $1"
    eval "$2"
    if [ $? -eq 0 ]; then
        echo "✅ $1 Passed"
    else
        echo "❌ $1 Failed"
        exit 1
    fi
    echo "--------------------------------"
}

# Step 1: Run an arbitrary command
run_test "Step 1: Run a command" \
    "$RUNTIME run echo 'Hello Coding Challenges!' | grep -q 'Hello Coding Challenges!'"

run_test "Step 1: Command exit code propagation" \
    "($RUNTIME run false; test \$? -eq 1)"

# Step 2: Isolate hostname
run_test "Step 2: Isolate hostname" \
    "$RUNTIME run hostname | grep -q 'container'"

# Step 3: Change root filesystem
run_test "Step 3: Isolate filesystem" \
    "$RUNTIME run sh -c 'test -f /ALPINE_FS_ROOT'"

run_test "Step 3: Filesystem isolation" \
    "$RUNTIME run sh -c 'cd ..; ls | grep -q bin'"

# Step 4: Isolate processes
run_test "Step 4: Isolate processes" \
    "test \$($RUNTIME run sh -c 'ps' | wc -l) -eq 2"

run_test "Step 4: /proc isolation" \
    "test \$(mount | grep 'proc' | wc -l) -eq 3"

# Step 5: Rootless containers
run_test "Step 5: Rootless containers" \
    "$RUNTIME run sh -c 'id -u' | grep -vq \"$(id -u)\""

run_test "Step 5: Rootless containers" \
    "test \$($RUNTIME run sh -c 'sleep 1' | ps -el | grep \"4.*\$(id -u).*sleep\" | wc -l) -eq 1"

# # Step 6: Limit resources
# run_test "Step 6: Memory limit enforcement" \
#     "($RUNTIME run /bin/busybox sh -c 'ulimit -m | grep -q 512')"

# run_test "Step 6: CPU limit enforcement" \
#     "($RUNTIME run /bin/busybox sh -c 'cat /sys/fs/cgroup/cpu/cpu.shares | grep -q 512')"

# # Step 7: Pull image from Docker Hub
# run_test "Step 7: Pull image layers" \
#     "($RUNTIME pull alpine && test -d images/alpine)"

# # Step 8: Run pulled image
# run_test "Step 8: Run pulled image" \
#     "($RUNTIME run alpine /bin/busybox sh -c 'test -f /bin/sh')"

# # Final output
# echo "🎉 All tests passed successfully!"
