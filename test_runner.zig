const std = @import("std");
const builtin = @import("builtin");

/// The test runner that is used to run tests.
pub fn main() !void {
    var last_test_namespace: []const u8 = "";
    var successes: usize = 0;
    var failures: usize = 0;

    const stdout = std.io.getStdOut().writer();
    for (builtin.test_functions) |t| {
        var test_name: []const u8 = undefined;
        var test_namespace: []const u8 = undefined;

        if (std.mem.indexOf(u8, t.name, ".test.")) |index| {
            test_name = t.name[index + 6 ..];
            test_namespace = t.name[0..index];
        } else {
            continue;
        }

        if (!std.mem.eql(u8, test_namespace, last_test_namespace)) {
            stdout.print("\n{s}:\n", .{test_namespace}) catch ioError();
            last_test_namespace = test_namespace;
        }

        const start = std.time.Instant.now() catch unreachable;
        const result = t.func();
        const stop = std.time.Instant.now() catch unreachable;
        const duration_ns = stop.since(start);

        if (result) {
            stdout.print(
                "\x1B[0;90m{s: <50}\x1B[0m \x1B[0;32mOK\x1B[0m ({})\n",
                .{ test_name, std.fmt.fmtDuration(duration_ns) },
            ) catch ioError();
            successes += 1;
        } else |_| {
            stdout.print(
                "\n\x1B[0;31m{s: <50}\x1B[0m \x1B[0;31mFAILED\x1B[0m\n\n",
                .{test_name},
            ) catch ioError();
            failures += 1;
        }
    }

    stdout.print("\nTest suit completed: \x1B[1;37m{}/{}\x1B[0m", .{successes, failures + successes}) catch ioError();
    if (failures != 0) {
        stdout.print(" (\x1B[0;31m{} test{s} failed\x1B[0m)", .{failures, if (failures == 1) "" else "s"}) catch ioError();
    }
    stdout.print("\n", .{}) catch ioError();
}

fn ioError() void {
    std.debug.panic("an I/O error occurred", .{});
}
